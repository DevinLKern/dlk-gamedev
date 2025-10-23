#[derive(Debug)]
pub enum Error {
    None,
    RendererError(renderer::result::Error),
    FileIoError(std::io::Error),
    NotImplemented,
    MalformedFile,
    InvalidState,
    Other,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match *self {
            _ => "NotImplemented",
        };
        write!(f, "{}", str)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
