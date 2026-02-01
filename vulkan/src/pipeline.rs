use crate::device::SharedDeviceRef;
use crate::trace_error;
use crate::{
    descriptor::DescriptorSetLayout,
    result::{Error, Result},
};
use ash::vk;
use spirv;
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;

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

            _ => vk::Format::UNDEFINED,
        },
        _ => vk::Format::UNDEFINED,
    }
}

pub unsafe fn create_shader_modules(
    device: SharedDeviceRef,
    shader_path: &std::path::Path,
) -> Result<(Rc<spirv::ShaderModule>, vk::ShaderModule)> {
    let shader_code = {
        let mut file = std::fs::File::open(shader_path)?;

        let mut data = Vec::<u8>::new();

        let _ = file.read_to_end(&mut data)?;

        data
    };

    let spv_module = Rc::new(spirv::ShaderModule::from_code(shader_code.as_slice())?);

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

impl PipelineLayout {
    pub fn new_graphics(
        device: SharedDeviceRef,
        vert_spv_module: &spirv::ShaderModule,
        frag_spv_module: &spirv::ShaderModule,
    ) -> Result<PipelineLayout> {
        // maps (set, binding) to the uniforms stages, type and name
        let mut uniform_infos =
            HashMap::<(u32, u32), (vk::ShaderStageFlags, spirv::UniformInfo)>::new();

        let vert_uniforms = vert_spv_module.get_uniforms()?;
        for u in vert_uniforms.into_iter() {
            uniform_infos.insert((u.set, u.binding), (vk::ShaderStageFlags::VERTEX, u));
        }
        let frag_uniforms = frag_spv_module.get_uniforms()?;
        for u in frag_uniforms.into_iter() {
            if let Some((flags, _)) = uniform_infos.get_mut(&(u.set, u.binding)) {
                *flags |= vk::ShaderStageFlags::FRAGMENT;
                continue;
            }
            uniform_infos.insert((u.set, u.binding), (vk::ShaderStageFlags::FRAGMENT, u));
        }

        let mut set_infos = HashMap::<u32, Vec<(vk::ShaderStageFlags, spirv::UniformInfo)>>::new();
        for ((set, _), info) in uniform_infos.into_iter() {
            if let Some(v) = set_infos.get_mut(&set) {
                v.push(info);
                continue;
            }

            set_infos.insert(set, vec![info]);
        }

        let mut set_layouts = Vec::new();
        for (set, infos) in set_infos.into_iter() {
            let layout =
                crate::descriptor::DescriptorSetLayout::new(device.clone(), set, infos.as_slice())?;
            set_layouts.push(layout);
        }

        set_layouts.sort_by(|a, b| a.set.cmp(&b.set));

        let pipeline_layout = {
            let layouts: Box<[vk::DescriptorSetLayout]> =
                set_layouts.iter().map(|l| l.handle).collect();
            let pipeline_layout_ceate_info = vk::PipelineLayoutCreateInfo {
                set_layout_count: layouts.len() as u32,
                p_set_layouts: layouts.as_ptr(),
                ..Default::default()
            };

            unsafe { device.create_pipeline_layout(&pipeline_layout_ceate_info) }?
        };

        Ok(PipelineLayout {
            device,
            bind_point: vk::PipelineBindPoint::GRAPHICS,
            set_layouts: set_layouts.into_boxed_slice(),
            handle: pipeline_layout,
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

pub enum PipelineCreateInfo {
    Graphics {
        vk_vertex_shader_module: vk::ShaderModule,
        spv_vertex_shader_module: Rc<spirv::ShaderModule>,
        vk_frag_shader_module: vk::ShaderModule,
        spv_frag_shader_module: Rc<spirv::ShaderModule>,
        layout: Rc<PipelineLayout>,
        color_formats: Rc<[vk::Format]>,
        depth_format: vk::Format,
        stencil_format: vk::Format,
    },
}

impl Pipeline {
    pub fn new(device: SharedDeviceRef, create_info: &PipelineCreateInfo) -> Result<Pipeline> {
        match create_info {
            PipelineCreateInfo::Graphics {
                vk_vertex_shader_module,
                spv_vertex_shader_module,
                vk_frag_shader_module,
                spv_frag_shader_module,
                layout,
                color_formats,
                depth_format,
                stencil_format,
            } => {
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
                    .ok_or_else(|| Error::CouldNotDetermineEntryPointName)
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
                    .ok_or_else(|| Error::CouldNotDetermineEntryPointName)
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
                    let mut inputs: Vec<spirv::ShaderIoInfo> =
                        spv_vertex_shader_module.get_inputs()?;
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
                        a.binding
                            .cmp(&b.binding)
                            .then_with(|| a.location.cmp(&b.location))
                    });
                    inputs.sort_by(|a, b| {
                        a.binding
                            .cmp(&b.binding)
                            .then_with(|| a.location.cmp(&b.location))
                    });

                    let mut vk_binding_descriptions = Vec::new();
                    let (mut l, mut r): (usize, usize) = (0, 0);
                    while r < vk_input_attributes.len() {
                        while r < vk_input_attributes.len()
                            && vk_input_attributes[l].binding == vk_input_attributes[r].binding
                        {
                            r += 1;
                        }

                        let mut stride: u32 = 0;
                        for i in l..r {
                            vk_input_attributes[i].offset = stride;
                            stride += inputs[i].stride;
                        }

                        vk_binding_descriptions.push(vk::VertexInputBindingDescription {
                            binding: vk_input_attributes[l].binding,
                            stride,
                            input_rate: vk::VertexInputRate::VERTEX,
                        });

                        l = r;
                    }

                    (vk_input_attributes, vk_binding_descriptions)
                };
                let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
                    vertex_binding_description_count: vertex_input_binding_descriptions.len()
                        as u32,
                    p_vertex_binding_descriptions: vertex_input_binding_descriptions.as_ptr(),
                    vertex_attribute_description_count: vertex_input_attribute_descriptions.len()
                        as u32,
                    p_vertex_attribute_descriptions: vertex_input_attribute_descriptions.as_ptr(),
                    ..Default::default()
                };
                let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
                    topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                    primitive_restart_enable: vk::FALSE,
                    ..Default::default()
                };
                let viewport_state = vk::PipelineViewportStateCreateInfo {
                    viewport_count: 1,
                    p_viewports: std::ptr::null(), // Since dynamic viewports is enabled this can be null
                    scissor_count: 1,
                    p_scissors: std::ptr::null(), // this is also be dynamic
                    ..Default::default()
                };
                let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
                    depth_clamp_enable: vk::FALSE,
                    rasterizer_discard_enable: vk::FALSE,
                    polygon_mode: vk::PolygonMode::FILL,
                    cull_mode: vk::CullModeFlags::NONE,
                    front_face: vk::FrontFace::CLOCKWISE,
                    depth_bias_enable: vk::FALSE,
                    depth_bias_constant_factor: 0.0,
                    depth_bias_clamp: 0.0,
                    depth_bias_slope_factor: 0.0,
                    line_width: 1.0, // dyamic states is on and VK_DYNAMIC_STATE_LINE_WIDTH is not
                    ..Default::default()
                };
                let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                    rasterization_samples: vk::SampleCountFlags::TYPE_1,
                    sample_shading_enable: vk::FALSE,
                    ..Default::default()
                };
                let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
                    depth_test_enable: vk::TRUE,
                    depth_write_enable: vk::TRUE,
                    depth_compare_op: vk::CompareOp::LESS,
                    depth_bounds_test_enable: vk::FALSE,
                    stencil_test_enable: vk::FALSE,
                    min_depth_bounds: 0.0,
                    max_depth_bounds: 1.0,
                    ..Default::default()
                };
                let attachments = [vk::PipelineColorBlendAttachmentState {
                    blend_enable: vk::FALSE,
                    src_color_blend_factor: vk::BlendFactor::ZERO,
                    dst_color_blend_factor: vk::BlendFactor::ZERO,
                    color_blend_op: vk::BlendOp::ADD,
                    src_alpha_blend_factor: vk::BlendFactor::ZERO,
                    dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                    alpha_blend_op: vk::BlendOp::ADD,
                    color_write_mask: vk::ColorComponentFlags::RGBA,
                }];
                let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
                    logic_op_enable: vk::FALSE,
                    logic_op: vk::LogicOp::COPY,
                    attachment_count: attachments.len() as u32,
                    p_attachments: attachments.as_ptr(),
                    blend_constants: [0.0, 0.0, 0.0, 0.0],
                    ..Default::default()
                };
                let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                let dynamic_state = vk::PipelineDynamicStateCreateInfo {
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
                let pipeline_create_info = vk::GraphicsPipelineCreateInfo {
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
                    layout: layout.handle,
                    render_pass: vk::RenderPass::null(), // dynamic rendering is enabled
                    subpass: 0,
                    ..Default::default()
                };

                let pipelines = unsafe {
                    device.create_graphics_pipelines(
                        vk::PipelineCache::null(),
                        &[pipeline_create_info],
                    )
                }
                .map_err(|(_, vk_err)| vk_err)
                .inspect_err(|e| trace_error!(e))?;

                Ok(Pipeline {
                    device,
                    layout: layout.clone(),
                    pipeline: pipelines[0],
                })
            }
        }
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
