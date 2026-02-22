use crate::device::SharedDeviceRef;
use crate::trace_error;
use crate::{
    descriptor::DescriptorSetLayout,
    result::Result,
};
use ash::vk::{self, GraphicsPipelineCreateInfo};
use spirv;
use std::collections::HashMap;
use std::rc::Rc;

pub struct PipelineLayout {
    // maps name to the set number and information about the set
    device: SharedDeviceRef,
    pub bind_point: vk::PipelineBindPoint,
    set_layouts: Box<[crate::descriptor::DescriptorSetLayout]>,
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

fn infer_vk_descriptor_type(
    ty: &spirv::TypeInfo,
    storage_class: u32,
) -> Option<vk::DescriptorType> {
    use spirv::TypeInfo;
    let ty = match ty {
        TypeInfo::Pointer { ptr_type } => ptr_type.as_ref(),
        _ => ty,
    };
    let descriptor_type = match (ty, storage_class) {
        (TypeInfo::Sampler, spirv::STORAGE_CLASS_UNIFORM_CONSTANT) => vk::DescriptorType::SAMPLER,
        (TypeInfo::SampledImage { .. }, spirv::STORAGE_CLASS_UNIFORM_CONSTANT) => {
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER
        }
        (
            TypeInfo::Image {
                sampled,
                dimentionality,
                ..
            },
            spirv::STORAGE_CLASS_UNIFORM_CONSTANT,
        ) => {
            if *dimentionality != spirv::DIM_BUFFER {
                if *sampled == 0 {
                    vk::DescriptorType::SAMPLED_IMAGE
                } else {
                    vk::DescriptorType::STORAGE_IMAGE
                }
            } else {
                if *sampled == 0 {
                    vk::DescriptorType::UNIFORM_TEXEL_BUFFER
                } else {
                    vk::DescriptorType::STORAGE_TEXEL_BUFFER
                }
            }
        }
        (TypeInfo::Struct { .. }, spirv::STORAGE_CLASS_UNIFORM) => {
            vk::DescriptorType::UNIFORM_BUFFER
        }
        (TypeInfo::Struct { .. }, spirv::STORAGE_CLASS_STORAGE_BUFFER) => {
            vk::DescriptorType::STORAGE_BUFFER
        }
        _ => {
            return None;
        }
    };

    Some(descriptor_type)
}

impl PipelineLayout {
    pub fn new(
        device: SharedDeviceRef,
        vert_module: &spirv::Module,
        frag_module: &spirv::Module,
    ) -> Result<PipelineLayout> {
        let vert_descriptor_sets = vert_module.get_uniform_info();
        let frag_descriptor_sets = frag_module.get_uniform_info();

        let mut descriptor_set_bindings1 =
            HashMap::<(u32, u32), vk::DescriptorSetLayoutBinding>::new();
        for uniform in vert_descriptor_sets {
            let binding_info = descriptor_set_bindings1
                .entry((uniform.set, uniform.binding))
                .or_default();
            binding_info.binding = uniform.binding;
            binding_info.descriptor_type =
                infer_vk_descriptor_type(&uniform.ty, uniform.storage_class).unwrap();
            binding_info.descriptor_count = uniform.descriptor_count;
            binding_info.stage_flags |= vk::ShaderStageFlags::VERTEX;
        }
        for uniform in frag_descriptor_sets {
            let binding_info = descriptor_set_bindings1
                .entry((uniform.set, uniform.binding))
                .or_default();
            binding_info.binding = uniform.binding;
            binding_info.descriptor_type =
                infer_vk_descriptor_type(&uniform.ty, uniform.storage_class).unwrap();
            binding_info.descriptor_count = uniform.descriptor_count;
            binding_info.stage_flags |= vk::ShaderStageFlags::FRAGMENT;
        }

        let mut descriptor_set_bindings2 =
            HashMap::<u32, Vec<vk::DescriptorSetLayoutBinding>>::new();
        for ((set, _), uniform) in descriptor_set_bindings1.into_iter() {
            let entry = descriptor_set_bindings2.entry(set).or_default();
            entry.push(uniform);
        }

        let mut descriptor_set_layouts = Vec::<crate::DescriptorSetLayout>::new();
        for (set, bindings) in descriptor_set_bindings2.into_iter() {
            let descriptor_set_layout = crate::DescriptorSetLayout::new_raw(
                device.clone(),
                set,
                bindings.into_boxed_slice(),
            )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }
        descriptor_set_layouts.sort_by(|a, b| a.set.cmp(&b.set));

        let handle = {
            let dsl_raw: Box<[vk::DescriptorSetLayout]> = descriptor_set_layouts
                .iter()
                .map(|dsl| dsl.handle)
                .collect();
            let create_info = vk::PipelineLayoutCreateInfo {
                set_layout_count: dsl_raw.len() as u32,
                p_set_layouts: dsl_raw.as_ptr(),
                ..Default::default()
            };

            unsafe { device.create_pipeline_layout(&create_info) }?
        };

        Ok(PipelineLayout {
            device,
            handle,
            bind_point: vk::PipelineBindPoint::GRAPHICS,
            set_layouts: descriptor_set_layouts.into_boxed_slice(),
        })
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
        .map_err(|(_, vk_err)| vk_err)
        .inspect_err(|e| trace_error!(e))?;

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
