use crate::{Error, Result};

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(crate) enum MtlToken {
    //
}

#[allow(unused)]
pub(crate) struct MtlTokenizer {
    line: String,
    reader: BufReader<File>,
}

impl MtlTokenizer {
    //
}
