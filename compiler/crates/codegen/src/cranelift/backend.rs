use std::collections::HashMap;
use std::mem;

use cranelift_codegen::ir::{self, InstBuilder, StackSlotData, StackSlotKind, types};
use cranelift_codegen::{Context, isa, settings};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use target_lexicon::Triple;

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{HirItem, HirItemKind, HirParam, Type};

use super::prescan::prescan_function_body;
use super::state::{CodegenCtx, FuncEnv, ModuleEnv, VarInfo, VarSlot};
use super::types::{hir_sig_to_clif, param_type_to_clif};
use super::variable::{build_var_info, var_mode};
use crate::CraneliftError;
use crate::runtime::vinyl_print_value;

use tracing::debug;

pub struct CraneliftBackend {
    module: JITModule,
    ctx: Context,
    decls: Vec<(String, cranelift_module::FuncId, Vec<HirParam>, Type)>,
    types: HashMap<String, HirItemKind>,
}

impl CraneliftBackend {
    pub fn new() -> Result<Self, CraneliftError> {
        let isa_builder = isa::lookup(Triple::host())
            .map_err(|e| CraneliftError::Msg(format!("isa lookup: {e}")))?;
        use cranelift_codegen::settings::Configurable;
        let mut flag_builder = settings::builder();
        flag_builder.enable("enable_llvm_abi_extensions").unwrap();
        let flags = settings::Flags::new(flag_builder);
        let isa = isa_builder
            .finish(flags)
            .map_err(|e| CraneliftError::Msg(format!("isa finish: {e}")))?;
        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        jit_builder.symbol("vinyl_print_value", vinyl_print_value as *const u8);
        let module = JITModule::new(jit_builder);
        let ctx = module.make_context();
        Ok(CraneliftBackend {
            module,
            ctx,
            decls: Vec::new(),
            types: HashMap::new(),
        })
    }
}

impl crate::CodegenBackend for CraneliftBackend {
    type Error = CraneliftError;

    fn compile(&mut self, items: &[HirItem]) -> Result<(), Self::Error> {
        let pointer_type = self.module.isa().pointer_type();
        let mut print_sig = ir::Signature::new(self.module.isa().default_call_conv());
        print_sig.params.push(ir::AbiParam::new(pointer_type));
        print_sig.params.push(ir::AbiParam::new(pointer_type));
        print_sig.params.push(ir::AbiParam::new(types::I8));
        print_sig.params.push(ir::AbiParam::new(types::I8));
        let print_func = self
            .module
            .declare_function("vinyl_print_value", Linkage::Import, &print_sig)
            .map_err(|e| CraneliftError::Msg(format!("declare vinyl_print_value: {e}")))?;

        for item in items {
            let name = match &item.kind {
                HirItemKind::Struct(s) => Some(s.name.clone()),
                HirItemKind::TupleStruct(t) => Some(t.name.clone()),
                HirItemKind::Enum(e) => Some(e.name.clone()),
                HirItemKind::TypeAlias(a) => Some(a.name.clone()),
                _ => None,
            };
            if let Some(name) = name {
                self.types.insert(name, item.kind.clone());
            }
        }

        for item in items {
            let HirItemKind::Function(f) = &item.kind else {
                continue;
            };
            let sig = hir_sig_to_clif(f, &self.types, pointer_type);
            let func_id = self
                .module
                .declare_function(&f.name, Linkage::Export, &sig)
                .map_err(|e| CraneliftError::Msg(format!("declare {}: {e}", f.name)))?;
            self.decls.push((
                f.name.clone(),
                func_id,
                f.params.clone(),
                f.return_type.clone(),
            ));
        }

        for (name, func_id, params, _) in &self.decls.clone() {
            let func = items
                .iter()
                .find_map(|item| {
                    if let HirItemKind::Function(f) = &item.kind {
                        if &f.name == name { Some(f) } else { None }
                    } else {
                        None
                    }
                })
                .ok_or_else(|| CraneliftError::Msg(format!("function {name} not found")))?;

            self.ctx.clear();
            self.ctx.func.signature = hir_sig_to_clif(func, &self.types, pointer_type);

            {
                let mut builder_ctx = FunctionBuilderContext::new();
                let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);
                let entry = builder.create_block();
                builder.switch_to_block(entry);

                let ref_vars = prescan_function_body(&func.body);
                let mut vars = HashMap::new();

                let ptr_size = pointer_type.bytes();
                let needs_sret = crate::layout::is_aggregate(&func.return_type)
                    && crate::layout::aggregate_register_count(
                        &func.return_type,
                        &self.types,
                        ptr_size,
                    ) == 0;
                if func.name == "main"
                    && (crate::layout::size_of(&func.return_type, &self.types, ptr_size) > 8
                        || matches!(
                            func.return_type,
                            Type::Primitive(
                                Primitive::Float32 | Primitive::Float64 | Primitive::Float
                            )
                        ))
                {
                    return Err(CraneliftError::Msg(
                        "main return type must fit in a 64-bit register (JIT entry limitation)"
                            .to_string(),
                    ));
                }
                let sret_ptr = if needs_sret {
                    let param = builder.append_block_param(entry, pointer_type);
                    Some(param)
                } else {
                    None
                };

                // Append every entry block parameter before emitting any instruction.
                let mut param_values: Vec<(Vec<ir::Value>, Type)> = Vec::new();
                for param in params {
                    let values = if crate::layout::is_aggregate(&param.type_) {
                        let chunks = crate::layout::aggregate_register_count(
                            &param.type_,
                            &self.types,
                            ptr_size,
                        );
                        if chunks == 0 {
                            // >16 bytes: passed by reference
                            vec![builder.append_block_param(entry, pointer_type)]
                        } else {
                            (0..chunks)
                                .map(|_| builder.append_block_param(entry, types::I64))
                                .collect()
                        }
                    } else {
                        let ty = param_type_to_clif(&param.type_, pointer_type);
                        vec![builder.append_block_param(entry, ty)]
                    };
                    param_values.push((values, param.type_.clone()));
                }

                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                for (param, (values, param_type)) in params.iter().zip(param_values) {
                    if crate::layout::is_aggregate(&param.type_) {
                        let chunks = crate::layout::aggregate_register_count(
                            &param.type_,
                            &self.types,
                            ptr_size,
                        );
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            if chunks == 0 {
                                crate::layout::size_of(&param.type_, &self.types, ptr_size)
                            } else {
                                crate::layout::aggregate_slot_size(
                                    &param.type_,
                                    &self.types,
                                    ptr_size,
                                )
                            },
                            0,
                        ));
                        let dest = builder.ins().stack_addr(pointer_type, slot, 0);
                        if chunks == 0 {
                            let copy_size = crate::layout::aggregate_copy_size(
                                &param.type_,
                                &self.types,
                                ptr_size,
                            );
                            for byte_offset in 0..copy_size {
                                let off = builder.ins().iconst(pointer_type, byte_offset as i64);
                                let src_addr = builder.ins().iadd(values[0], off);
                                let b = builder.ins().load(types::I8, mflags, src_addr, 0);
                                let dst_addr = builder.ins().iadd(dest, off);
                                builder.ins().store(mflags, b, dst_addr, 0);
                            }
                        } else {
                            for (i, chunk) in values.iter().enumerate() {
                                let off = builder.ins().iconst(pointer_type, (i as i64) * 8);
                                let addr = builder.ins().iadd(dest, off);
                                builder.ins().store(mflags, *chunk, addr, 0);
                            }
                        }
                        vars.insert(
                            param.name.clone(),
                            VarInfo {
                                slot: VarSlot::StackSlot(slot, pointer_type),
                                vinyl_type: param_type,
                            },
                        );
                    } else {
                        let ty = param_type_to_clif(&param.type_, pointer_type);
                        let mode = var_mode(&param.name, param.mutable, &ref_vars);
                        let (slot, _) = build_var_info(
                            &mut builder,
                            &param.type_,
                            ty,
                            values[0],
                            mode,
                            pointer_type,
                        );
                        vars.insert(
                            param.name.clone(),
                            VarInfo {
                                slot,
                                vinyl_type: param.type_.clone(),
                            },
                        );
                    }
                }

                let mut ctx = CodegenCtx {
                    module: ModuleEnv {
                        module: &mut self.module,
                        decls: &self.decls,
                        print_func,
                        types: &self.types,
                        pointer_type,
                    },
                    func: FuncEnv {
                        builder: &mut builder,
                        vars: &mut vars,
                        ref_vars: &ref_vars,
                        break_target: None,
                        continue_target: None,
                        return_type: func.return_type.clone(),
                        sret_ptr,
                    },
                };

                let mut terminated = false;
                for stmt in &func.body {
                    ctx.compile_stmt(stmt, &mut terminated)?;
                }

                if !terminated {
                    ctx.func.builder.ins().return_(&[]);
                }

                ctx.func.builder.seal_all_blocks();
            }

            let ir_string = self.ctx.func.display().to_string();
            debug!("IR for {name}:\n{ir_string}");
            self.module
                .define_function(*func_id, &mut self.ctx)
                .map_err(|e| {
                    CraneliftError::Msg(format!("define {name}: {e}\nIR:\n{ir_string}"))
                })?;
            self.module.clear_context(&mut self.ctx);
        }

        self.module
            .finalize_definitions()
            .map_err(|e| CraneliftError::Msg(format!("finalize: {e}")))?;

        Ok(())
    }

    fn run(&self) -> Result<i64, Self::Error> {
        let Some((main_id, main_return)) = self
            .decls
            .iter()
            .find(|(n, _, _, _)| n == "main")
            .map(|(_, id, _, ret_type)| (id, ret_type))
        else {
            return Ok(0);
        };

        if matches!(main_return, Type::Primitive(Primitive::Unit)) {
            let main_fn: unsafe extern "C" fn() =
                unsafe { mem::transmute(self.module.get_finalized_function(*main_id)) };
            unsafe { main_fn() };
            return Ok(0);
        }

        let main_ptr = self.module.get_finalized_function(*main_id);

        if matches!(
            main_return,
            Type::Primitive(Primitive::Float64 | Primitive::Float)
        ) {
            let main_fn: unsafe extern "C" fn() -> f64 = unsafe { mem::transmute(main_ptr) };
            let result = unsafe { main_fn() };
            return Ok(result.to_bits() as i64);
        }

        let main_fn: unsafe extern "C" fn() -> i64 = unsafe { mem::transmute(main_ptr) };
        let result = unsafe { main_fn() };
        Ok(result)
    }
}
