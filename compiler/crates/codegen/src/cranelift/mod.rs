use std::fmt;

#[derive(Debug)]
pub enum CraneliftError {
    Msg(String),
}

impl fmt::Display for CraneliftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CraneliftError::Msg(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CraneliftError {}

pub mod backend;
pub mod expression;
pub mod prescan;
pub mod state;
pub mod statement;
pub mod types;
pub mod variable;

pub use self::backend::CraneliftBackend;
