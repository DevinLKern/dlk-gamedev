use crate::result::{Error, Result};
use crate::trace_error;
use ash::prelude::VkResult;
use ash::vk;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { *p_callback_data };
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
    };

    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

#[allow(dead_code)]
pub struct Instance {
    debug_enabled: bool,
    entry: ash::Entry,
    instance: ash::Instance,
    allocation_callbacks: Option<vk::AllocationCallbacks<'static>>,
    debug_utils: ash::ext::debug_utils::Instance,
    surface_loader: ash::khr::surface::Instance,
}

impl Instance {
    pub fn new(
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<std::rc::Rc<Instance>> {
        let entry = unsafe { ash::Entry::load() }?;

        let allocation_callbacks: Option<vk::AllocationCallbacks> = None;

        let instance = {
            let app_name = std::ffi::CString::new("My Vulkan App")?;
            let engine_name = std::ffi::CString::new("My Engine")?;

            let app_info = vk::ApplicationInfo {
                s_type: vk::StructureType::APPLICATION_INFO,
                p_next: std::ptr::null(),
                p_application_name: app_name.as_ptr(),
                application_version: vk::make_api_version(0, 1, 0, 0),
                p_engine_name: engine_name.as_ptr(),
                engine_version: vk::make_api_version(0, 1, 0, 0),
                api_version: vk::API_VERSION_1_3,
                ..Default::default()
            };
            let mut enabled_layer_names = Vec::with_capacity(4);
            let mut enabled_extension_names =
                { ash_window::enumerate_required_extensions(display_handle.as_raw())?.to_vec() };

            if debug_enabled {
                enabled_layer_names.push(c"VK_LAYER_KHRONOS_validation".as_ptr());
                enabled_extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
            }

            let available_layer_properties =
                unsafe { entry.enumerate_instance_layer_properties() }?;
            for layer_name in enabled_layer_names.iter() {
                let mut found = false;
                let enabled_layer_name = unsafe { std::ffi::CStr::from_ptr(*layer_name) };
                for layer_properties in available_layer_properties.iter() {
                    let available_layer_name =
                        unsafe { std::ffi::CStr::from_ptr(layer_properties.layer_name.as_ptr()) };
                    if enabled_layer_name == available_layer_name {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(Error::CouldNotFindLayer(enabled_layer_name.into()));
                }
            }

            let available_extension_properties =
                unsafe { entry.enumerate_instance_extension_properties(None) }?;
            for extension_name in enabled_extension_names.iter() {
                let mut found = false;
                let enabled_extension_name = unsafe { std::ffi::CStr::from_ptr(*extension_name) };
                for extension_properties in available_extension_properties.iter() {
                    let available_extension_name = unsafe {
                        std::ffi::CStr::from_ptr(extension_properties.extension_name.as_ptr())
                    };
                    if enabled_extension_name == available_extension_name {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(Error::CouldNotFindExtension(enabled_extension_name.into()));
                }
            }

            let instance_create_info = vk::InstanceCreateInfo {
                p_application_info: &app_info,
                enabled_layer_count: enabled_layer_names.len() as u32,
                pp_enabled_layer_names: enabled_layer_names.as_ptr(),
                enabled_extension_count: enabled_extension_names.len() as u32,
                pp_enabled_extension_names: enabled_extension_names.as_ptr(),
                ..Default::default()
            };

            unsafe { entry.create_instance(&instance_create_info, allocation_callbacks.as_ref()) }
                .inspect_err(|e| trace_error!(e))?
        };

        let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        Ok(std::rc::Rc::new(Instance {
            debug_enabled,
            entry,
            instance,
            allocation_callbacks,
            debug_utils,
            surface_loader,
        }))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .destroy_instance(self.allocation_callbacks.as_ref());
        }
    }
}

#[allow(dead_code)]
pub struct Device {
    instance: std::rc::Rc<Instance>,
    physical_device: vk::PhysicalDevice,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    device: ash::Device,
    swapchain_loader: ash::khr::swapchain::Device,
    queue: vk::Queue,
    queue_family_index: u32,
}

#[allow(dead_code)]
impl Device {
    pub fn new(instance: std::rc::Rc<Instance>) -> Result<Device> {
        let debug_messenger = unsafe {
            let debug_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
                s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                pfn_user_callback: Some(vulkan_debug_callback),
                p_user_data: std::ptr::null_mut(),
                ..Default::default()
            };

            instance
                .debug_utils
                .create_debug_utils_messenger(
                    &debug_messenger_create_info,
                    instance.allocation_callbacks.as_ref(),
                )
                .inspect_err(|e| {
                    trace_error!(e);
                    instance
                        .instance
                        .destroy_instance(instance.allocation_callbacks.as_ref());
                })?
        };

        let queue_priority: f32 = 1.0;

        let (queue_create_info, physical_device) = {
            let all_physical_devices = unsafe {
                instance
                    .instance
                    .enumerate_physical_devices()
                    .inspect_err(|e| {
                        trace_error!(e);
                        instance
                            .instance
                            .destroy_instance(instance.allocation_callbacks.as_ref());
                    })
            }?;

            let viable_physical_devices: Box<[(usize, vk::PhysicalDevice)]> = all_physical_devices
                .into_iter()
                .enumerate()
                .filter(|(_, pd)| {
                    let mut properties = vk::PhysicalDeviceProperties2::default();
                    unsafe {
                        instance
                            .instance
                            .get_physical_device_properties2(*pd, &mut properties);
                    }

                    if properties.properties.api_version < vk::API_VERSION_1_3 {
                        return false;
                    }

                    let queue_family_properties = unsafe {
                        let count = instance
                            .instance
                            .get_physical_device_queue_family_properties2_len(*pd);
                        let mut properties =
                            vec![vk::QueueFamilyProperties2::default(); count].into_boxed_slice();
                        instance
                            .instance
                            .get_physical_device_queue_family_properties2(*pd, properties.as_mut());
                        properties
                    };

                    if queue_family_properties
                        .iter()
                        .find(|qfp| {
                            qfp.queue_family_properties
                                .queue_flags
                                .contains(vk::QueueFlags::GRAPHICS)
                        })
                        .is_none()
                    {
                        return false;
                    }

                    true
                })
                .collect();

            if viable_physical_devices.len() == 0 {
                unsafe {
                    instance.debug_utils.destroy_debug_utils_messenger(
                        debug_messenger,
                        instance.allocation_callbacks.as_ref(),
                    );
                    instance
                        .instance
                        .destroy_instance(instance.allocation_callbacks.as_ref());
                }
                return Err(Error::NoViablePhysicalDevices);
            }

            match viable_physical_devices.into_iter().max_by_key(|(_, pd)| {
                let mut properties = vk::PhysicalDeviceProperties2::default();
                unsafe {
                    instance
                        .instance
                        .get_physical_device_properties2(*pd, &mut properties);
                }

                match properties.properties.device_type {
                    vk::PhysicalDeviceType::CPU => 1,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 3,
                    vk::PhysicalDeviceType::DISCRETE_GPU => 4,
                    _ => 0,
                }
            }) {
                Some((qfi, pd)) => (
                    vk::DeviceQueueCreateInfo {
                        queue_family_index: qfi.clone() as u32,
                        queue_count: 1,
                        p_queue_priorities: &queue_priority,
                        ..Default::default()
                    },
                    pd,
                ),
                None => {
                    unsafe {
                        instance.debug_utils.destroy_debug_utils_messenger(
                            debug_messenger,
                            instance.allocation_callbacks.as_ref(),
                        );
                        instance
                            .instance
                            .destroy_instance(instance.allocation_callbacks.as_ref());
                    }
                    return Err(Error::NoViablePhysicalDevices);
                }
            }
        };

        let device = {
            let enabled_device_extension_names = vec![ash::khr::swapchain::NAME.as_ptr()];
            let enabled_features = vk::PhysicalDeviceFeatures {
                ..Default::default()
            };
            let synchronization2_features = vk::PhysicalDeviceSynchronization2Features {
                synchronization2: vk::TRUE,
                ..Default::default()
            };
            let dynamic_rendering_features = vk::PhysicalDeviceDynamicRenderingFeatures {
                p_next: &synchronization2_features as *const _ as *mut std::ffi::c_void,
                dynamic_rendering: vk::TRUE,
                ..Default::default()
            };
            let device_create_info = vk::DeviceCreateInfo {
                p_next: &dynamic_rendering_features as *const _ as *const std::ffi::c_void,
                queue_create_info_count: 1,
                p_queue_create_infos: &queue_create_info,
                enabled_extension_count: enabled_device_extension_names.len() as u32,
                pp_enabled_extension_names: enabled_device_extension_names.as_ptr(),
                p_enabled_features: &enabled_features,
                ..Default::default()
            };

            unsafe {
                instance
                    .instance
                    .create_device(
                        physical_device,
                        &device_create_info,
                        instance.allocation_callbacks.as_ref(),
                    )
                    .inspect_err(|e| {
                        trace_error!(e);
                        instance.debug_utils.destroy_debug_utils_messenger(
                            debug_messenger,
                            instance.allocation_callbacks.as_ref(),
                        );
                        instance
                            .instance
                            .destroy_instance(instance.allocation_callbacks.as_ref());
                    })?
            }
        };

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance.instance, &device);

        let queue = {
            let get_queue_info = vk::DeviceQueueInfo2 {
                queue_family_index: queue_create_info.queue_family_index,
                queue_index: 0,
                ..Default::default()
            };
            unsafe { device.get_device_queue2(&get_queue_info) }
        };

        Ok(Device {
            instance,
            debug_messenger,
            physical_device,
            device,
            swapchain_loader,
            queue,
            queue_family_index: queue_create_info.queue_family_index,
        })
    }

    #[inline]
    unsafe fn get_alloc_callbacks(&self) -> Option<&vk::AllocationCallbacks<'_>> {
        self.instance.allocation_callbacks.as_ref()
    }

    #[inline]
    pub unsafe fn get_physical_device_format_properties(
        &self,
        format: vk::Format,
    ) -> vk::FormatProperties {
        unsafe {
            self.instance
                .instance
                .get_physical_device_format_properties(self.physical_device, format)
        }
    }

    #[inline]
    pub unsafe fn get_physical_device_surface_formats(
        &self,
        surface: vk::SurfaceKHR,
    ) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            self.instance
                .surface_loader
                .get_physical_device_surface_formats(self.physical_device, surface)
        }
    }

    #[inline]
    pub unsafe fn get_physical_device_surface_capabilities(
        &self,
        surface: vk::SurfaceKHR,
    ) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            self.instance
                .surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, surface)
        }
    }

    #[inline]
    pub unsafe fn get_physical_device_surface_present_modes(
        &self,
        surface: vk::SurfaceKHR,
    ) -> VkResult<Vec<vk::PresentModeKHR>> {
        unsafe {
            self.instance
                .surface_loader
                .get_physical_device_surface_present_modes(self.physical_device, surface)
        }
    }
    #[inline]
    pub(crate) unsafe fn get_physical_device_memory_properties(
        &self,
    ) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.instance
                .instance
                .get_physical_device_memory_properties(self.physical_device)
        }
    }

    #[inline]
    pub(crate) unsafe fn get_buffer_memory_requirements(
        &self,
        buffer: vk::Buffer,
    ) -> vk::MemoryRequirements {
        unsafe { self.device.get_buffer_memory_requirements(buffer) }
    }

    #[inline]
    pub(crate) unsafe fn get_image_memory_requirements(
        &self,
        image: vk::Image,
    ) -> vk::MemoryRequirements {
        unsafe { self.device.get_image_memory_requirements(image) }
    }

    #[inline]
    pub(crate) unsafe fn allocate_memory(
        &self,
        alloc_info: &vk::MemoryAllocateInfo,
    ) -> VkResult<vk::DeviceMemory> {
        unsafe {
            self.device
                .allocate_memory(alloc_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn free_memory(&self, memory: vk::DeviceMemory) {
        unsafe {
            self.device.free_memory(memory, self.get_alloc_callbacks());
        }
    }

    #[inline]
    pub(crate) unsafe fn create_buffer(
        &self,
        create_info: &vk::BufferCreateInfo,
    ) -> VkResult<vk::Buffer> {
        unsafe {
            self.device
                .create_buffer(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn destroy_buffer(&self, buffer: vk::Buffer) {
        unsafe {
            self.device
                .destroy_buffer(buffer, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn map_memory(
        &self,
        memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
        flags: vk::MemoryMapFlags,
    ) -> VkResult<*mut std::ffi::c_void> {
        unsafe { self.device.map_memory(memory, offset, size, flags) }
    }

    #[inline]
    pub(crate) unsafe fn unmap_memory(&self, memory: vk::DeviceMemory) {
        unsafe { self.device.unmap_memory(memory) }
    }

    #[inline]
    pub(crate) unsafe fn create_image(
        &self,
        create_info: &vk::ImageCreateInfo,
    ) -> VkResult<vk::Image> {
        unsafe {
            self.device
                .create_image(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn destroy_image(&self, image: vk::Image) {
        unsafe { self.device.destroy_image(image, self.get_alloc_callbacks()) }
    }

    #[inline]
    pub(crate) unsafe fn create_image_view(
        &self,
        create_info: &vk::ImageViewCreateInfo,
    ) -> VkResult<vk::ImageView> {
        unsafe {
            self.device
                .create_image_view(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn destroy_image_view(&self, image_view: vk::ImageView) {
        unsafe {
            self.device
                .destroy_image_view(image_view, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn bind_image_memory(
        &self,
        image: vk::Image,
        device_memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
    ) -> VkResult<()> {
        unsafe { self.device.bind_image_memory(image, device_memory, offset) }
    }

    #[inline]
    pub(crate) unsafe fn bind_buffer_memory(
        &self,
        buffer: vk::Buffer,
        device_memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
    ) -> VkResult<()> {
        unsafe {
            self.device
                .bind_buffer_memory(buffer, device_memory, offset)
        }
    }

    #[inline]
    pub(crate) unsafe fn create_shader_module(
        &self,
        create_info: &vk::ShaderModuleCreateInfo,
    ) -> VkResult<vk::ShaderModule> {
        unsafe {
            self.device
                .create_shader_module(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn destroy_shader_module(&self, shader: vk::ShaderModule) {
        unsafe {
            self.device
                .destroy_shader_module(shader, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn create_pipeline_layout(
        &self,
        create_info: &vk::PipelineLayoutCreateInfo,
    ) -> VkResult<vk::PipelineLayout> {
        unsafe {
            self.device
                .create_pipeline_layout(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn destroy_pipeline_layout(&self, pipeline_layout: vk::PipelineLayout) {
        unsafe {
            self.device
                .destroy_pipeline_layout(pipeline_layout, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn create_descriptor_set_layout(
        &self,
        create_info: &vk::DescriptorSetLayoutCreateInfo,
    ) -> VkResult<vk::DescriptorSetLayout> {
        unsafe {
            self.device
                .create_descriptor_set_layout(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn destroy_descriptor_set_layout(&self, layout: vk::DescriptorSetLayout) {
        unsafe {
            self.device
                .destroy_descriptor_set_layout(layout, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub(crate) unsafe fn create_graphics_pipelines(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::GraphicsPipelineCreateInfo],
    ) -> std::result::Result<Vec<vk::Pipeline>, (Vec<vk::Pipeline>, vk::Result)> {
        unsafe {
            self.device.create_graphics_pipelines(
                pipeline_cache,
                create_infos,
                self.get_alloc_callbacks(),
            )
        }
    }

    #[inline]
    pub(crate) unsafe fn destroy_pipeline(&self, pipeline: vk::Pipeline) {
        unsafe {
            self.device.destroy_pipeline(pipeline, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn create_swapchain(
        &self,
        create_info: &vk::SwapchainCreateInfoKHR,
    ) -> VkResult<vk::SwapchainKHR> {
        unsafe {
            self.swapchain_loader
                .create_swapchain(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn destroy_swapchain(&self, swapchain: vk::SwapchainKHR) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(swapchain, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn get_swapchain_images(
        &self,
        swapchain: vk::SwapchainKHR,
    ) -> VkResult<Vec<vk::Image>> {
        unsafe { self.swapchain_loader.get_swapchain_images(swapchain) }
    }

    #[inline]
    pub unsafe fn create_command_pool(
        &self,
        create_info: &vk::CommandPoolCreateInfo,
    ) -> VkResult<vk::CommandPool> {
        unsafe {
            self.device
                .create_command_pool(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn destroy_command_pool(&self, command_pool: vk::CommandPool) {
        unsafe {
            self.device
                .destroy_command_pool(command_pool, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn create_fence(&self, create_info: &vk::FenceCreateInfo) -> VkResult<vk::Fence> {
        unsafe {
            self.device
                .create_fence(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn destroy_fence(&self, fence: vk::Fence) {
        unsafe { self.device.destroy_fence(fence, self.get_alloc_callbacks()) }
    }

    #[inline]
    pub fn find_viable_depth_stencil_format(&self) -> Option<vk::Format> {
        let formats = [
            ash::vk::Format::D32_SFLOAT_S8_UINT,
            ash::vk::Format::D24_UNORM_S8_UINT,
            ash::vk::Format::D16_UNORM_S8_UINT,
        ];

        formats
            .into_iter()
            .filter_map(|f| {
                let properties = unsafe { self.get_physical_device_format_properties(f) };

                if properties
                    .optimal_tiling_features
                    .contains(ash::vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
                {
                    Some(f)
                } else {
                    None
                }
            })
            .next()
    }

    #[inline]
    pub unsafe fn create_surface(
        &self,
        window: &winit::window::Window,
    ) -> Result<ash::vk::SurfaceKHR> {
        use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

        let display_handle = window.display_handle()?;
        let window_handle = window.window_handle()?;

        let surface = unsafe {
            ash_window::create_surface(
                &self.instance.entry,
                &self.instance.instance,
                display_handle.as_raw(),
                window_handle.as_raw(),
                self.get_alloc_callbacks(),
            )
        }?;

        Ok(surface)
    }

    #[inline]
    pub unsafe fn destroy_surface(&self, surface: vk::SurfaceKHR) {
        unsafe {
            self.instance
                .surface_loader
                .destroy_surface(surface, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn create_semaphore(
        &self,
        create_info: &vk::SemaphoreCreateInfo,
    ) -> VkResult<vk::Semaphore> {
        unsafe {
            self.device
                .create_semaphore(create_info, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn destroy_semaphore(&self, semaphore: vk::Semaphore) {
        unsafe {
            self.device
                .destroy_semaphore(semaphore, self.get_alloc_callbacks())
        }
    }

    #[inline]
    pub unsafe fn allocate_command_buffers(
        &self,
        allocate_info: &vk::CommandBufferAllocateInfo,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        unsafe { self.device.allocate_command_buffers(allocate_info) }
    }

    #[inline]
    pub unsafe fn free_command_buffers(
        &self,
        command_pool: vk::CommandPool,
        command_buffers: &[vk::CommandBuffer],
    ) {
        unsafe {
            self.device
                .free_command_buffers(command_pool, command_buffers);
        }
    }

    #[inline]
    pub unsafe fn begin_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        begin_info: &vk::CommandBufferBeginInfo,
    ) -> VkResult<()> {
        unsafe { self.device.begin_command_buffer(command_buffer, begin_info) }
    }

    #[inline]
    pub unsafe fn end_command_buffer(&self, command_buffer: vk::CommandBuffer) -> VkResult<()> {
        unsafe { self.device.end_command_buffer(command_buffer) }
    }

    #[inline]
    pub unsafe fn cmd_begin_rendering(
        &self,
        command_buffer: vk::CommandBuffer,
        rendering_info: &vk::RenderingInfo,
    ) {
        unsafe {
            self.device
                .cmd_begin_rendering(command_buffer, rendering_info)
        }
    }

    #[inline]
    pub unsafe fn cmd_end_rendering(&self, command_buffer: vk::CommandBuffer) {
        unsafe { self.device.cmd_end_rendering(command_buffer) }
    }

    #[inline]
    pub unsafe fn wait_for_fences(&self, fences: &[vk::Fence]) -> VkResult<()> {
        unsafe { self.device.wait_for_fences(fences, true, u64::MAX) }
    }

    #[inline]
    pub(crate) unsafe fn acquire_next_image(
        &self,
        swapchain: vk::SwapchainKHR,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> VkResult<(u32, bool)> {
        unsafe {
            self.swapchain_loader
                .acquire_next_image(swapchain, u64::MAX, semaphore, fence)
        }
    }

    #[inline]
    pub unsafe fn queue_submit(
        &self,
        submits: &[vk::SubmitInfo],
        fence: vk::Fence,
    ) -> VkResult<()> {
        unsafe { self.device.queue_submit(self.queue, submits, fence) }
    }

    #[inline]
    pub unsafe fn queue_present(&self, present_info: &vk::PresentInfoKHR) -> VkResult<bool> {
        unsafe {
            self.swapchain_loader
                .queue_present(self.queue, present_info)
        }
    }

    #[inline]
    pub unsafe fn reset_fences(&self, fences: &[vk::Fence]) -> VkResult<()> {
        unsafe { self.device.reset_fences(fences) }
    }

    #[inline]
    pub unsafe fn reset_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        flags: vk::CommandBufferResetFlags,
    ) -> VkResult<()> {
        unsafe { self.device.reset_command_buffer(command_buffer, flags) }
    }

    #[inline]
    pub unsafe fn cmd_pipeline_barrier2(
        &self,
        command_buffer: vk::CommandBuffer,
        dependency_info: &vk::DependencyInfo,
    ) {
        unsafe {
            self.device
                .cmd_pipeline_barrier2(command_buffer, dependency_info);
        }
    }

    #[inline]
    pub unsafe fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.device.device_wait_idle() }
    }

    #[inline]
    pub(crate) unsafe fn cmd_bind_pipeline(&self, command_buffer: vk::CommandBuffer, pipeline_bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline) {
        unsafe { self.device.cmd_bind_pipeline(command_buffer, pipeline_bind_point, pipeline) }
    }

    #[inline]
    pub unsafe fn cmd_set_viewport(&self, command_buffer: vk::CommandBuffer, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe { self.device.cmd_set_viewport(command_buffer, first_viewport, viewports) }
    }

    #[inline]
    pub unsafe  fn cmd_set_scissor(&self, command_buffer: vk::CommandBuffer, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe { self.device.cmd_set_scissor(command_buffer, first_scissor, scissors) }
    }

    #[inline]
    pub(crate) unsafe fn cmd_bind_vertex_buffers(&self, command_buffer: vk::CommandBuffer, first_binding: u32, buffers: &[vk::Buffer], offsets: &[vk::DeviceSize]) {
        unsafe { self.device.cmd_bind_vertex_buffers(command_buffer, first_binding, buffers, offsets) }
    }

    #[inline]
    pub(crate) unsafe fn cmd_bind_index_buffers(&self, command_buffer: vk::CommandBuffer, buffer: vk::Buffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        unsafe { self.device.cmd_bind_index_buffer(command_buffer, buffer, offset, index_type) }
    }

    #[inline]
    pub(crate) unsafe fn cmd_draw_indexed(&self, command_buffer: vk::CommandBuffer, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe { self.device.cmd_draw_indexed(command_buffer, index_count, instance_count, first_index, vertex_offset, first_instance) }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(self.get_alloc_callbacks());
            self.instance
                .debug_utils
                .destroy_debug_utils_messenger(self.debug_messenger, self.get_alloc_callbacks());
        }
    }
}
