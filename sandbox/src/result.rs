#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    WinitEventLoopError(winit::error::EventLoopError),
    WinitHandleError(winit::raw_window_handle::HandleError),
    VulkanError(vulkan::result::Error),
    NotImplemented,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IoError: {}", e),
            Self::WinitEventLoopError(e) => write!(f, "EventLoopError({})", e),
            Self::WinitHandleError(e) => write!(f, "HandleError({})", e),
            Self::VulkanError(e) => write!(f, "VulkanError({})", e),
            _ => write!(f, "std::fmt::Display not implemented!"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError(value)
    }
}

impl From<winit::error::EventLoopError> for Error {
    fn from(value: winit::error::EventLoopError) -> Self {
        Error::WinitEventLoopError(value)
    }
}

impl From<winit::raw_window_handle::HandleError> for Error {
    fn from(value: winit::raw_window_handle::HandleError) -> Self {
        Error::WinitHandleError(value)
    }
}

impl From<vulkan::result::Error> for Error {
    fn from(value: vulkan::result::Error) -> Self {
        Error::VulkanError(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
