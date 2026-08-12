use core::fmt;

/// A source-level type.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(Primitive),
    Named(String),
    Generic { name: String, args: Vec<Type> },
    Ref(Box<Type>),
    Array { element: Box<Type>, size: usize },
    Tuple(Vec<Type>),
    Var(usize),
}

impl Type {
    /// Returns the primitive type, if this is one.
    pub fn as_primitive(&self) -> Option<&Primitive> {
        match self {
            Type::Primitive(p) => Some(p),
            _ => None,
        }
    }

    /// Returns whether this is an integer type.
    pub fn is_int(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_int())
    }

    /// Returns whether this is an unsigned integer type.
    pub fn is_uint(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_uint())
    }

    /// Returns whether this is a signed integer type.
    pub fn is_signed(&self) -> bool {
        self.is_int()
    }

    /// Returns whether this is an unsigned integer type.
    pub fn is_unsigned(&self) -> bool {
        self.is_uint()
    }

    /// Returns whether this is a floating-point type.
    pub fn is_float(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_float())
    }

    /// Returns whether this is numeric.
    pub fn is_numeric(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_numeric())
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Primitive(p) => fmt::Display::fmt(p, f),
            Type::Named(name) => write!(f, "{name}"),
            Type::Generic { name, args } => {
                write!(f, "{name}<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    fmt::Display::fmt(arg, f)?;
                }
                write!(f, ">")
            }
            Type::Ref(inner) => write!(f, "&{inner}"),
            Type::Array { element, size } => write!(f, "[{element}; {size}]"),
            Type::Tuple(elements) => {
                write!(f, "(")?;
                for (i, t) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    fmt::Display::fmt(t, f)?;
                }
                if elements.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Type::Var(id) => write!(f, "_{id}"),
        }
    }
}

/// A built-in primitive type.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    ISize,
    UInt,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    USize,
    Float,
    Float32,
    Float64,
    Bool,
    Char,
    String,
    Unit,
}

impl Primitive {
    /// Returns whether this is a signed integer primitive.
    pub fn is_int(&self) -> bool {
        matches!(
            self,
            Primitive::Int
                | Primitive::Int8
                | Primitive::Int16
                | Primitive::Int32
                | Primitive::Int64
                | Primitive::Int128
                | Primitive::ISize
        )
    }

    /// Returns whether this is an unsigned integer primitive.
    pub fn is_uint(&self) -> bool {
        matches!(
            self,
            Primitive::UInt
                | Primitive::UInt8
                | Primitive::UInt16
                | Primitive::UInt32
                | Primitive::UInt64
                | Primitive::UInt128
                | Primitive::USize
        )
    }

    /// Returns whether this is a signed integer primitive.
    pub fn is_signed(&self) -> bool {
        self.is_int()
    }

    /// Returns whether this is an unsigned integer primitive.
    pub fn is_unsigned(&self) -> bool {
        self.is_uint()
    }

    /// Returns whether this is a floating-point primitive.
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            Primitive::Float | Primitive::Float32 | Primitive::Float64
        )
    }

    /// Returns whether this is numeric.
    pub fn is_numeric(&self) -> bool {
        self.is_int() || self.is_uint() || self.is_float()
    }
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Int => write!(f, "int"),
            Primitive::Int8 => write!(f, "int8"),
            Primitive::Int16 => write!(f, "int16"),
            Primitive::Int32 => write!(f, "int32"),
            Primitive::Int64 => write!(f, "int64"),
            Primitive::Int128 => write!(f, "int128"),
            Primitive::ISize => write!(f, "isize"),
            Primitive::UInt => write!(f, "uint"),
            Primitive::UInt8 => write!(f, "uint8"),
            Primitive::UInt16 => write!(f, "uint16"),
            Primitive::UInt32 => write!(f, "uint32"),
            Primitive::UInt64 => write!(f, "uint64"),
            Primitive::UInt128 => write!(f, "uint128"),
            Primitive::USize => write!(f, "usize"),
            Primitive::Float => write!(f, "float"),
            Primitive::Float32 => write!(f, "float32"),
            Primitive::Float64 => write!(f, "float64"),
            Primitive::Bool => write!(f, "bool"),
            Primitive::Char => write!(f, "char"),
            Primitive::String => write!(f, "string"),
            Primitive::Unit => write!(f, "unit"),
        }
    }
}
