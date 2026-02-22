use ash::vk;

use crate::Result;

use std::io::Read;

pub struct ShaderModule {
    handle: vk::ShaderModule,
    device: crate::device::SharedDeviceRef,
}

impl ShaderModule {
    pub fn new(
        shader_path: &std::path::Path,
        device: crate::device::SharedDeviceRef,
    ) -> Result<ShaderModule> {
        let shader_code = {
            let mut file = std::fs::File::open(shader_path)?;

            let mut data = Vec::<u8>::new();

            let _ = file.read_to_end(&mut data)?;

            data
        };
        let handle = {
            let shader_module_create_info = vk::ShaderModuleCreateInfo {
                code_size: shader_code.len(),
                p_code: shader_code.as_ptr() as *const u32,
                ..Default::default()
            };

            unsafe { device.create_shader_module(&shader_module_create_info) }?
        };

        Ok(ShaderModule { handle, device })
    }

    pub unsafe fn raw(&self) -> &vk::ShaderModule {
        &self.handle
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe { self.device.destroy_shader_module(self.handle) };
    }
}
