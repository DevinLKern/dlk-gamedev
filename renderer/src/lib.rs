mod render_context;
mod result;

include!(concat!(env!("OUT_DIR"), "/variable_types.rs"));
include!(concat!(env!("OUT_DIR"), "/shader_paths.rs"));
include!(concat!(env!("OUT_DIR"), "/entry_points.rs"));

pub use render_context::RenderContext;
pub use result::Error;
pub use result::Result;

use ash::vk;
use std::rc::Rc;
use vulkan::device::SharedDeviceRef;

pub struct Renderer {
    device: SharedDeviceRef,
    command_pool: vk::CommandPool,
    sampler: vk::Sampler,
}

impl Renderer {
    pub fn new(device: SharedDeviceRef) -> result::Result<Renderer> {
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
    fn get_transfer_buffer(&self, size: u64) -> result::Result<vulkan::Buffer> {
        let transfer_buffer_create_info = vulkan::BufferCreateInfo {
            size: size,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT,
        };

        let transfer_buffer =
            vulkan::Buffer::new(self.device.clone(), &transfer_buffer_create_info)?;

        Ok(transfer_buffer)
    }
    #[inline]
    pub fn create_render_context(
        &self,
        camera: &crate::CameraUBO,
        objects: &[crate::MeshUBO],
        light: &crate::GlobalLightUBO,
        window: &winit::window::Window,
        image: Rc<vulkan::Image>,
    ) -> result::Result<RenderContext> {
        let vert_shader_path = std::path::Path::new(&crate::SHADER_VERT);
        let frag_shader_path = std::path::Path::new(&crate::SHADER_FRAG);

        let vert_spv_module = Rc::new(spirv::Module::from_file(&vert_shader_path)?);
        let frag_spv_module = Rc::new(spirv::Module::from_file(&frag_shader_path)?);

        let pipeline_layout = Rc::new(vulkan::PipelineLayout::new(
            self.device.clone(),
            &vert_spv_module,
            &frag_spv_module,
        )?);

        let per_frame_ds_index = 0;
        let per_obj_ds_index = 1;
        let other_ds_index = 2;

        let (per_frame_descriptor_sets, per_obj_descriptor_set, other_descriptor_set) = {
            let all_layouts = pipeline_layout.get_set_layouts();

            let descriptor_pool = {
                let mut pool_sizes = std::collections::HashMap::<vk::DescriptorType, u32>::new();
                for l in all_layouts.iter() {
                    let count = if l.set == per_frame_ds_index { 3 } else { 1 };
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

                Rc::new(vulkan::DescriptorPool::new(
                    self.device.clone(),
                    &pool_create_info,
                )?)
            };

            let per_frame_descriptor_sets = {
                let mut sets = Vec::new();
                for _ in (0..3).into_iter() {
                    let s = vulkan::DescriptorSet::allocate(
                        self.device.clone(),
                        descriptor_pool.clone(),
                        per_frame_ds_index,
                        pipeline_layout.clone(),
                    )?;
                    sets.push(s);
                }
                sets.into_boxed_slice()
            };
            let per_obj_descriptor_set = {
                vulkan::DescriptorSet::allocate(
                    self.device.clone(),
                    descriptor_pool.clone(),
                    per_obj_ds_index,
                    pipeline_layout.clone(),
                )?
            };
            let other_descriptor_set = {
                vulkan::DescriptorSet::allocate(
                    self.device.clone(),
                    descriptor_pool.clone(),
                    other_ds_index,
                    pipeline_layout.clone(),
                )?
            };

            (
                per_frame_descriptor_sets,
                per_obj_descriptor_set,
                other_descriptor_set,
            )
        };

        let per_frame_uniform_buffer_size = {
            let obj1 = std::mem::size_of::<crate::CameraUBO>() as u64;

            let alignment = unsafe {
                let properties = self.device.get_physical_device_properties();
                properties.limits.min_uniform_buffer_offset_alignment
            };

            obj1.next_multiple_of(alignment)
        };
        let per_obj_uniform_buffer_size = {
            let obj1 = std::mem::size_of::<crate::MeshUBO>() as u64;

            let alignment = unsafe {
                let properties = self.device.get_physical_device_properties();
                properties.limits.min_uniform_buffer_offset_alignment
            };

            obj1.next_multiple_of(alignment)
        };
        let other_uniform_buffer_size = {
            let obj1 = std::mem::size_of::<crate::GlobalLightUBO>() as u64;

            let alignment = unsafe {
                let properties = self.device.get_physical_device_properties();
                properties.limits.min_uniform_buffer_offset_alignment
            };

            obj1.next_multiple_of(alignment)
        };

        // 3 == maximum number of frames?
        let per_frame_uniform_buffers = self.create_uniform_buffers(
            per_frame_uniform_buffer_size,
            per_frame_descriptor_sets.len() as u64,
        )?;
        for bv in per_frame_uniform_buffers.iter() {
            unsafe {
                let dst = bv.buffer.map_memory(bv.offset, bv.size)?;
                let data = camera;

                std::ptr::copy_nonoverlapping(data, dst as *mut crate::CameraUBO, 1);

                bv.buffer.unmap();
            }
        }

        let per_obj_uniform_buffers =
            self.create_dynamic_uniform_buffers(per_obj_uniform_buffer_size, objects.len() as u64)?;
        for (bv, data) in per_obj_uniform_buffers.iter().zip(objects.iter()) {
            unsafe {
                let dst = bv.buffer.map_memory(bv.offset, bv.size)?;

                std::ptr::copy_nonoverlapping(data, dst as *mut crate::MeshUBO, 1);

                bv.buffer.unmap();
            }
        }

        let other_uniform_buffer = self.create_uniform_buffers(other_uniform_buffer_size, 1)?;
        let other_uniform_buffer = other_uniform_buffer.into_iter().next().unwrap();
        unsafe {
            let dst = other_uniform_buffer
                .buffer
                .map_memory(other_uniform_buffer.offset, other_uniform_buffer.size)?;
            let data = light;

            std::ptr::copy_nonoverlapping(data, dst as *mut crate::GlobalLightUBO, 1);

            other_uniform_buffer.buffer.unmap();
        }

        {
            let mut per_frame_buffer_infos = Vec::new();
            for bv in per_frame_uniform_buffers.iter() {
                let info = vk::DescriptorBufferInfo {
                    buffer: bv.buffer.handle,
                    offset: bv.offset,
                    range: bv.size,
                };
                per_frame_buffer_infos.push(info);
            }

            let mut descriptor_writes = Vec::new();
            for (bi, ds) in per_frame_buffer_infos
                .iter()
                .zip(per_frame_descriptor_sets.iter())
            {
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

            let mut per_obj_buffer_infos = Vec::new();
            for bv in per_obj_uniform_buffers.iter() {
                let info = vk::DescriptorBufferInfo {
                    buffer: bv.buffer.handle,
                    offset: 0,
                    range: bv.size,
                };
                per_obj_buffer_infos.push(info);
            }
            for bi in per_obj_buffer_infos.iter() {
                descriptor_writes.push(vk::WriteDescriptorSet {
                    dst_set: per_obj_descriptor_set.handle,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                    p_buffer_info: bi,
                    ..Default::default()
                });
            }

            let other_bi = vk::DescriptorBufferInfo {
                buffer: other_uniform_buffer.buffer.handle,
                offset: other_uniform_buffer.offset,
                range: other_uniform_buffer.size,
            };
            let image_info = vk::DescriptorImageInfo {
                sampler: self.sampler,
                image_view: image.view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // TODO: should be stored in the image class?
            };
            descriptor_writes.push(vk::WriteDescriptorSet {
                dst_set: other_descriptor_set.handle,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &other_bi,
                ..Default::default()
            });
            descriptor_writes.push(vk::WriteDescriptorSet {
                dst_set: other_descriptor_set.handle,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &image_info,
                ..Default::default()
            });
            unsafe { self.device.update_descriptor_sets(&descriptor_writes, &[]) };
        }

        let per_obj_descriptor_set = Rc::new(per_obj_descriptor_set);
        let other_descriptor_sets = Box::new([other_descriptor_set]);
        let other_uniform_buffer = Rc::new(other_uniform_buffer);

        crate::RenderContext::new(
            self.device.clone(),
            window,
            &vert_shader_path,
            &frag_shader_path,
            pipeline_layout,
            per_frame_descriptor_sets,
            per_obj_descriptor_set,
            other_descriptor_sets,
            per_frame_uniform_buffers,
            per_obj_uniform_buffers,
            other_uniform_buffer,
            image,
        )
    }
    fn get_command_buffer(&self) -> Result<vk::CommandBuffer> {
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
    ) -> vulkan::Result<Rc<vulkan::VertexBV>> {
        let buffer = {
            let buffer_create_info = vulkan::BufferCreateInfo {
                size: data.len() as u64,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                memory_property_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::Buffer::new(self.device.clone(), &buffer_create_info)?
        };

        let buffer = Rc::new(buffer);

        unsafe {
            let dst = buffer.map_memory(buffer.offset, buffer.size)?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

            buffer.unmap();
        }

        let view = vulkan::VertexBV {
            buffer,
            vertex_count,
            instance_count: 1,
            first_binding: 0,
            offset: 0,
        };

        Ok(Rc::new(view))
    }
    pub fn create_index_buffer(
        &self,
        data: &[u8],
        index_type: vk::IndexType,
        index_count: u32,
        first_index: u32,
    ) -> result::Result<Rc<vulkan::IndexBV>> {
        let buffer = {
            let buffer_create_info = vulkan::buffer::BufferCreateInfo {
                size: data.len() as u64,
                usage: vk::BufferUsageFlags::INDEX_BUFFER,
                memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::Buffer::new(self.device.clone(), &buffer_create_info)?
        };

        let buffer = Rc::new(buffer);

        unsafe {
            let dst = buffer.map_memory(buffer.offset, buffer.size)?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

            buffer.unmap();
        }

        let view = vulkan::IndexBV {
            buffer,
            offset: 0,
            index_count,
            instance_count: 1,
            first_index,
            vertex_offset: 0,
            first_instance: 0,
            index_type,
        };

        Ok(Rc::new(view))
    }
    pub fn create_uniform_buffers(
        &self,
        size: u64,
        count: u64,
    ) -> result::Result<Box<[vulkan::UniformBV]>> {
        let buffer = {
            let buffer_create_info = vulkan::BufferCreateInfo {
                size: size * count,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::Buffer::new(self.device.clone(), &buffer_create_info)?
        };

        let buffer = Rc::new(buffer);

        let views: Box<[vulkan::UniformBV]> = (0..count)
            .map(|i| vulkan::UniformBV {
                buffer: buffer.clone(),
                offset: i * size,
                size: size,
            })
            .collect();

        Ok(views)
    }
    pub fn update_uniform_buffer(
        &self,
        data: *const u8,
        byte_count: usize,
        uniform_bv: &vulkan::UniformBV,
    ) -> result::Result<()> {
        unsafe {
            let dst = uniform_bv
                .buffer
                .map_memory(uniform_bv.offset, uniform_bv.size)?;

            std::ptr::copy_nonoverlapping(data, dst as *mut u8, byte_count);

            Ok(uniform_bv.buffer.unmap())
        }
    }
    pub fn create_dynamic_uniform_buffers(
        &self,
        size: u64,
        count: u64,
    ) -> Result<Box<[vulkan::DynamicUniformBV]>> {
        let aligned_size = {
            let props = unsafe { self.device.get_physical_device_properties() };
            size.next_multiple_of(props.limits.min_uniform_buffer_offset_alignment)
        };

        let buffer = {
            let create_info = vulkan::BufferCreateInfo {
                size: aligned_size * count,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                memory_property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
            };

            vulkan::Buffer::new(self.device.clone(), &create_info)?
        };

        let buffer = Rc::new(buffer);

        let views: Box<[vulkan::DynamicUniformBV]> = (0..count)
            .map(|i| vulkan::DynamicUniformBV {
                buffer: buffer.clone(),
                offset: aligned_size * i,
                size: size,
            })
            .collect();

        Ok(views)
    }
    pub fn create_image(
        &self,
        image_data: image::DynamicImage,
    ) -> result::Result<Rc<vulkan::Image>> {
        use image::GenericImageView;

        let (width, height) = image_data.dimensions();
        let rgba = image_data.into_rgba8();
        let data = rgba.as_raw();
        let size = data.len() as u64;

        let image = {
            let image_create_info = vulkan::ImageCreateInfo {
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

            vulkan::Image::new(self.device.clone(), &image_create_info)?
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

            self.device
                .queue_submit(self.device.queue, &submit_info, vk::Fence::null())?;
            self.device.device_wait_idle()?;
            self.device
                .free_command_buffers(self.command_pool, &[command_buffer]);
        }

        Ok(Rc::new(image))
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_command_pool(self.command_pool);
            self.device.destroy_sampler(self.sampler);
        }
    }
}
