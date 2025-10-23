#[derive(Debug)]
pub enum Error {
    InvalidFileLength(usize),
    IncorrectMagicWord(u32),
    InvalidOperandEnd((usize, usize)),
    Io(std::io::Error),
    NoAssociatedType(u32),
    InvalidType,
    LocationMissing(u32),
    NameMissing(u32),
    DecorationMissing(u32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFileLength(len) => {
                write!(
                    f,
                    "Invalid file length: expected minimum length not met (got {len} bytes)"
                )
            }
            Self::IncorrectMagicWord(word) => {
                write!(
                    f,
                    "Incorrect magic word: expected SPIR-V magic, got {word:#X}"
                )
            }
            Self::InvalidOperandEnd((start, end)) => {
                write!(
                    f,
                    "Invalid operand end: operand spans {start}..{end}, which is out of bounds"
                )
            }
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::NoAssociatedType(id) => {
                write!(f, "No associated type found for id {id}")
            }
            Self::InvalidType => write!(f, "Encountered an invalid type"),
            Self::LocationMissing(id) => write!(f, "Missing location for id {id}"),
            Self::NameMissing(id) => write!(f, "Missing name for id {id}"),
            Self::DecorationMissing(id) => write!(f, "Missing decoration for id {id}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
