use std::collections::HashMap;

use vinyl_typecheck::hir::{HirItemKind, Type};

/// Target properties required by the Vinyl ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetData {
    /// Size of a native pointer in bytes.
    pub pointer_size: u32,
}

impl TargetData {
    /// Creates target data from a native pointer size.
    pub fn new(pointer_size: u32) -> Self {
        Self { pointer_size }
    }

    /// Returns how an aggregate is passed across a function boundary.
    pub fn aggregate_abi(self, type_: &Type, types: &HashMap<String, HirItemKind>) -> AggregateAbi {
        match crate::layout::aggregate_register_count(type_, types, self.pointer_size) {
            0 => AggregateAbi::Indirect,
            registers => AggregateAbi::Registers { count: registers },
        }
    }
}

/// ABI representation of an aggregate value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateAbi {
    /// The value is passed through a hidden or explicit memory pointer.
    Indirect,
    /// The value is passed in one or more integer registers.
    Registers { count: usize },
}

impl AggregateAbi {
    /// Returns the number of register chunks, or zero for indirect values.
    pub const fn register_count(self) -> usize {
        match self {
            Self::Indirect => 0,
            Self::Registers { count } => count,
        }
    }

    /// Returns whether the value is passed indirectly.
    pub const fn is_indirect(self) -> bool {
        matches!(self, Self::Indirect)
    }
}
