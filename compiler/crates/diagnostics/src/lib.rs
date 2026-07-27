use std::fmt;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    pub namespace: &'static str,
    pub type_name: &'static str,
    pub variant: &'static str,
}

impl fmt::Display for Code {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}::{}::{}",
            self.namespace, self.type_name, self.variant
        )
    }
}

impl Code {
    pub const fn with_detail(self, detail: &'static str) -> DetailedCode {
        DetailedCode { base: self, detail }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailedCode {
    pub base: Code,
    pub detail: &'static str,
}

impl fmt::Display for DetailedCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}::{}", self.base, self.detail)
    }
}

#[macro_export]
macro_rules! diagnostic_codes {
    ($namespace:literal, $visibility:vis enum $name:ident { $($variant:ident),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $visibility enum $name {
            $($variant),*
        }

        impl $name {
            pub const fn code(self) -> $crate::Code {
                $crate::Code {
                    namespace: $namespace,
                    type_name: stringify!($name),
                    variant: match self {
                        $(Self::$variant => stringify!($variant)),*
                    },
                }
            }
        }

    };
}

#[derive(Debug, Error, Diagnostic)]
pub enum CompilerDiagnostic {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] vinyl_parser::error::ParserDiagnostic),

    // #[error(transparent)]
    // #[diagnostic(transparent)]
    // Type(#[from] vinyl_typecheck::TypecheckerError),
    //
    // #[error(transparent)]
    // #[diagnostic(transparent)]
    // Resolve(#[from] vinyl_resolver::ResolverError),
}
