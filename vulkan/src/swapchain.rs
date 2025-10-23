use crate::device::Device;
use crate::result::{Error, Result};
use crate::trace_error;
use ash::vk;
use std::rc::Rc;

pub struct Swapchain {
    device: Rc<Device>,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    extent: vk::Extent2D,
    format: vk::Format,
    images: Box<[vk::Image]>,
    image_views: Box<[vk::ImageView]>,
}

impl Swapchain {
    pub fn new(device: Rc<Device>, window: &winit::window::Window) -> Result<Swapchain> {
        let surface = unsafe { device.create_surface(window) }?;

        let surface_format = unsafe { device.get_physical_device_surface_formats(surface) }
            .inspect_err(|e| trace_error!(e))?
            .into_iter()
            .next()
            .ok_or(Error::NoSurfaceFomratsSupported)?;

        let (min_image_count, max_image_count, image_extent) = {
            let capabilities = unsafe { device.get_physical_device_surface_capabilities(surface) }
                .inspect_err(|e| trace_error!(e))?;

            let extent = if capabilities.current_extent.width == u32::MAX {
                ash::vk::Extent2D {
                    width: window.inner_size().width,
                    height: window.inner_size().height,
                }
            } else {
                capabilities.current_extent
            };

            if capabilities.min_image_count > capabilities.max_image_count {
                (
                    capabilities.min_image_count,
                    capabilities.min_image_count,
                    extent,
                )
            } else {
                (
                    capabilities.min_image_count,
                    capabilities.max_image_count,
                    extent,
                )
            }
        };

        let (present_mode, desired_image_count) = {
            let modes = unsafe { device.get_physical_device_surface_present_modes(surface) }
                .inspect_err(|e| trace_error!(e))?;

            if modes.contains(&ash::vk::PresentModeKHR::MAILBOX) {
                (ash::vk::PresentModeKHR::MAILBOX, 3)
            } else {
                (ash::vk::PresentModeKHR::FIFO, 2)
            }
        };

        let swapchain = {
            let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR {
                surface: surface,
                min_image_count: desired_image_count.clamp(min_image_count, max_image_count),
                image_format: surface_format.format,
                image_color_space: surface_format.color_space,
                image_extent,
                image_usage: ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
                present_mode,
                composite_alpha: ash::vk::CompositeAlphaFlagsKHR::OPAQUE,
                pre_transform: ash::vk::SurfaceTransformFlagsKHR::IDENTITY,
                clipped: ash::vk::FALSE,
                image_array_layers: 1,
                ..Default::default()
            };

            unsafe { device.create_swapchain(&swapchain_create_info) }
                .inspect_err(|e| trace_error!(e))?
        };

        let swapchain_images = unsafe { device.get_swapchain_images(swapchain) }
            .inspect_err(|e| trace_error!(e))?
            .into_boxed_slice();

        let mut views = Vec::with_capacity(swapchain_images.len());
        for image in swapchain_images.iter() {
            let image_view_create_info = ash::vk::ImageViewCreateInfo {
                image: *image,
                view_type: ash::vk::ImageViewType::TYPE_2D,
                format: surface_format.format,
                components: ash::vk::ComponentMapping {
                    r: ash::vk::ComponentSwizzle::IDENTITY,
                    g: ash::vk::ComponentSwizzle::IDENTITY,
                    b: ash::vk::ComponentSwizzle::IDENTITY,
                    a: ash::vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: ash::vk::ImageSubresourceRange {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };

            let view = unsafe { device.create_image_view(&image_view_create_info) }
                .inspect_err(|e| trace_error!(e))?;
            views.push(view);
        }
        let views = views.into_boxed_slice();

        Ok(Swapchain {
            device,
            surface,
            swapchain,
            format: surface_format.format,
            extent: image_extent,
            images: swapchain_images,
            image_views: views,
        })
    }

    #[inline]
    pub fn get_extent(&self) -> &vk::Extent2D {
        &self.extent
    }

    #[inline]
    pub fn get_image_count(&self) -> usize {
        self.images.len()
    }

    #[inline]
    pub fn get_image_view(&self, index: usize) -> Option<&vk::ImageView> {
        self.image_views.get(index)
    }

    #[inline]
    pub fn get_image(&self, index: usize) -> Option<&vk::Image> {
        self.images.get(index)
    }

    #[inline]
    pub fn get_surface(&self) -> vk::SurfaceKHR {
        self.surface
    }

    #[inline]
    pub fn get_format(&self) -> vk::Format {
        self.format
    }

    pub unsafe fn acquire_next_image(
        &self,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> ash::prelude::VkResult<(u32, bool)> {
        unsafe {
            self.device
                .acquire_next_image(self.swapchain, semaphore, fence)
        }
    }

    #[inline]
    pub unsafe fn get_swapchain_ptr(&self) -> *const vk::SwapchainKHR {
        &self.swapchain
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for image_view in self.image_views.iter().rev() {
                self.device.destroy_image_view(*image_view);
            }
            self.device.destroy_swapchain(self.swapchain);

            self.device.destroy_surface(self.surface);
        }
    }
}
