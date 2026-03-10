#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    WinitExternalError(winit::error::ExternalError),
    WinitEventLoopError(winit::error::EventLoopError),
    WinitHandleError(winit::raw_window_handle::HandleError),
    VulkanError(vulkan::result::Error),
    ImageError(image::ImageError),
    WavefrontError(wavefront_obj::ParseError),
    RendererError(renderer::Error),
    WindowIdInvalid,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IoError: {}", e),
            Self::WinitExternalError(e) => write!(f, "ExternalError({})", e),
            Self::WinitEventLoopError(e) => write!(f, "EventLoopError({})", e),
            Self::WinitHandleError(e) => write!(f, "HandleError({})", e),
            Self::VulkanError(e) => write!(f, "VulkanError({})", e),
            Self::ImageError(e) => write!(f, "ImageError({})", e),
            Self::RendererError(e) => write!(f, "RendererError({})", e),
            Self::WindowIdInvalid => write!(f, "WindowIdInvalid"),
            Self::WavefrontError(e) => write!(f, "WavefrontError({})", e),
            // _ => write!(f, "std::fmt::Display not implemented!"),
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

impl From<wavefront_obj::ParseError> for Error {
    fn from(value: wavefront_obj::ParseError) -> Self {
        Error::WavefrontError(value)
    }
}

impl From<renderer::Error> for Error {
    fn from(value: renderer::Error) -> Self {
        match value {
            renderer::Error::VulkanError(e) => Error::VulkanError(e),
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
