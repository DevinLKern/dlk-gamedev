use std::fmt::Display;

mod obj_tokenizer;
pub(crate) use obj_tokenizer::*;

mod mtl_tokenizer;
pub(crate) use mtl_tokenizer::*;

mod obj;
pub use obj::*;

mod mtl;
pub use mtl::*;

#[allow(unused)]
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "Io({e})"),
            Error::Parse(e) => write!(f, "Parse({e})"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[allow(unused)]
pub type Result<T> = std::result::Result<T, Error>;
