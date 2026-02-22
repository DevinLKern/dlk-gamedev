pub mod module;
pub mod result;

pub use module::{Module, StructMemberInfo, TypeInfo, UniformInfo};

include!(concat!(env!("OUT_DIR"), "/opcode.rs"));
include!(concat!(env!("OUT_DIR"), "/opkind.rs"));
include!(concat!(env!("OUT_DIR"), "/magic_numbers.rs"));

use result::{Error, Result};
