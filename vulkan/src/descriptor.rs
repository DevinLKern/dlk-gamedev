use crate::device::SharedDeviceRef;

use ash::prelude::VkResult;
use ash::vk;

#[derive(Debug)]
pub struct DescriptorSetLayoutBindingInfo {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub p_immutable_shader: *const vk::Sampler,
    pub size: Option<u32>,
}

impl std::fmt::Display for DescriptorSetLayoutBindingInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{binding: {}, descriptor_type: {:?}, descriptor_count: {:?}, stage_flags: {:?}, size: {:?}}}",
            self.binding, self.descriptor_type, self.descriptor_count, self.stage_flags, self.size,
        )
    }
}

pub struct DescriptorSetLayout {
    device: SharedDeviceRef,
    pub set: u32,
    pub bindings: Box<[DescriptorSetLayoutBindingInfo]>,
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub fn new(
        device: SharedDeviceRef,
        set: u32,
        bindings: &[vk::DescriptorSetLayoutBinding],
    ) -> VkResult<DescriptorSetLayout> {
        let create_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };

        let handle = unsafe { device.create_descriptor_set_layout(&create_info) }?;

        Ok(DescriptorSetLayout {
            device,
            set,
            bindings: bindings
                .into_iter()
                .map(|b| DescriptorSetLayoutBindingInfo {
                    binding: b.binding,
                    descriptor_type: b.descriptor_type,
                    descriptor_count: b.descriptor_count,
                    stage_flags: b.stage_flags,
                    p_immutable_shader: b.p_immutable_samplers,
                    size: None,
                })
                .collect(),
            handle,
        })
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.handle);
        }
    }
}

impl std::fmt::Display for DescriptorSetLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{bindings: [")?;
        for binding in self.bindings.iter() {
            write!(
                f,
                "{{binding: {}, descriptor_type: {:?}, descriptor_count: {:?}, stage_flags: {:?}, size: {:?}}}",
                binding.binding,
                binding.descriptor_type,
                binding.descriptor_count,
                binding.stage_flags,
                binding.size
            )?;
        }
        write!(f, "], handle: {:?}}}", self.handle)
    }
}
