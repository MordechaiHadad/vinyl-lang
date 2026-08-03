use std::collections::HashSet;

use cranelift_codegen::ir::{self, InstBuilder, StackSlotData, StackSlotKind};
use cranelift_frontend::FunctionBuilder;

use vinyl_typecheck::hir::Type;

use super::state::{CodegenCtx, VarSlot};
use super::types::ir_type_from_primitive;
use crate::CraneliftError;

pub enum VarMode {
    Value,
    Variable,
    StackSlot,
}

pub fn var_mode(name: &str, mutable: bool, ref_vars: &HashSet<String>) -> VarMode {
    if ref_vars.contains(name) {
        VarMode::StackSlot
    } else if mutable {
        VarMode::Variable
    } else {
        VarMode::Value
    }
}

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
