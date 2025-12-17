use crate::device::Device;
use crate::result::Result;
use std::rc::Rc;

use ash::vk;

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
