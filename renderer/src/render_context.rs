use crate::trace_error;

use ash::vk;
use std::rc::Rc;

pub struct RenderContext {
    swapchain: vulkan::swapchain::Swapchain,
    device: Rc<vulkan::device::Device>,
    command_buffer_executed: Box<[vk::Fence]>,
    image_acquired: Box<[vk::Semaphore]>,
    render_complete: Box<[vk::Semaphore]>,
    command_infos: Box<[(vk::CommandPool, vk::CommandBuffer)]>,
    depth_images: Box<[vulkan::image::Image]>,
    pipeline: Rc<vulkan::pipeline::Pipeline>,
    index: usize,
}

impl RenderContext {
    pub fn new(
        device: Rc<vulkan::device::Device>,
        window: &winit::window::Window,
    ) -> vulkan::result::Result<RenderContext> {
        let swapchain = vulkan::swapchain::Swapchain::new(device.clone(), window)
            .inspect_err(|e| trace_error!(e))?;

        let command_buffer_executed = {
            let mut fences: Vec<vk::Fence> = Vec::with_capacity(swapchain.get_image_count());
            for _ in 0..swapchain.get_image_count() {
                let fence_create_info = ash::vk::FenceCreateInfo {
                    flags: vk::FenceCreateFlags::SIGNALED,
                    ..Default::default()
                };
                let fence =
                    unsafe { device.create_fence(&fence_create_info) }.inspect_err(|e| {
                        trace_error!(e);
                        unsafe {
                            for f in fences.iter() {
                                device.destroy_fence(*f);
                            }
                        }
                    })?;
                fences.push(fence);
            }

            fences.into_boxed_slice()
        };

        let (image_acquired, render_complete) = {
            let mut semaphores = Vec::with_capacity(swapchain.get_image_count() * 2);

            for _ in 0..(swapchain.get_image_count() * 2) {
                let semaphore_create_info = vk::SemaphoreCreateInfo {
                    ..Default::default()
                };
                let semaphore = unsafe { device.create_semaphore(&semaphore_create_info) }
                    .inspect_err(|e| {
                        trace_error!(e);
                        unsafe {
                            for s in semaphores.iter() {
                                device.destroy_semaphore(*s);
                            }
                            for fence in command_buffer_executed.iter() {
                                device.destroy_fence(*fence);
                            }
                        }
                    })?;
                semaphores.push(semaphore);
            }

            let completed = semaphores
                .split_off(swapchain.get_image_count())
                .into_boxed_slice();

            (semaphores.into_boxed_slice(), completed)
        };

        let command_infos = {
            let mut infos = Vec::with_capacity(swapchain.get_image_count());

            for _ in 0..swapchain.get_image_count() {
                let pool = {
                    let pool_create_info = vk::CommandPoolCreateInfo {
                        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                        queue_family_index: 0,
                        ..Default::default()
                    };

                    unsafe { device.create_command_pool(&pool_create_info) }.inspect_err(|e| {
                        trace_error!(e);
                        unsafe {
                            for semaphore in image_acquired.iter() {
                                device.destroy_semaphore(*semaphore);
                            }
                            for semaphore in render_complete.iter() {
                                device.destroy_semaphore(*semaphore);
                            }
                            for fence in command_buffer_executed.iter() {
                                device.destroy_fence(*fence);
                            }
                        }
                    })?
                };
                let buffers = {
                    let buffer_allocate_info = ash::vk::CommandBufferAllocateInfo {
                        command_pool: pool,
                        command_buffer_count: 1,
                        level: vk::CommandBufferLevel::PRIMARY,
                        ..Default::default()
                    };

                    unsafe { device.allocate_command_buffers(&buffer_allocate_info) }.inspect_err(
                        |e| {
                            trace_error!(e);
                            unsafe {
                                device.destroy_command_pool(pool);
                                for (pool, buffer) in infos.iter() {
                                    device.free_command_buffers(*pool, &[*buffer]);
                                    device.destroy_command_pool(*pool);
                                }
                                for semaphore in image_acquired.iter() {
                                    device.destroy_semaphore(*semaphore);
                                }
                                for semaphore in render_complete.iter() {
                                    device.destroy_semaphore(*semaphore);
                                }
                                for fence in command_buffer_executed.iter() {
                                    device.destroy_fence(*fence);
                                }
                            }
                        },
                    )?
                };

                infos.push((pool, buffers[0]));
            }

            infos.into_boxed_slice()
        };

        let depth_stencil_format = device
            .find_viable_depth_stencil_format()
            .ok_or(vulkan::result::Error::CouldNotDetermineFormat)
            .inspect_err(|e| trace_error!(e))?;

        let depth_images = {
            let mut images = Vec::with_capacity(swapchain.get_image_count());

            let depth_image_create_info = vulkan::image::ImageCreateInfo {
                mip_levels: 1,
                image_type: vk::ImageType::TYPE_2D,
                format: depth_stencil_format,
                width: swapchain.get_extent().width,
                height: swapchain.get_extent().height,
                depth: 1,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                array_layers: 1,
            };

            for _ in 0..swapchain.get_image_count() {
                let image = vulkan::image::Image::new(device.clone(), &depth_image_create_info)
                    .inspect_err(|e| {
                        trace_error!(e);
                        unsafe {
                            for (pool, buffer) in command_infos.iter() {
                                device.free_command_buffers(*pool, &[*buffer]);
                                device.destroy_command_pool(*pool);
                            }
                            for semaphore in image_acquired.iter() {
                                device.destroy_semaphore(*semaphore);
                            }
                            for semaphore in render_complete.iter() {
                                device.destroy_semaphore(*semaphore);
                            }
                            for fence in command_buffer_executed.iter() {
                                device.destroy_fence(*fence);
                            }
                        }
                    })?;
                images.push(image);
            }

            images.into_boxed_slice()
        };

        let pipeline = {
            let (spv_vertex_shader_module, vk_vertex_shader_module) = unsafe {
                vulkan::pipeline::create_shader_modules(
                    device.clone(),
                    String::from("shaders/compiled/shader.vert.spv"),
                )
            }?;
            let (spv_frag_shader_module, vk_frag_shader_module) = unsafe {
                vulkan::pipeline::create_shader_modules(
                    device.clone(),
                    String::from("shaders/compiled/shader.frag.spv"),
                )
            }
            .inspect_err(|_| {
                unsafe { device.destroy_shader_module(vk_vertex_shader_module) };
            })?;
            let color_formats = Rc::new([swapchain.get_format()]);
            let pipeline_create_info = vulkan::pipeline::PipelineCreateInfo::Graphics {
                vk_vertex_shader_module,
                spv_vertex_shader_module,
                vk_frag_shader_module,
                spv_frag_shader_module,
                color_formats,
                depth_format: depth_stencil_format,
                stencil_format: depth_stencil_format,
            };
            let pipeline = vulkan::pipeline::Pipeline::new(device.clone(), &pipeline_create_info);

            unsafe {
                device.destroy_shader_module(vk_vertex_shader_module);
                device.destroy_shader_module(vk_frag_shader_module);
            }

            Rc::new(pipeline?)
        };
        Ok(RenderContext {
            device,
            swapchain,
            command_buffer_executed,
            image_acquired,
            render_complete,
            command_infos,
            depth_images,
            pipeline,
            index: 0,
        })
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.wait_idle();

            for (pool, buffer) in self.command_infos.iter_mut() {
                self.device.free_command_buffers(*pool, &[*buffer]);
                self.device.destroy_command_pool(*pool);
            }
            for semaphore in self.render_complete.iter_mut() {
                self.device.destroy_semaphore(*semaphore);
            }
            for semaphore in self.image_acquired.iter_mut() {
                self.device.destroy_semaphore(*semaphore);
            }
            for fence in self.command_buffer_executed.iter_mut() {
                self.device.destroy_fence(*fence);
            }
        }
    }
}

impl RenderContext {
    pub unsafe fn draw<F>(&mut self, record_draw_commands: F) -> vulkan::result::Result<()>
    where
        F: FnOnce(vk::CommandBuffer),
    {
        // Acquire image
        let (swapchain_image_index, swapchain_image_view) = {
            unsafe {
                self.device
                    .wait_for_fences(&[self.command_buffer_executed[self.index]])?
            };

            let (image_index, _) = unsafe {
                self.swapchain
                    .acquire_next_image(self.image_acquired[self.index], vk::Fence::null())?
            };

            unsafe {
                self.device
                    .reset_fences(&[self.command_buffer_executed[self.index]])?
            };

            (
                image_index as usize,
                self.swapchain.get_image_view(image_index as usize).unwrap(),
            )
        };

        let (_, command_buffer) = self.command_infos.get(swapchain_image_index).unwrap();

        // Begin command buffer
        let begin_info = vk::CommandBufferBeginInfo {
            flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe {
            // Reset the command buffer (requires pool/reset capability)
            self.device
                .reset_command_buffer(*command_buffer, vk::CommandBufferResetFlags::empty())?;

            self.device
                .begin_command_buffer(*command_buffer, &begin_info)?;
        }

        {
            let color_barrier = ash::vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
                src_access_mask: vk::AccessFlags2::empty(),
                dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                image: *self.swapchain.get_image(swapchain_image_index).unwrap(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let depth_barrier = vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
                src_access_mask: vk::AccessFlags2::empty(),
                dst_stage_mask: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
                dst_access_mask: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                image: self.depth_images.get(swapchain_image_index).unwrap().handle,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let dependency_info = vk::DependencyInfo {
                image_memory_barrier_count: 2,
                p_image_memory_barriers: [color_barrier, depth_barrier].as_ptr(),
                ..Default::default()
            };
            unsafe {
                self.device
                    .cmd_pipeline_barrier2(*command_buffer, &dependency_info)
            };
        }

        // begin dynamic rendering
        {
            let color_attachment_info = vk::RenderingAttachmentInfo {
                image_view: *swapchain_image_view,
                image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                clear_value: vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
                ..Default::default()
            };

            let depth_image = self.depth_images.get(swapchain_image_index).unwrap();
            let depth_attachment_info = ash::vk::RenderingAttachmentInfo {
                image_view: depth_image.view,
                image_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                clear_value: vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
                ..Default::default()
            };

            let rendering_info = ash::vk::RenderingInfo {
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: *self.swapchain.get_extent(),
                },
                layer_count: 1,
                view_mask: 0,
                color_attachment_count: 1,
                p_color_attachments: &color_attachment_info,
                p_depth_attachment: &depth_attachment_info,
                ..Default::default()
            };

            let viewport = ash::vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.get_extent().width as f32,
                height: self.swapchain.get_extent().height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: *self.swapchain.get_extent(),
            };
            unsafe {
                self.device
                    .cmd_begin_rendering(*command_buffer, &rendering_info);

                self.device
                    .cmd_set_viewport(*command_buffer, 0, &[viewport]);
                self.device.cmd_set_scissor(*command_buffer, 0, &[scissor]);

                self.pipeline.bind(*command_buffer);
            };
        }

        record_draw_commands(*command_buffer);

        // End rendering & end command buffer
        unsafe {
            self.device.cmd_end_rendering(*command_buffer);
        }

        // Barrier to transition for pres
        {
            let to_present = ash::vk::ImageMemoryBarrier2 {
                src_stage_mask: ash::vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: ash::vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                dst_stage_mask: ash::vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
                dst_access_mask: ash::vk::AccessFlags2::empty(),
                old_layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                new_layout: ash::vk::ImageLayout::PRESENT_SRC_KHR,
                image: *self.swapchain.get_image(swapchain_image_index).unwrap(),
                subresource_range: ash::vk::ImageSubresourceRange {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let dependency_info = ash::vk::DependencyInfo {
                image_memory_barrier_count: 1,
                p_image_memory_barriers: &to_present,
                ..Default::default()
            };

            unsafe {
                self.device
                    .cmd_pipeline_barrier2(*command_buffer, &dependency_info)
            };
        }

        unsafe {
            self.device
                .end_command_buffer(*command_buffer)
                .inspect_err(|e| trace_error!(e))?;
        }

        // Submit
        {
            let wait_stages = [ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let wait_semaphores = [self.image_acquired[self.index]];
            let signal_semaphores = [self.render_complete[self.index]];
            let command_buffers = [*command_buffer];

            let submit_info = ash::vk::SubmitInfo {
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: wait_semaphores.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: command_buffers.len() as u32,
                p_command_buffers: command_buffers.as_ptr(),
                signal_semaphore_count: signal_semaphores.len() as u32,
                p_signal_semaphores: signal_semaphores.as_ptr(),
                ..Default::default()
            };

            unsafe {
                self.device.queue_submit(
                    &[submit_info],
                    *self.command_buffer_executed.get(self.index).unwrap(),
                )?
            };

            let present_wait_semaphores = [signal_semaphores[0]];
            let present_info = ash::vk::PresentInfoKHR {
                wait_semaphore_count: present_wait_semaphores.len() as u32,
                p_wait_semaphores: present_wait_semaphores.as_ptr(),
                swapchain_count: 1,
                p_swapchains: unsafe { self.swapchain.get_swapchain_ptr() },
                p_image_indices: &(swapchain_image_index as u32),
                ..Default::default()
            };
            unsafe { self.device.queue_present(&present_info)? };
        }

        self.index += 1;
        self.index %= self.swapchain.get_image_count();

        Ok(())
    }
}
