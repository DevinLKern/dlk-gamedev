use crate::device::SharedDeviceRef;
use crate::{descriptor::DescriptorSetLayout, result::Result};
use ash::vk::{self, GraphicsPipelineCreateInfo};
use std::rc::Rc;

pub struct PipelineLayout {
    // maps name to the set number and information about the set
    device: SharedDeviceRef,
    pub bind_point: vk::PipelineBindPoint,
    set_layouts: Box<[crate::DescriptorSetLayout]>,
    pub handle: vk::PipelineLayout,
}

impl std::fmt::Display for PipelineLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{set_layouts: [")?;
        for layout in self.set_layouts.iter() {
            write!(f, "{}", layout)?;
        }
        write!(f, "], ")?;

        write!(f, "handle: {:?}}}", self.handle)
    }
}

impl PipelineLayout {
    // bindings should be sorted such that bindings[0] corresponds to set 0
    pub fn new(device: SharedDeviceRef, set_bindings: &[&[vk::DescriptorSetLayoutBinding]]) -> Result<PipelineLayout> {
        let mut set_layouts = Vec::<crate::DescriptorSetLayout>::new();
        for (set, bindings) in set_bindings.iter().enumerate() {
            let set_layout = crate::DescriptorSetLayout::new(
                device.clone(),
                set as u32,
                bindings,
            )?;
            set_layouts.push(set_layout);
        }
        let set_layouts = set_layouts.into_boxed_slice();
        
        let handle = {
            let vk_set_layouts: Box<[vk::DescriptorSetLayout]> = set_layouts.iter().map(|dsl| dsl.handle).collect();
            let create_info = vk::PipelineLayoutCreateInfo {
                set_layout_count: vk_set_layouts.len() as u32,
                p_set_layouts: vk_set_layouts.as_ptr(),
                ..Default::default()
            };

            unsafe { device.create_pipeline_layout(&create_info) }?
        };
        
        Ok(PipelineLayout { device, bind_point: vk::PipelineBindPoint::GRAPHICS, set_layouts, handle })
    }
    #[inline]
    pub fn get_set_layouts(&self) -> &[DescriptorSetLayout] {
        &self.set_layouts
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline_layout(self.handle);
        }
    }
}

#[allow(dead_code)]
pub struct Pipeline {
    device: SharedDeviceRef,
    layout: Rc<PipelineLayout>,
    pipeline: vk::Pipeline,
}

impl Pipeline {
    pub fn new_graphics(
        device: SharedDeviceRef,
        layout: Rc<PipelineLayout>,
        create_info: &GraphicsPipelineCreateInfo,
    ) -> Result<Self> {
        let pipeline_create_info = [*create_info];
        let pipelines = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_create_info)
        }
        .map_err(|(_, vk_err)| vk_err)?;

        Ok(Pipeline {
            device,
            layout: layout,
            pipeline: pipelines[0],
        })
    }

    pub unsafe fn bind(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device
                .cmd_bind_pipeline(command_buffer, self.layout.bind_point, self.pipeline)
        }
    }

    #[inline]
    pub fn get_layout(&self) -> &PipelineLayout {
        &self.layout
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe { self.device.destroy_pipeline(self.pipeline) }
    }
}
