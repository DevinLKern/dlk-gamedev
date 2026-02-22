#[macro_export]
macro_rules! trace_error {
    ($e:expr) => {
        println!(
            "[ERROR] LINE: {}, FILE \'{}\', ERROR: \'{}\'",
            line!(),
            file!(),
            $e
        )
    };
}

#[derive(Debug)]
pub enum Error {
    VulkanError(vulkan::result::Error),
    SpirvError(spirv::result::Error),
    ExpectedUniformBufferView,
    NotAdded,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VulkanError(e) => write!(f, "VulkanError({})", e),
            Self::SpirvError(e) => write!(f, "SpirvError({})", e),
            _ => write!(f, "Error type not added yet"),
        }
    }
}

impl From<ash::vk::Result> for Error {
    #[inline]
    fn from(value: ash::vk::Result) -> Self {
        Self::VulkanError(value.into())
    }
}

impl From<vulkan::result::Error> for Error {
    fn from(value: vulkan::result::Error) -> Self {
        Self::VulkanError(value)
    }
}

impl From<spirv::result::Error> for Error {
    fn from(value: spirv::result::Error) -> Self {
        Self::SpirvError(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
