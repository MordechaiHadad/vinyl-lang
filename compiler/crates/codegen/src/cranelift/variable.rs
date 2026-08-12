use std::collections::HashSet;

use cranelift_codegen::ir::{self, InstBuilder, StackSlotData, StackSlotKind};
use cranelift_frontend::FunctionBuilder;

use vinyl_typecheck::hir::Type;

use super::state::{CodegenCtx, VarSlot};
use super::types::ir_type_from_primitive;
use crate::CraneliftError;
use crate::locals::FunctionBackend;

/// Storage mode selected for a local variable.
pub enum VarMode {
    Value,
    Variable,
    StackSlot,
}

/// Selects local storage from mutability and address-taking information.
pub fn var_mode(name: &str, mutable: bool, ref_vars: &HashSet<String>) -> VarMode {
    if ref_vars.contains(name) {
        VarMode::StackSlot
    } else if mutable {
        VarMode::Variable
    } else {
        VarMode::Value
    }
}

/// Creates backend storage and initializes it with a value.
pub fn build_var_info(
    builder: &mut FunctionBuilder,
    _vtype: &Type,
    clif_type: ir::Type,
    initial_val: ir::Value,
    mode: VarMode,
    pointer_type: ir::Type,
) -> (VarSlot, ir::Type) {
    match mode {
        VarMode::Value => (VarSlot::Value(initial_val), clif_type),
        VarMode::Variable => {
            let var = builder.declare_var(clif_type);
            builder.def_var(var, initial_val);
            (VarSlot::Variable(var), clif_type)
        }
        VarMode::StackSlot => {
            let ptr_size = pointer_type.bytes();
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                ptr_size.max(clif_type.bytes()),
                0,
            ));
            let addr = builder.ins().stack_addr(pointer_type, slot, 0);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            builder.ins().store(mflags, initial_val, addr, 0);
            (VarSlot::StackSlot(slot, clif_type), clif_type)
        }
    }
}

impl<'a> CodegenCtx<'a> {
    pub fn read_var(&mut self, name: &str) -> Result<ir::Value, CraneliftError> {
        let val = self.read_var_raw(name)?;
        let info = self
            .func
            .vars
            .get(name)
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`")))?;
        if let Type::Ref(inner) = &info.vinyl_type {
            let inner_ty = ir_type_from_primitive(inner.as_ref(), self.module.pointer_type);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            Ok(self.func.builder.ins().load(inner_ty, mflags, val, 0))
        } else {
            Ok(val)
        }
    }

    pub fn read_var_raw(&mut self, name: &str) -> Result<ir::Value, CraneliftError> {
        let info = self
            .func
            .vars
            .get(name)
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`")))?;
        match info.slot {
            VarSlot::Value(v) => Ok(v),
            VarSlot::Variable(v) => Ok(self.func.builder.use_var(v)),
            VarSlot::StackSlot(slot, ty) => {
                let addr = self
                    .func
                    .builder
                    .ins()
                    .stack_addr(self.module.pointer_type, slot, 0);
                let ptr_size = self.module.pointer_type.bytes();
                let is_large = crate::layout::is_aggregate(&info.vinyl_type)
                    && crate::layout::size_of(&info.vinyl_type, self.module.types, ptr_size) > 8;
                if is_large {
                    return Ok(addr);
                }
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                let val = self.func.builder.ins().load(ty, mflags, addr, 0);
                Ok(val)
            }
        }
    }

    pub fn write_var(&mut self, name: &str, val: ir::Value) -> Result<(), CraneliftError> {
        let info = self
            .func
            .vars
            .get_mut(name)
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`")))?;
        match info.slot {
            VarSlot::Value(_) => Err(CraneliftError::Msg(format!(
                "cannot write to immutable variable `{name}`"
            ))),
            VarSlot::Variable(v) => {
                self.func.builder.def_var(v, val);
                Ok(())
            }
            VarSlot::StackSlot(slot, _ty) => {
                let addr = self
                    .func
                    .builder
                    .ins()
                    .stack_addr(self.module.pointer_type, slot, 0);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                self.func.builder.ins().store(mflags, val, addr, 0);
                Ok(())
            }
        }
    }
}

impl<'a> FunctionBackend for CodegenCtx<'a> {
    type Value = ir::Value;
    type Storage = VarSlot;
    type Error = CraneliftError;

    fn declare_local(
        &mut self,
        name: &str,
        type_: &Type,
        value: ir::Value,
        mutable: bool,
        address_taken: bool,
    ) -> Result<VarSlot, CraneliftError> {
        if crate::layout::is_aggregate(type_)
            && self
                .func
                .target
                .aggregate_abi(type_, self.module.types)
                .needs_memory_storage()
        {
            let pointer_type = self.module.pointer_type;
            let slot = self.func.builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    crate::layout::aggregate_slot_size(
                        type_,
                        self.module.types,
                        self.func.target.pointer_size,
                    ),
                    0,
                ),
            );
            let destination = self.func.builder.ins().stack_addr(pointer_type, slot, 0);
            self.emit_memcpy(
                destination,
                value,
                crate::layout::aggregate_copy_size(
                    type_,
                    self.module.types,
                    self.func.target.pointer_size,
                ),
            )?;
            return Ok(VarSlot::StackSlot(slot, pointer_type));
        }
        let mode = if address_taken {
            VarMode::StackSlot
        } else {
            var_mode(name, mutable, self.func.ref_vars)
        };
        let clif_type = ir_type_from_primitive(type_, self.module.pointer_type);
        Ok(build_var_info(
            self.func.builder,
            type_,
            clif_type,
            value,
            mode,
            self.module.pointer_type,
        )
        .0)
    }

    fn load_local(&mut self, storage: &VarSlot) -> Result<ir::Value, CraneliftError> {
        match *storage {
            VarSlot::Value(value) => Ok(value),
            VarSlot::Variable(variable) => Ok(self.func.builder.use_var(variable)),
            VarSlot::StackSlot(slot, type_) => {
                let address = self
                    .func
                    .builder
                    .ins()
                    .stack_addr(self.module.pointer_type, slot, 0);
                let flags = cranelift_codegen::ir::MachMemFlags::trusted();
                Ok(self.func.builder.ins().load(type_, flags, address, 0))
            }
        }
    }

    fn store_local(&mut self, storage: &VarSlot, value: ir::Value) -> Result<(), CraneliftError> {
        match *storage {
            VarSlot::Value(_) => Err(CraneliftError::Msg(
                "cannot write to immutable variable".to_string(),
            )),
            VarSlot::Variable(variable) => {
                self.func.builder.def_var(variable, value);
                Ok(())
            }
            VarSlot::StackSlot(slot, _) => {
                let address = self
                    .func
                    .builder
                    .ins()
                    .stack_addr(self.module.pointer_type, slot, 0);
                let flags = cranelift_codegen::ir::MachMemFlags::trusted();
                self.func.builder.ins().store(flags, value, address, 0);
                Ok(())
            }
        }
    }

    fn invalid_local(message: &'static str) -> CraneliftError {
        CraneliftError::Msg(message.to_string())
    }
}
