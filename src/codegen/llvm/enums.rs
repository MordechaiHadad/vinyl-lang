use inkwell::values::{GlobalValue, PointerValue};

pub enum AnyVariableEnum<'ctx> {
    PointerValue(PointerValue<'ctx>),
    GlobalValue(GlobalValue<'ctx>),
}
