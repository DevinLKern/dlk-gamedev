use crate::device::Device;
use crate::result::Result;
use std::rc::Rc;

use ash::prelude::VkResult;
use ash::vk;

fn spirv_uniform_type_to_vk_descriptor_type(
    uniform_type: &spirv::UniformType,
) -> vk::DescriptorType {
    match uniform_type {
        spirv::UniformType::Sampler => vk::DescriptorType::SAMPLER,
        spirv::UniformType::SampledImage => vk::DescriptorType::COMBINED_IMAGE_SAMPLER, // TODO: fix this. it's VERY questionable.
        spirv::UniformType::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
        spirv::UniformType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        spirv::UniformType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        _ => ash::vk::DescriptorType::UNIFORM_BUFFER,
    }
}

#[allow(dead_code)]
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

// #[derive(Debug)]
pub struct DescriptorSetLayout {
    device: Rc<crate::device::Device>,
    pub set: u32,
    pub bindings: Box<[DescriptorSetLayoutBindingInfo]>,
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub(crate) fn new(
        device: Rc<crate::device::Device>,
        set: u32,
        bindings: &[(vk::ShaderStageFlags, spirv::UniformInfo)],
    ) -> VkResult<DescriptorSetLayout> {
        let owned_bindings: Box<[DescriptorSetLayoutBindingInfo]> = bindings
            .iter()
            .map(|(f, u)| DescriptorSetLayoutBindingInfo {
                binding: u.binding,
                descriptor_type: spirv_uniform_type_to_vk_descriptor_type(&u.uniform_type),
                descriptor_count: 1,
                stage_flags: *f,
                p_immutable_shader: std::ptr::null(),
                size: u.size,
            })
            .collect();

        let handle = {
            let vk_bindings: Box<[vk::DescriptorSetLayoutBinding<'_>]> = bindings
                .iter()
                .map(|(f, u)| vk::DescriptorSetLayoutBinding {
                    binding: u.binding,
                    descriptor_type: spirv_uniform_type_to_vk_descriptor_type(&u.uniform_type),
                    descriptor_count: 1,
                    stage_flags: *f,
                    ..Default::default()
                })
                .collect();
            let create_info = vk::DescriptorSetLayoutCreateInfo {
                binding_count: vk_bindings.len() as u32,
                p_bindings: vk_bindings.as_ptr(),
                ..Default::default()
            };
            unsafe { device.create_descriptor_set_layout(&create_info) }?
        };

        Ok(DescriptorSetLayout {
            device,
            bindings: owned_bindings,
            set,
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

pub struct DescriptorPool {
    device: Rc<Device>,
    handle: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(device: Rc<Device>, create_info: &vk::DescriptorPoolCreateInfo) -> Result<Self> {
        let pool = unsafe { device.create_descriptor_pool(create_info) }?;
        Ok(Self {
            device,
            handle: pool,
        })
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.handle) }
    }
}

#[allow(dead_code)]
pub struct DescriptorSet {
    device: Rc<Device>,
    pool: Rc<DescriptorPool>,
    pub handle: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn allocate(
        device: Rc<Device>,
        pool: Rc<DescriptorPool>,
        set_layouts: &[vk::DescriptorSetLayout],
    ) -> Result<Box<[Self]>> {
        let allocate_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool.handle,
            descriptor_set_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&allocate_info) }?;

        Ok(descriptor_sets
            .into_iter()
            .map(|set| Self {
                device: device.clone(),
                pool: pool.clone(),
                handle: set,
            })
            .collect())
    }
}
