pub mod render_context;
pub mod result;

use ash::vk;
use std::rc::Rc;

pub struct Renderer {
    device: Rc<vulkan::device::Device>,
    command_pool: vk::CommandPool,
    sampler: vk::Sampler,
}

impl Renderer {
    pub fn new(device: Rc<vulkan::device::Device>) -> result::Result<Renderer> {
        let command_pool = {
            let command_pool_create_info = vk::CommandPoolCreateInfo {
                queue_family_index: device.get_queue_family_index(),
                ..Default::default()
            };

            unsafe { device.create_command_pool(&command_pool_create_info) }?
        };

        let sampler = {
            let properties = unsafe { device.get_physical_device_properties() };
            let sampler_create_info = vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                address_mode_u: vk::SamplerAddressMode::REPEAT,
                address_mode_v: vk::SamplerAddressMode::REPEAT,
                address_mode_w: vk::SamplerAddressMode::REPEAT,
                mip_lod_bias: 0.0,
                anisotropy_enable: vk::TRUE,
                max_anisotropy: properties.limits.max_sampler_anisotropy,
                compare_enable: vk::FALSE,
                compare_op: vk::CompareOp::ALWAYS,
                ..Default::default()
            };

            unsafe {
                device
                    .create_sampler(&sampler_create_info)
                    .inspect_err(|_| {
                        device.destroy_command_pool(command_pool);
                    })?
            }
        };

        Ok(Renderer {
            device,
            command_pool,
            sampler,
        })
    }
    fn get_transfer_buffer(&self, size: u64) -> result::Result<vulkan::buffer::Buffer> {
        let transfer_buffer_create_info = vulkan::buffer::BufferCreateInfo {
            size: size,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT,
        };

        let transfer_buffer =
            vulkan::buffer::Buffer::new(self.device.clone(), &transfer_buffer_create_info)?;

        Ok(transfer_buffer)
    }
    #[inline]
    pub fn create_render_context(
        &self,
        camera: &crate::render_context::CameraUBO,
        window: &winit::window::Window,
        image: Rc<vulkan::image::Image>,
    ) -> result::Result<render_context::RenderContext> {
        let vert_shader_path = std::path::Path::new("compiled-shaders").join("shader.vert.spv");
        let frag_shader_path = std::path::Path::new("compiled-shaders").join("shader.frag.spv");

        let vert_spv_module = Rc::new(spirv::ShaderModule::from_file(&vert_shader_path)?);
        let frag_spv_module = Rc::new(spirv::ShaderModule::from_file(&frag_shader_path)?);

        let pipeline_layout = Rc::new(vulkan::pipeline::PipelineLayout::new_graphics(
            self.device.clone(),
            &vert_spv_module,
            &frag_spv_module,
        )?);

        let per_frame_descriptor_set = 0;

        let (per_frame_descriptor_sets, other_descriptor_sets) = {
            let all_layouts = pipeline_layout.get_set_layouts();

            let descriptor_pool = {
                let mut pool_sizes = std::collections::HashMap::<vk::DescriptorType, u32>::new();
                for l in all_layouts.iter() {
                    let count = if l.set == per_frame_descriptor_set {
                        3
                    } else {
                        1
                    };
                    for b in l.bindings.iter() {
                        if let Some(c) = pool_sizes.get_mut(&b.descriptor_type) {
                            *c += count * b.descriptor_count;
                        } else {
                            pool_sizes.insert(b.descriptor_type, count * b.descriptor_count);
                        }
                    }
                }
                let pool_sizes: Box<[vk::DescriptorPoolSize]> = pool_sizes
                    .iter()
                    .map(|(k, v)| vk::DescriptorPoolSize {
                        ty: *k,
                        descriptor_count: *v,
                    })
                    .collect();

                let pool_create_info = vk::DescriptorPoolCreateInfo {
                    max_sets: (all_layouts.len() + 3) as u32,
                    pool_size_count: pool_sizes.len() as u32,
                    p_pool_sizes: pool_sizes.as_ptr(),
                    ..Default::default()
                };

                Rc::new(vulkan::descriptor::DescriptorPool::new(
                    self.device.clone(),
                    &pool_create_info,
                )?)
            };

            let per_frame_descriptor_sets = {
                let layouts: Box<[vk::DescriptorSetLayout]> =
                    (0..3).map(|_| all_layouts[0].handle.clone()).collect();
                vulkan::descriptor::DescriptorSet::allocate(
                    self.device.clone(),
                    descriptor_pool.clone(),
                    &layouts,
                )?
            };
            let other_descriptor_sets = {
                let layouts: Box<[vk::DescriptorSetLayout]> =
                    all_layouts.into_iter().skip(1).map(|l| l.handle).collect();
                vulkan::descriptor::DescriptorSet::allocate(
                    self.device.clone(),
                    descriptor_pool.clone(),
                    &layouts,
                )?
            };

            (per_frame_descriptor_sets, other_descriptor_sets)
        };

        // 3 == maximum number of frames?
        let per_frame_uniform_buffers = self.create_per_frame_unifrom_buffers(
            std::mem::size_of::<render_context::CameraUBO>() as u64,
            per_frame_descriptor_sets.len() as u64,
        )?;
        for bv in per_frame_uniform_buffers.iter() {
            match bv {
                vulkan::buffer::BufferView::Uniform {
                    buffer,
                    offset,
                    size,
                } => unsafe {
                    let dst = buffer.map_memory(*offset, *size)?;
                    let data = camera;

                    std::ptr::copy_nonoverlapping(data, dst as *mut render_context::CameraUBO, 1);

                    buffer.unmap();
                },
                _ => {
                    panic!("This is bad code");
                }
            }
        }

        {
            let image_info = vk::DescriptorImageInfo {
                sampler: self.sampler,
                image_view: image.view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // TODO: should be stored in the image class?
            };
            let mut buffer_infos = Vec::new();
            for bv in per_frame_uniform_buffers.iter() {
                match bv {
                    vulkan::buffer::BufferView::Uniform {
                        buffer,
                        offset,
                        size,
                    } => {
                        let info = vk::DescriptorBufferInfo {
                            buffer: buffer.handle,
                            offset: *offset,
                            range: *size,
                        };
                        buffer_infos.push(info);
                    }
                    _ => return Err(result::Error::NotAdded),
                }
            }

            let mut descriptor_writes = Vec::new();
            for (bi, ds) in buffer_infos.iter().zip(per_frame_descriptor_sets.iter()) {
                descriptor_writes.push(vk::WriteDescriptorSet {
                    dst_set: ds.handle,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: bi,
                    ..Default::default()
                });
            }

            descriptor_writes.push(vk::WriteDescriptorSet {
                dst_set: other_descriptor_sets[0].handle,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &image_info,
                ..Default::default()
            });
            unsafe { self.device.update_descriptor_sets(&descriptor_writes, &[]) };
        }

        render_context::RenderContext::new(
            self.device.clone(),
            window,
            &vert_shader_path,
            &frag_shader_path,
            pipeline_layout,
            per_frame_descriptor_sets,
            per_frame_uniform_buffers,
            other_descriptor_sets,
            image,
        )
    }
    fn get_command_buffer(&self) -> result::Result<vk::CommandBuffer> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: self.command_pool,
            command_buffer_count: 1,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };

        let command_buffers = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)
        }?;

        Ok(*command_buffers.get(0).unwrap())
    }
    pub fn create_vertex_buffer(
        &self,
        data: &[u8],
        vertex_count: u32,
        first_vertex: u32,
    ) -> vulkan::result::Result<Rc<vulkan::buffer::BufferView>> {
        let buffer = {
            let buffer_create_info = vulkan::buffer::BufferCreateInfo {
                size: data.len() as u64,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                memory_property_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::buffer::Buffer::new(self.device.clone(), &buffer_create_info)?
        };

        let buffer = Rc::new(buffer);

        unsafe {
            let dst = buffer.map_memory(buffer.offset, buffer.size)?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

            buffer.unmap();
        }

        let view = vulkan::buffer::BufferView::Vertex {
            buffer,
            vertex_count,
            instance_count: 1,
            first_vertex,
            first_instance: 0,
        };

        Ok(Rc::new(view))
    }
    pub fn create_index_buffer(
        &self,
        data: &[u8],
        index_type: vk::IndexType,
        index_count: u32,
        first_index: u32,
    ) -> result::Result<Rc<vulkan::buffer::BufferView>> {
        let buffer = {
            let buffer_create_info = vulkan::buffer::BufferCreateInfo {
                size: data.len() as u64,
                usage: vk::BufferUsageFlags::INDEX_BUFFER,
                memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::buffer::Buffer::new(self.device.clone(), &buffer_create_info)?
        };

        let buffer = Rc::new(buffer);

        unsafe {
            let dst = buffer.map_memory(buffer.offset, buffer.size)?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

            buffer.unmap();
        }

        let view = vulkan::buffer::BufferView::Index {
            buffer,
            index_count,
            instance_count: 1,
            first_index,
            index_type,
        };

        Ok(Rc::new(view))
    }
    pub fn create_per_frame_unifrom_buffers(
        &self,
        size: u64,
        count: u64,
    ) -> result::Result<Box<[vulkan::buffer::BufferView]>> {
        let buffer = {
            let buffer_create_info = vulkan::buffer::BufferCreateInfo {
                size: size * count,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::buffer::Buffer::new(self.device.clone(), &buffer_create_info)?
        };

        let buffer = Rc::new(buffer);

        let views: Box<[vulkan::buffer::BufferView]> = (0..count)
            .map(|i| vulkan::buffer::BufferView::Uniform {
                buffer: buffer.clone(),
                offset: i * size,
                size: size,
            })
            .collect();

        Ok(views)
    }
    pub fn update_uniform_buffer(
        &self,
        data: &[u8],
        uniform_buffer: &vulkan::buffer::BufferView,
    ) -> result::Result<()> {
        match uniform_buffer {
            vulkan::buffer::BufferView::Uniform {
                buffer,
                offset,
                size,
            } => unsafe {
                let dst = buffer.map_memory(*offset, *size)?;

                std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

                Ok(buffer.unmap())
            },
            _ => Err(result::Error::ExpectedUniformBufferView),
        }
    }
    pub fn create_image(
        &self,
        image_data: image::DynamicImage,
    ) -> result::Result<Rc<vulkan::image::Image>> {
        use image::GenericImageView;

        let (width, height) = image_data.dimensions();
        let rgba = image_data.into_rgba8();
        let data = rgba.as_raw();
        let size = data.len() as u64;

        let image = {
            let image_create_info = vulkan::image::ImageCreateInfo {
                memory_property_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                mip_levels: 1,
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::R8G8B8A8_SRGB,
                width,
                height,
                depth: 1,
                usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                array_layers: 1,
            };

            vulkan::image::Image::new(self.device.clone(), &image_create_info)?
        };

        let transfer_buffer = self.get_transfer_buffer(size)?;

        unsafe {
            let dst = transfer_buffer.map_memory(0, size)?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, size as usize);

            transfer_buffer.unmap();
        }

        let command_buffer = self.get_command_buffer()?;

        {
            let begin_info = vk::CommandBufferBeginInfo {
                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };

            unsafe {
                self.device
                    .begin_command_buffer(command_buffer, &begin_info)
            }?;
        }

        // transfer commands here
        {
            // to I need the stage mask here?
            let barriers = [vk::ImageMemoryBarrier2 {
                image: image.handle,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                ..Default::default()
            }];

            let dependency_info = vk::DependencyInfo {
                image_memory_barrier_count: barriers.len() as u32,
                p_image_memory_barriers: barriers.as_ptr(),
                ..Default::default()
            };

            unsafe {
                self.device
                    .cmd_pipeline_barrier2(command_buffer, &dependency_info)
            };

            let regions = [vk::BufferImageCopy2 {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                image_extent: vk::Extent3D {
                    width: image.width,
                    height: image.height,
                    depth: image.depth,
                },
                ..Default::default()
            }];

            let copy_buffer_to_image_info = vk::CopyBufferToImageInfo2 {
                src_buffer: transfer_buffer.handle,
                dst_image: image.handle,
                dst_image_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                region_count: regions.len() as u32,
                p_regions: regions.as_ptr(),
                ..Default::default()
            };

            unsafe {
                self.device
                    .cmd_copy_buffer_to_image2(command_buffer, &copy_buffer_to_image_info)
            };

            let barriers = [vk::ImageMemoryBarrier2 {
                image: image.handle,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                ..Default::default()
            }];

            let dependency_info = vk::DependencyInfo {
                image_memory_barrier_count: barriers.len() as u32,
                p_image_memory_barriers: barriers.as_ptr(),
                ..Default::default()
            };

            unsafe {
                self.device
                    .cmd_pipeline_barrier2(command_buffer, &dependency_info)
            };
        }

        unsafe {
            self.device.end_command_buffer(command_buffer)?;

            let submit_info = [vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &command_buffer,
                ..Default::default()
            }];

            self.device.queue_submit(&submit_info, vk::Fence::null())?;
            self.device.wait_idle()?;
            self.device
                .free_command_buffers(self.command_pool, &[command_buffer]);
        }

        Ok(Rc::new(image))
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.wait_idle();
            self.device.destroy_command_pool(self.command_pool);
            self.device.destroy_sampler(self.sampler);
        }
    }
}
