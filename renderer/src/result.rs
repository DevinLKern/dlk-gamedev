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
    NotAdded,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VulkanError(e) => write!(f, "{}", e),
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

pub type Result<T> = std::result::Result<T, Error>;
