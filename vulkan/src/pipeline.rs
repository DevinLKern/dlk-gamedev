use crate::result::{Error, Result};
use crate::trace_error;
use ash::prelude::VkResult;
use ash::vk;
use spirv;
use std::collections::HashMap;
use std::rc::Rc;
use std::io::Read;


fn spirv_type_to_vk_format(spirv_type: &spirv::ShaderIoType) -> vk::Format {
    match spirv_type {
        spirv::ShaderIoType::Vector {
            component_type,
            component_width,
            component_count,
        } => match (component_type, component_width, component_count) {
            (spirv::ScalarType::Int, 16, 1) => vk::Format::R16_SINT,
            (spirv::ScalarType::Int, 16, 2) => vk::Format::R16G16_SINT,
            (spirv::ScalarType::Int, 16, 3) => vk::Format::R16G16B16_SINT,
            (spirv::ScalarType::Int, 16, 4) => vk::Format::R16G16B16A16_SINT,

            (spirv::ScalarType::Unsigned, 16, 1) => vk::Format::R16_UINT,
            (spirv::ScalarType::Unsigned, 16, 2) => vk::Format::R16G16_UINT,
            (spirv::ScalarType::Unsigned, 16, 3) => vk::Format::R16G16B16_UINT,
            (spirv::ScalarType::Unsigned, 16, 4) => vk::Format::R16G16B16A16_UINT,

            (spirv::ScalarType::Float, 16, 1) => vk::Format::R16_SFLOAT,
            (spirv::ScalarType::Float, 16, 2) => vk::Format::R16G16_SFLOAT,
            (spirv::ScalarType::Float, 16, 3) => vk::Format::R16G16B16_SFLOAT,
            (spirv::ScalarType::Float, 16, 4) => vk::Format::R16G16B16A16_SFLOAT,

            (spirv::ScalarType::Int, 32, 1) => vk::Format::R32_SINT,
            (spirv::ScalarType::Int, 32, 2) => vk::Format::R32G32_SINT,
            (spirv::ScalarType::Int, 32, 3) => vk::Format::R32G32B32_SINT,
            (spirv::ScalarType::Int, 32, 4) => vk::Format::R32G32B32A32_SINT,

            (spirv::ScalarType::Unsigned, 32, 1) => vk::Format::R32_UINT,
            (spirv::ScalarType::Unsigned, 32, 2) => vk::Format::R32G32_UINT,
            (spirv::ScalarType::Unsigned, 32, 3) => vk::Format::R32G32B32_UINT,
            (spirv::ScalarType::Unsigned, 32, 4) => vk::Format::R32G32B32A32_UINT,

            (spirv::ScalarType::Float, 32, 1) => vk::Format::R32_SFLOAT,
            (spirv::ScalarType::Float, 32, 2) => vk::Format::R32G32_SFLOAT,
            (spirv::ScalarType::Float, 32, 3) => vk::Format::R32G32B32_SFLOAT,
            (spirv::ScalarType::Float, 32, 4) => vk::Format::R32G32B32A32_SFLOAT,

            _ => ash::vk::Format::UNDEFINED,
        },
        _ => ash::vk::Format::UNDEFINED,
    }
}

fn spirv_uniform_type_to_vk_descriptor_type(
    uniform_type: &spirv::UniformType,
) -> ash::vk::DescriptorType {
    match uniform_type {
        spirv::UniformType::Sampler => vk::DescriptorType::SAMPLER,
        spirv::UniformType::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
        spirv::UniformType::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
        spirv::UniformType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        spirv::UniformType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        _ => ash::vk::DescriptorType::UNIFORM_BUFFER,
    }
}

pub unsafe fn create_shader_modules(
    device: Rc<crate::device::Device>,
    shader_path: String,
) -> Result<(spirv::ShaderModule, vk::ShaderModule)> {
    let shader_code = {
        let mut file = std::fs::File::open(shader_path)?;

        let mut data = Vec::<u8>::new();

        let _ = file.read_to_end(&mut data)?;

        data 
    };

    let spv_module = spirv::ShaderModule::from_code(shader_code.as_slice())?;

    let vk_module = {
        let shader_module_create_info = vk::ShaderModuleCreateInfo {
            code_size: shader_code.len(),
            p_code: shader_code.as_ptr() as *const u32,
            ..Default::default()
        };

        unsafe { device.create_shader_module(&shader_module_create_info) }?
    };

    Ok((spv_module, vk_module))
}

#[allow(dead_code)]
pub struct OwnedDescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub p_immutable_shader: *const vk::Sampler,
}

pub struct DescriptorSetLayout {
    device: Rc<crate::device::Device>,
    pub name_to_binding: HashMap<Rc<str>, usize>,
    pub bindings: Box<[OwnedDescriptorSetLayoutBinding]>,
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub(crate) fn new(
        device: Rc<crate::device::Device>,
        binding_names: &[(Rc<str>, u32)],
        bindings: &[vk::DescriptorSetLayoutBinding<'_>]
    ) -> VkResult<DescriptorSetLayout> {
        let descriptor_set_layout = {
            let create_info = vk::DescriptorSetLayoutCreateInfo {
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            };

            unsafe { device.create_descriptor_set_layout(&create_info)? }
        };

        let owned_bindings: Box<[OwnedDescriptorSetLayoutBinding]>  = bindings
            .iter()
            .map(|b| OwnedDescriptorSetLayoutBinding {
                binding: b.binding,
                descriptor_type: b.descriptor_type,
                descriptor_count: b.descriptor_count,
                stage_flags: b.stage_flags,
                p_immutable_shader: b.p_immutable_samplers,
            }).collect();

        let mut name_to_binding = HashMap::<Rc<str>, usize>::new();
        for (name, binding) in binding_names.iter() {
            let index = owned_bindings
                .iter()
                .enumerate()
                .find_map(|(i, b)| {
                    if b.binding == *binding {
                        Some(i)
                    } else {
                        None
                    }
                });
            if let Some(i) = index {
                name_to_binding.insert(name.clone(), i);
            }
        }

        Ok(DescriptorSetLayout{
            device,
            name_to_binding,
            bindings: owned_bindings,
            handle: descriptor_set_layout
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
        for (name, binding_index) in self.name_to_binding.iter() {
            if let Some(binding) = self.bindings.get(*binding_index) {
                write!(
                    f,
                    "\'{}\': {{binding: {}, descriptor_type: {:?}, descriptor_count: {:?}, stage_flags: {:?}}}",
                    name,
                    binding.binding,
                    binding.descriptor_type,
                    binding.descriptor_count,
                    binding.stage_flags
                )?;
            }
        }
        write!(f, "], handle: {:?}}}", self.handle)
    }
}

pub struct PipelineLayout {
    // maps name to the set number and information about the set
    device: Rc<crate::device::Device>,
    set_layouts: Box<[DescriptorSetLayout]>,
    handle: vk::PipelineLayout,
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

impl<'a> PipelineLayout {
    fn new(
        device: Rc<crate::device::Device>,
        vert_spv_module: &spirv::ShaderModule,
        frag_spv_module: &spirv::ShaderModule,
    ) -> Result<PipelineLayout> {
        // maps (set, binding) to the uniforms stages, type and name
        let mut set_infos = HashMap::<
            (u32, u32),
            (vk::ShaderStageFlags, spirv::UniformType, Option<Rc<str>>),
        >::new();

        let vert_uniforms = vert_spv_module.get_uniforms()?;
        for u in vert_uniforms.into_iter() {
            set_infos.insert(
                (u.set, u.binding),
                (vk::ShaderStageFlags::VERTEX, u.uniform_type, u.name),
            );
        }
        let frag_uniforms = frag_spv_module.get_uniforms()?;
        for u in frag_uniforms.into_iter() {
            if let Some((flags, uniform_type, name)) = set_infos.get_mut(&(u.set, u.binding)) {
                if *uniform_type != u.uniform_type || *name != u.name {
                    return Err(Error::NotImplemented); // TODO: add type
                }
                *flags |= vk::ShaderStageFlags::FRAGMENT;
                continue;
            }
            set_infos.insert(
                (u.set, u.binding),
                (vk::ShaderStageFlags::FRAGMENT, u.uniform_type, u.name),
            );
        }

        let mut set_bindings = HashMap::<u32, Vec<vk::DescriptorSetLayoutBinding>>::new();
        let mut set_names = HashMap::<Rc<str>, (u32, u32)>::new();
        for ((set, binding), (stage_flags, uniform_type, name)) in set_infos.into_iter() {
            if let Some(name) = name {
                if let Some(_) = set_names.insert(name, (set, binding)) {
                    return Err(Error::NotImplemented); // TODO: add type
                }
            }
            let binding = vk::DescriptorSetLayoutBinding {
                binding,
                descriptor_type: spirv_uniform_type_to_vk_descriptor_type(&uniform_type),
                descriptor_count: 1,
                stage_flags,
                ..Default::default()
            };
            if let Some(bindings) = set_bindings.get_mut(&set) {
                bindings.push(binding);
            } else {
                set_bindings.insert(set, vec![binding]);
            }
        }

        let mut set_layouts = Vec::new();
        for (set, bindings) in set_bindings.into_iter() {
            let binding_names: Box<[(Rc<str>, u32)]> = set_names.iter().filter_map(|(n, (s, b))| {
                if *s == set {
                    Some((n.clone(), *b))
                } else {
                    None
                }
            }).collect();
            let set_layout = DescriptorSetLayout::new(
                device.clone(),
                &binding_names,
                bindings.as_slice()
            )?;

            set_layouts.push(set_layout);
        }

        let pipeline_layout = {
            let layouts: Box<[vk::DescriptorSetLayout]> = set_layouts.iter().map(|l| l.handle).collect();
            let pipeline_layout_ceate_info = vk::PipelineLayoutCreateInfo {
                set_layout_count: layouts.len() as u32,
                p_set_layouts: layouts.as_ptr(),
                ..Default::default()
            };

            unsafe { device.create_pipeline_layout(&pipeline_layout_ceate_info) }?
        };

        Ok(PipelineLayout {
            device,
            set_layouts: set_layouts.into_boxed_slice(),
            handle: pipeline_layout
        })
    }
}

impl<'a> Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline_layout(self.handle);
        }
    }
}

#[allow(dead_code)]
pub struct Pipeline {
    device: Rc<crate::device::Device>,
    layout: PipelineLayout,
    pipeline: vk::Pipeline,
}

pub enum PipelineCreateInfo {
    Graphics {
        vk_vertex_shader_module: vk::ShaderModule,
        spv_vertex_shader_module: spirv::ShaderModule,
        vk_frag_shader_module: vk::ShaderModule,
        spv_frag_shader_module: spirv::ShaderModule,
        color_formats: Rc<[vk::Format]>,
        depth_format: vk::Format,
        stencil_format: vk::Format,
    },
}

impl Pipeline {
    pub fn new(
        device: Rc<crate::device::Device>,
        create_info: &PipelineCreateInfo,
    ) -> Result<Pipeline> {
        match create_info {
            PipelineCreateInfo::Graphics {
                vk_vertex_shader_module,
                spv_vertex_shader_module,
                vk_frag_shader_module,
                spv_frag_shader_module,
                color_formats,
                depth_format,
                stencil_format,
            } => {
                let pipeline_layout =
                    PipelineLayout::new(device.clone(), &spv_vertex_shader_module, &spv_frag_shader_module)
                        .inspect_err(|e| trace_error!(e))?;

                let pipeline = {
                    let vert_entry_point_name = spv_vertex_shader_module
                        .get_input_names()
                        .iter()
                        .find_map(|s| {
                            if s.as_ref() == "main" {
                                std::ffi::CString::new(s.as_ref()).ok()
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| {
                            Error::CouldNotDetermineEntryPointName
                        })
                        .inspect_err(|e| trace_error!(e))?;
                    let frag_entry_point_name = spv_frag_shader_module
                        .get_input_names()
                        .iter()
                        .find_map(|s| {
                            if s.as_ref() == "main" {
                                std::ffi::CString::new(s.as_ref()).ok()
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| {
                            Error::CouldNotDetermineEntryPointName
                        })
                        .inspect_err(|e| trace_error!(e))?;
                    let stages = {
                        let vert_stage = vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::VERTEX,
                            module: *vk_vertex_shader_module,
                            p_name: vert_entry_point_name.as_ptr(),
                            ..Default::default()
                        };
                        let frag_stage = vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::FRAGMENT,
                            module: *vk_frag_shader_module,
                            p_name: frag_entry_point_name.as_ptr(),
                            ..Default::default()
                        };
                        [vert_stage, frag_stage]
                    };

                    let (vertex_input_attribute_descriptions, vertex_input_binding_descriptions) = {
                        let mut inputs = spv_vertex_shader_module.get_inputs()?;
                        let mut vk_input_attributes = Vec::new();
                        for input in inputs.iter() {
                            let attribute = vk::VertexInputAttributeDescription {
                                location: input.location,
                                binding: input.binding,
                                format: spirv_type_to_vk_format(&input.io_type),
                                offset: 0,
                                ..Default::default()
                            };
                            vk_input_attributes.push(attribute);
                        }

                        // sort inputs by location and then by binding
                        vk_input_attributes.sort_by(|a, b| {
                            a.binding.cmp(&b.binding).then_with(|| a.location.cmp(&b.location))
                        });
                        inputs.sort_by(|a, b| {
                            a.binding.cmp(&b.binding).then_with(|| a.location.cmp(&b.location))
                        });

                        let mut vk_binding_descriptions = Vec::new();
                        let (mut l, mut r): (usize, usize) = (0, 0);
                        while r < vk_input_attributes.len() {
                            while r < vk_input_attributes.len() && vk_input_attributes[l].binding == vk_input_attributes[r].binding {
                                r += 1;
                            }

                            let mut stride: u32 = 0;
                            for i in l..r {
                                vk_input_attributes[i].offset = stride;
                                stride += inputs[i].stride;
                            }

                            vk_binding_descriptions.push(vk::VertexInputBindingDescription{
                                binding: vk_input_attributes[l].binding,
                                stride,
                                input_rate: vk::VertexInputRate::VERTEX
                            });

                            l = r;
                        }

                        (vk_input_attributes, vk_binding_descriptions)
                    };
                    let vertex_input_state = ash::vk::PipelineVertexInputStateCreateInfo {
                        vertex_binding_description_count: vertex_input_binding_descriptions.len()
                            as u32,
                        p_vertex_binding_descriptions: vertex_input_binding_descriptions.as_ptr(),
                        vertex_attribute_description_count: vertex_input_attribute_descriptions
                            .len()
                            as u32,
                        p_vertex_attribute_descriptions: vertex_input_attribute_descriptions
                            .as_ptr(),
                        ..Default::default()
                    };
                    let input_assembly_state = ash::vk::PipelineInputAssemblyStateCreateInfo {
                        topology: ash::vk::PrimitiveTopology::TRIANGLE_LIST,
                        primitive_restart_enable: ash::vk::FALSE,
                        ..Default::default()
                    };
                    let viewport_state = ash::vk::PipelineViewportStateCreateInfo {
                        viewport_count: 1,
                        p_viewports: std::ptr::null(), // Since dynamic viewports is enabled this can be null
                        scissor_count: 1,
                        p_scissors: std::ptr::null(), // this is also be dynamic
                        ..Default::default()
                    };
                    let rasterization_state = ash::vk::PipelineRasterizationStateCreateInfo {
                        depth_clamp_enable: ash::vk::FALSE,
                        rasterizer_discard_enable: ash::vk::FALSE,
                        polygon_mode: ash::vk::PolygonMode::FILL,
                        cull_mode: ash::vk::CullModeFlags::NONE,
                        front_face: ash::vk::FrontFace::CLOCKWISE,
                        depth_bias_enable: ash::vk::FALSE,
                        depth_bias_constant_factor: 0.0,
                        depth_bias_clamp: 0.0,
                        depth_bias_slope_factor: 0.0,
                        line_width: 1.0, // dyamic states is on and VK_DYNAMIC_STATE_LINE_WIDTH is not
                        ..Default::default()
                    };
                    let multisample_state = ash::vk::PipelineMultisampleStateCreateInfo {
                        rasterization_samples: ash::vk::SampleCountFlags::TYPE_1,
                        sample_shading_enable: ash::vk::FALSE,
                        ..Default::default()
                    };
                    let depth_stencil_state = ash::vk::PipelineDepthStencilStateCreateInfo {
                        depth_test_enable: ash::vk::TRUE,
                        depth_write_enable: ash::vk::TRUE,
                        depth_compare_op: ash::vk::CompareOp::LESS,
                        depth_bounds_test_enable: ash::vk::FALSE,
                        stencil_test_enable: ash::vk::FALSE,
                        min_depth_bounds: 0.0,
                        max_depth_bounds: 1.0,
                        ..Default::default()
                    };
                    let attachments = [ash::vk::PipelineColorBlendAttachmentState {
                        blend_enable: ash::vk::FALSE,
                        src_color_blend_factor: ash::vk::BlendFactor::ZERO,
                        dst_color_blend_factor: ash::vk::BlendFactor::ZERO,
                        color_blend_op: ash::vk::BlendOp::ADD,
                        src_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
                        dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
                        alpha_blend_op: ash::vk::BlendOp::ADD,
                        color_write_mask: ash::vk::ColorComponentFlags::RGBA,
                    }];
                    let color_blend_state = ash::vk::PipelineColorBlendStateCreateInfo {
                        logic_op_enable: ash::vk::FALSE,
                        logic_op: ash::vk::LogicOp::COPY,
                        attachment_count: attachments.len() as u32,
                        p_attachments: attachments.as_ptr(),
                        blend_constants: [0.0, 0.0, 0.0, 0.0],
                        ..Default::default()
                    };
                    let dynamic_states = [
                        ash::vk::DynamicState::VIEWPORT,
                        ash::vk::DynamicState::SCISSOR,
                    ];
                    let dynamic_state = ash::vk::PipelineDynamicStateCreateInfo {
                        dynamic_state_count: dynamic_states.len() as u32,
                        p_dynamic_states: dynamic_states.as_ptr(),
                        ..Default::default()
                    };
                    let pipeline_rendering_info = vk::PipelineRenderingCreateInfo {
                        color_attachment_count: color_formats.len() as u32,
                        p_color_attachment_formats: color_formats.as_ptr(),
                        depth_attachment_format: *depth_format,
                        stencil_attachment_format: *stencil_format,
                        ..Default::default()
                    };
                    let pipeline_create_info = ash::vk::GraphicsPipelineCreateInfo {
                        p_next: &pipeline_rendering_info as *const _ as *const std::ffi::c_void,
                        stage_count: stages.len() as u32,
                        p_stages: stages.as_ptr(),
                        p_vertex_input_state: &vertex_input_state,
                        p_input_assembly_state: &input_assembly_state,
                        p_tessellation_state: std::ptr::null(),
                        p_viewport_state: &viewport_state,
                        p_rasterization_state: &rasterization_state,
                        p_multisample_state: &multisample_state,
                        p_depth_stencil_state: &depth_stencil_state,
                        p_color_blend_state: &color_blend_state,
                        p_dynamic_state: &dynamic_state,
                        layout: pipeline_layout.handle,
                        render_pass: ash::vk::RenderPass::null(), // dynamic rendering is enabled
                        subpass: 0,
                        ..Default::default()
                    };

                    let pipelines = unsafe {
                        device.create_graphics_pipelines(
                            ash::vk::PipelineCache::null(),
                            &[pipeline_create_info],
                        )
                    }
                    .map_err(|(_, vk_err)| vk_err)
                    .inspect_err(|e| trace_error!(e))?;

                    pipelines[0]
                };

                Ok(Pipeline {
                    device,
                    layout: pipeline_layout,
                    pipeline,
                })
            }
        }
    }

    pub unsafe fn bind(&self, command_buffer: vk::CommandBuffer) {
        unsafe { self.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline) }
    }
}

impl Drop for Pipeline{
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline)
        }
    }
}
