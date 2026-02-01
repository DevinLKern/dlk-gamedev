use crate::{Error, Result, trace_error};
use ash::vk::{self, AllocationCallbacks};
use ash::prelude::VkResult;

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
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}",
    );

    vk::FALSE
}

#[allow(dead_code)]
pub struct Instance {
    pub(crate) entry: ash::Entry,
    instance: ash::Instance,
    allocation_callbacks: Option<vk::AllocationCallbacks<'static>>,
    debug_utils: Option<ash::ext::debug_utils::Instance>,
    pub(crate) surface_loader: ash::khr::surface::Instance,
}

pub type SharedInstanceRef = std::sync::Arc<Instance>;

impl Instance {
    pub fn new(
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<SharedInstanceRef> {
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

        let debug_utils = if debug_enabled {
                Some(ash::ext::debug_utils::Instance::new(&entry, &instance))
            } else {
                None
            };

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        Ok(std::sync::Arc::new(Instance {
            entry,
            instance,
            allocation_callbacks,
            debug_utils,
            surface_loader,
        }))
    }
    #[inline]
    pub const fn allocation_callbacks_ref(&self) -> Option<&AllocationCallbacks<'_>> {
        self.allocation_callbacks.as_ref()
    }
    #[inline] pub const fn raw(&self) -> &ash::Instance {
        &self.instance
    }
    pub fn create_debug_utils_messenger(&self) -> VkResult<Option<vk::DebugUtilsMessengerEXT>> {
        if let Some(utils) = self.debug_utils.as_ref() {
            let create_info = vk::DebugUtilsMessengerCreateInfoEXT {
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
            
            let res = unsafe { utils.create_debug_utils_messenger(&create_info, self.allocation_callbacks_ref()) }?;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
    pub unsafe fn destroy_debug_utils_messenger(&self, messenger: vk::DebugUtilsMessengerEXT) {
        if let Some(utils) = self.debug_utils.as_ref() {
            unsafe { utils.destroy_debug_utils_messenger(messenger, self.allocation_callbacks_ref()) };
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .destroy_instance(self.allocation_callbacks_ref());
        }
    }
}

