use std::collections::HashMap;
use std::mem;

use cranelift_codegen::ir::{InstBuilder, StackSlotData, StackSlotKind, types};
use cranelift_codegen::{isa, settings, Context};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use target_lexicon::Triple;

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{HirItem, HirItemKind, HirParam, Type};

use super::state::{CodegenCtx, FuncEnv, ModuleEnv, VarInfo, VarSlot};
use super::prescan::prescan_function_body;
use super::types::{hir_sig_to_clif, param_type_to_clif};
use super::variable::{build_var_info, var_mode};
use super::CraneliftError;

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
        let flags = settings::Flags::new(settings::builder());
        let isa = isa_builder
            .finish(flags)
            .map_err(|e| CraneliftError::Msg(format!("isa finish: {e}")))?;
        let jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
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

        for item in items {
            let name = match &item.kind {
                HirItemKind::Struct(s) => Some(s.name.clone()),
                HirItemKind::TupleStruct(t) => Some(t.name.clone()),
                HirItemKind::Enum(e) => Some(e.name.clone()),
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

                // Skip sret param slot if present
                // todo: baseline sret, multi-register return replaces this
                let needs_sret = match &func.return_type {
                    Type::Primitive(Primitive::Unit) => false,
                    other => crate::layout::size_of(other, &self.types, pointer_type.bytes()) > 8,
                };
                if needs_sret {
                    let _sret_ptr = builder.append_block_param(entry, pointer_type);
                }

                for param in params.iter() {
                    let ptr_size = pointer_type.bytes();
                    let param_size =
                        crate::layout::size_of(&param.type_, &self.types, ptr_size);
                    if param_size > 8 {
                        // todo: baseline by-ref, multi-register decomposition replaces this
                        let ptr = builder.append_block_param(entry, pointer_type);
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            param_size,
                            0,
                        ));
                        let dest = builder.ins().stack_addr(pointer_type, slot, 0);
                        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                        // Inline byte-by-byte copy from ptr to slot
                        for byte_offset in 0..param_size {
                            let off =
                                builder.ins().iconst(pointer_type, byte_offset as i64);
                            let src = builder.ins().iadd(ptr, off);
                            let b = builder.ins().load(types::I8, mflags, src, 0);
                            let dst = builder.ins().iadd(dest, off);
                            builder.ins().store(mflags, b, dst, 0);
                        }
                        vars.insert(
                            param.name.clone(),
                            VarInfo {
                                slot: VarSlot::StackSlot(slot, pointer_type),
                                vinyl_type: param.type_.clone(),
                            },
                        );
                    } else {
                        let ty = param_type_to_clif(&param.type_, pointer_type);
                        let val = builder.append_block_param(entry, ty);
                        let mode = var_mode(&param.name, param.mutable, &ref_vars);
                        let (slot, _) = build_var_info(
                            &mut builder,
                            &param.type_,
                            ty,
                            val,
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
                        types: &self.types,
                        pointer_type,
                    },
                    func: FuncEnv {
                        builder: &mut builder,
                        vars: &mut vars,
                        ref_vars: &ref_vars,
                        break_target: None,
                        continue_target: None,
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
        let main_fn: unsafe extern "C" fn() -> i64 = unsafe { mem::transmute(main_ptr) };
        let result = unsafe { main_fn() };
        Ok(result)
    }
}
