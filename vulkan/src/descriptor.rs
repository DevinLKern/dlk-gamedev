use crate::device::SharedDeviceRef;
use crate::result::Result;
use std::rc::Rc;

use ash::prelude::VkResult;
use ash::vk;

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

pub struct DescriptorSetLayout {
    device: SharedDeviceRef,
    pub set: u32,
    pub bindings: Box<[DescriptorSetLayoutBindingInfo]>,
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub fn new_raw(
        device: SharedDeviceRef,
        set: u32,
        bindings: Box<[vk::DescriptorSetLayoutBinding]>,
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

pub struct DescriptorPool {
    device: SharedDeviceRef,
    handle: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(
        device: SharedDeviceRef,
        create_info: &vk::DescriptorPoolCreateInfo,
    ) -> Result<Self> {
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
    device: SharedDeviceRef,
    pool: Rc<DescriptorPool>,
    pipeline_layout: Rc<crate::PipelineLayout>,
    set_index: u32,
    pub handle: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn allocate(
        device: SharedDeviceRef,
        pool: Rc<DescriptorPool>,
        set_index: u32,
        pipeline_layout: Rc<crate::PipelineLayout>,
    ) -> Result<Self> {
        let set_layouts = [pipeline_layout.get_set_layouts()[set_index as usize].handle];
        let allocate_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool.handle,
            descriptor_set_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&allocate_info) }?;

        Ok(Self {
            device,
            pool,
            pipeline_layout,
            set_index,
            handle: descriptor_sets[0],
        })
    }

    pub fn bind(&self, command_buffer: vk::CommandBuffer, dynamic_offsets: &[u32]) {
        let sets = [self.handle];
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                self.pipeline_layout.bind_point,
                self.pipeline_layout.handle,
                self.set_index,
                &sets,
                dynamic_offsets,
            );
        }
    }
}
