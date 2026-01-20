#[derive(Debug)]
pub enum Error {
    IncorrectProgramUsage,
    IoError(std::io::Error),
    WinitExternalError(winit::error::ExternalError),
    WinitEventLoopError(winit::error::EventLoopError),
    WinitHandleError(winit::raw_window_handle::HandleError),
    VulkanError(vulkan::result::Error),
    ImageError(image::ImageError),
    RendererError(renderer::result::Error),
    NotImplemented,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncorrectProgramUsage => {
                write!(f, "Incorrect usage. Expects: [working_directory] [image]")
            }
            Self::IoError(e) => write!(f, "IoError: {}", e),
            Self::WinitExternalError(e) => write!(f, "ExternalError({})", e),
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

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Error::ImageError(value)
    }
}

impl From<renderer::result::Error> for Error {
    fn from(value: renderer::result::Error) -> Self {
        match value {
            renderer::result::Error::VulkanError(e) => Error::VulkanError(e),
            e => Error::RendererError(e),
        }
    }
}

impl From<winit::error::ExternalError> for Error {
    fn from(value: winit::error::ExternalError) -> Self {
        Error::WinitExternalError(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
