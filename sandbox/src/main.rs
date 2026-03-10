mod camera;
mod constants;
mod result;

use camera::Camera;
use constants::{WORLD_FORWARDS, WORLD_RIGHT, WORLD_UP};
use renderer::ShaderVertVertex;
use result::{Error, Result};

use ash::vk;

use std::collections::HashMap;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowId},
};

use math::Identity;
use math::Vec3;
use math::Vec4;
use math::{Quat, Zero};

#[allow(dead_code)]
struct Application {
    mouse_sensitivity: f64,
    focused_window: Option<WindowId>,
    active_window: Option<WindowId>,
    windows: HashMap<WindowId, (renderer::RenderContext, Window, Camera)>,
    renderer: renderer::Renderer,
    plane_vertex_buffer: Rc<vulkan::VertexBV>,
    plane_index_buffer: Rc<vulkan::IndexBV>,
    model_vertex_buffers: Rc<[vulkan::VertexBV]>,
    model_index_buffers: Rc<[vulkan::IndexBV]>,
    image: Rc<vulkan::Image>,
    model_transform: math::AffineTransform,
    model_base_color: Vec4<f32>,
    model_flags: u32,
    plane_transform: math::AffineTransform,
    plane_base_color: Vec4<f32>,
    plane_flags: u32,
    global_light_direction: Vec3<f32>,
    global_light_color: Vec4<f32>,
    global_ambient_light: f32,
    exiting: bool,
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { *p_callback_data };
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
    };

    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    let message = format!("{message_type:?} [{message_id_name} ({message_id_number})] : {message}");

    if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        tracing::error!(message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        tracing::warn!(message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
        tracing::info!(message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE) {
        tracing::trace!(message);
    }

    vk::FALSE
}

const DEFAULT_IMAGE: &[u8] = include_bytes!("../../files/images/default.png");

impl Application {
    fn new(
        derive_normals: bool,
        obj_to_world: math::Mat3<f32>,
        model_path: &std::path::Path,
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<Self> {
        let instance = vulkan::Instance::new(debug_enabled, display_handle)?;
        let device = vulkan::Device::new(instance, Some(vulkan_debug_callback))?;
        let renderer = renderer::Renderer::new(device)?;

        let (model_vertex_buffers, model_index_buffers, model_transform) = {
            let file_contents = std::fs::read_to_string(model_path)?;
            let scene = wavefront_obj::obj::parse(file_contents)?;

            let mut object_data = Vec::<(Vec<ShaderVertVertex>, Vec<u32>)>::new();

            // problem is in this for loop somewhere
            for obj in scene.objects.into_iter() {
                let mut vertices = Vec::<ShaderVertVertex>::new();
                let mut indices = Vec::<u32>::new();
                let mut vertex_map = HashMap::<(usize, Option<usize>, Option<usize>), u32>::new();

                for geo in obj.geometry.iter() {
                    for shape in geo.shapes.iter() {
                        if let wavefront_obj::obj::Primitive::Triangle(v1, v2, v3) = shape.primitive
                        {
                            let derived_normal = if derive_normals {
                                match (v1.1, v2.1, v3.1) {
                                    (None, None, None) => {
                                        let p1 = obj.vertices[v1.0];
                                        let p1 = Vec3::new(p1.x as f32, p1.y as f32, p1.z as f32);
                                        let p2 = obj.vertices[v2.0];
                                        let p2 = Vec3::new(p2.x as f32, p2.y as f32, p2.z as f32);
                                        let p3 = obj.vertices[v3.0];
                                        let p3 = Vec3::new(p3.x as f32, p3.y as f32, p3.z as f32);

                                        let face_normal = p2.sub(p3).cross(p3.sub(p1));

                                        Some(obj_to_world.mul_vec(face_normal).into_arr())
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            };
                            let derived_normal = match derived_normal {
                                Some(n) => n,
                                None => [0.0, 0.0, 0.0],
                            };

                            for v in [v1, v2, v3] {
                                let key = (v.0, v.1, v.2);

                                let index = if let Some(&i) = vertex_map.get(&key) {
                                    i
                                } else {
                                    let position = obj_to_world
                                        .mul_vec(Vec3::new(
                                            obj.vertices[v.0].x as f32,
                                            obj.vertices[v.0].y as f32,
                                            obj.vertices[v.0].z as f32,
                                        ))
                                        .into_arr();

                                    let tex_coord = if let Some(i) = v.1 {
                                        [obj.tex_vertices[i].u as f32, obj.tex_vertices[i].v as f32]
                                    } else {
                                        [0.0, 0.0]
                                    };

                                    let normal = if let Some(i) = v.2 {
                                        obj_to_world
                                            .mul_vec(Vec3::new(
                                                obj.normals[i].x as f32,
                                                obj.normals[i].y as f32,
                                                obj.normals[i].z as f32,
                                            ))
                                            .into_arr()
                                    } else {
                                        derived_normal
                                    };

                                    let vert = ShaderVertVertex {
                                        position,
                                        tex_coord,
                                        normal,
                                    };

                                    let i = vertices.len() as u32;
                                    vertices.push(vert);
                                    vertex_map.insert(key, i);
                                    i
                                };

                                indices.push(index);
                            }
                        }
                    }
                }

                object_data.push((vertices, indices));
            }

            let model_transform = {
                let mut min = [f32::MAX; 3];
                let mut max = [f32::MIN; 3];
                for (vertices, _) in object_data.iter() {
                    for v in vertices.iter() {
                        for i in 0..3 {
                            min[i] = min[i].min(v.position[i]);
                            max[i] = max[i].max(v.position[i]);
                        }
                    }
                }
                let center = Vec3::new(
                    (min[0] + max[0]) * 0.5,
                    (min[1] + max[1]) * 0.5,
                    (min[2] + max[2]) * 0.5,
                );

                let model_scale = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]);
                let model_scale = 1.0 / model_scale;

                math::AffineTransform {
                    position: Vec3::ZERO.sub(center).add(WORLD_FORWARDS.scaled(1.5)),
                    orientation: Quat::IDENTITY,
                    scalar: Vec3::new(model_scale, model_scale, model_scale),
                }
            };

            let (vertex_buffers, index_buffers) = {
                let mut vbs = Vec::<vulkan::VertexBV>::new();
                let mut ibs = Vec::<vulkan::IndexBV>::new();

                for (vertices, indices) in object_data.iter() {
                    let vb_data = unsafe {
                        std::slice::from_raw_parts(
                            vertices.as_ptr() as *const u8,
                            vertices.len() * std::mem::size_of::<renderer::ShaderVertVertex>(),
                        )
                    };

                    let vb = renderer.create_vertex_buffer(vb_data, vertices.len() as u32)?;

                    let ib_data = unsafe {
                        std::slice::from_raw_parts(
                            indices.as_ptr() as *const u8,
                            indices.len() * std::mem::size_of::<u32>(),
                        )
                    };

                    let ib = renderer.create_index_buffer(
                        ib_data,
                        vk::IndexType::UINT32,
                        indices.len() as u32,
                        0,
                    )?;

                    vbs.push(vb);
                    ibs.push(ib);
                }

                (vbs, ibs)
            };

            (vertex_buffers, index_buffers, model_transform)
        };

        const PLANE_VERTEX_BUFFER_DATA: [renderer::ShaderVertVertex; 4] = {
            const T: Vec3<f32> = WORLD_UP;
            const B: Vec3<f32> = Vec3::ZERO.sub(WORLD_UP);
            const R: Vec3<f32> = WORLD_RIGHT;
            const L: Vec3<f32> = Vec3::ZERO.sub(WORLD_RIGHT);

            const TR: Vec3<f32> = T.add(R);
            const TL: Vec3<f32> = T.add(L);
            const BR: Vec3<f32> = B.add(R);
            const BL: Vec3<f32> = B.add(L);
            [
                renderer::ShaderVertVertex {
                    position: TL.into_arr(),
                    tex_coord: [1.0, 0.0],
                    normal: WORLD_FORWARDS.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: TR.into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: WORLD_FORWARDS.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: BR.into_arr(),
                    tex_coord: [0.0, 1.0],
                    normal: WORLD_FORWARDS.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: BL.into_arr(),
                    tex_coord: [1.0, 1.0],
                    normal: WORLD_FORWARDS.into_arr(),
                },
            ]
        };
        const PLANE_INDEX_BUFFER_DATA: [u32; 6] = [0, 1, 2, 2, 3, 0];

        let plane_vertex_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    PLANE_VERTEX_BUFFER_DATA.as_ptr() as *const u8,
                    PLANE_VERTEX_BUFFER_DATA.len()
                        * std::mem::size_of::<renderer::ShaderVertVertex>(),
                )
            };

            let vb = renderer
                .create_vertex_buffer(data, PLANE_VERTEX_BUFFER_DATA.len() as u32)
                .inspect_err(|e| tracing::error!("{e}"))?;

            Rc::new(vb)
        };
        let plane_index_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    PLANE_INDEX_BUFFER_DATA.as_ptr() as *const u8,
                    PLANE_INDEX_BUFFER_DATA.len() * std::mem::size_of::<u32>(),
                )
            };

            renderer.create_index_buffer(
                data,
                vk::IndexType::UINT32,
                PLANE_INDEX_BUFFER_DATA.len() as u32,
                0,
            ).inspect_err(|e| tracing::error!("{e}"))?
        };

        let plane_index_buffer = Rc::new(plane_index_buffer);

        let image = {
            let image_data =
                image::load_from_memory_with_format(DEFAULT_IMAGE, image::ImageFormat::Png)?;

            renderer.create_image(image_data).inspect_err(|e| tracing::error!("{e}"))?
        };

        let plane_transform = math::AffineTransform {
            position: model_transform.position.add(Vec3::ZERO.sub(WORLD_UP)),
            orientation: math::Quat::unit_from_angle_axis(
                std::f32::consts::FRAC_PI_2,
                WORLD_RIGHT,
            ),
            scalar: Vec3::new(20.0, 20.0, 20.0),
        };

        Ok(Self {
            mouse_sensitivity: 0.001,
            focused_window: None,
            active_window: None,
            renderer,
            windows: std::collections::HashMap::new(),
            plane_vertex_buffer,
            plane_index_buffer,
            model_vertex_buffers: model_vertex_buffers.into(),
            model_index_buffers: model_index_buffers.into(),
            exiting: false,
            image,
            model_transform,
            model_base_color: Vec4::new(1.0, 0.1, 0.4, 1.0),
            model_flags: 0,
            plane_transform,
            plane_base_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            global_light_direction: Vec3::ZERO.sub(WORLD_UP).add(WORLD_RIGHT.scaled(0.2)),
            global_light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            global_ambient_light: 0.1,
            plane_flags: 1,
        })
    }
}

impl Application {
    // returns true if a window close was requested.
    fn handle_window_event(
        &mut self,
        event: winit::event::WindowEvent,
        window_id: &winit::window::WindowId,
    ) -> Result<bool> {
        use winit::event::WindowEvent;

        let (context, window, camera) = self
            .windows
            .get_mut(window_id)
            .ok_or(Error::WindowIdInvalid)?;

        match event {
            WindowEvent::CloseRequested => {
                tracing::debug!("close requested!");
                // unsafe { self.renderer.destroy_render_context(context) };
                return Ok(true);
            }
            WindowEvent::Resized(s) => {
                {
                    let (w, h) = (s.width as f32, s.height as f32);
                    let aspect_ratio = w / h;

                    camera.set_aspect_ratio(aspect_ratio);
                }

                let camera_ubo = renderer::CameraUBO {
                    view: camera.get_view_matrix().into_2d_arr(),
                    proj: camera.get_projection_matrix().into_2d_arr(),
                    ..Default::default()
                };
                let mesh_ubos = [
                    renderer::MeshUBO {
                        model: self.plane_transform.as_mat4().into_2d_arr(),
                        base_color: self.plane_base_color.into_arr(),
                        flags: self.plane_flags,
                    },
                    renderer::MeshUBO {
                        model: self.model_transform.as_mat4().into_2d_arr(),
                        base_color: self.model_base_color.into_arr(),
                        flags: self.model_flags,
                    },
                ];

                let light_ubo = renderer::GlobalLightUBO {
                    direction: self.global_light_direction.into_arr(),
                    color: self.global_light_color.into_arr(),
                    ambient: self.global_ambient_light,
                    ..Default::default()
                };
                let new_context = self.renderer.create_render_context(
                    &camera_ubo,
                    &mesh_ubos,
                    &light_ubo,
                    window,
                    self.image.clone(),
                ).inspect_err(|e| tracing::error!("{e}"))?;

                *context = new_context;
            }
            WindowEvent::RedrawRequested => {
                // println!("Redraw requested!");

                let camera_ubo = [renderer::CameraUBO {
                    view: camera.get_view_matrix().into_2d_arr(),
                    proj: camera.get_projection_matrix().into_2d_arr(),
                    ..Default::default()
                }];
                let camera_ubo_ptr = camera_ubo.as_ptr() as *const u8;
                let current_buffer = context.get_current_per_frame_buffer();
                self.renderer
                    .update_uniform_buffer(
                        camera_ubo_ptr,
                        std::mem::size_of::<renderer::CameraUBO>(),
                        current_buffer,
                    )
                    .inspect_err(|e| tracing::error!("{e}"))?;

                let current_ds = context.get_current_per_frame_descriptor_set();
                let obj_ds = context.get_per_obj_descriptor_set();
                let other_ds = context.get_other_descriptor_set();

                let pipeline = context.get_pipeline();

                let plane_dynamic_offset: [u32; 1] =
                    [context.get_per_obj_dynamic_uniform_buffers()[0].offset as u32];
                let plane_vertex_buffer = self.plane_vertex_buffer.clone();
                let plane_index_buffer = self.plane_index_buffer.clone();

                let model_dynamic_offset: [u32; 1] =
                    [context.get_per_obj_dynamic_uniform_buffers()[1].offset as u32];
                let mesh_ubo = [renderer::MeshUBO {
                    model: self.model_transform.as_mat4().into_2d_arr(),
                    base_color: self.model_base_color.into_arr(),
                    flags: self.model_flags,
                }];
                let mesh_ubo_ptr = mesh_ubo.as_ptr() as *const u8;
                self.renderer
                    .update_dynamic_uniform_buffer(
                        mesh_ubo_ptr,
                        std::mem::size_of::<renderer::MeshUBO>(),
                        &context.get_per_obj_dynamic_uniform_buffers()[1],
                    )
                    .inspect_err(|e| tracing::error!("{e}"))?;
                let model_vertex_buffers = self.model_vertex_buffers.clone();
                let model_index_buffers = self.model_index_buffers.clone();

                let record_draw_commands = |command_buffer: vk::CommandBuffer| unsafe {
                    current_ds.bind(command_buffer, &[]);
                    other_ds.bind(command_buffer, &[]);

                    pipeline.bind(command_buffer);

                    obj_ds.bind(command_buffer, &plane_dynamic_offset);
                    plane_vertex_buffer.bind(command_buffer);
                    plane_index_buffer.bind(command_buffer);
                    plane_index_buffer.draw(command_buffer);

                    obj_ds.bind(command_buffer, &model_dynamic_offset);

                    for (vb, ib) in model_vertex_buffers.iter().zip(model_index_buffers.iter()) {
                        vb.bind(command_buffer);
                        ib.bind(command_buffer);
                        ib.draw(command_buffer);
                    }
                };
                unsafe {
                    context
                        .draw(record_draw_commands)
                        .inspect_err(|e| tracing::error!("{e}"))?;
                }
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::event::KeyEvent;
                use winit::keyboard::KeyCode;

                const ANGLE: f32 = 0.025;
                const SPEED: f32 = 0.025;
                match event {
                    KeyEvent { physical_key, .. } => match physical_key {
                        winit::keyboard::PhysicalKey::Code(c) => match c {
                            KeyCode::Escape => {
                                self.active_window = None;
                                match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                                    Err(e) => {
                                        tracing::error!("{}", e);
                                    }
                                    _ => {}
                                }
                                window.set_cursor_visible(true);
                            }
                            KeyCode::KeyE => {
                                camera.move_local(WORLD_FORWARDS.scaled(SPEED));
                            }
                            KeyCode::KeyD => {
                                camera.move_local(WORLD_FORWARDS.scaled(-SPEED));
                            }
                            KeyCode::KeyF => {
                                camera.move_local(WORLD_RIGHT.scaled(SPEED));
                            }
                            KeyCode::KeyS => {
                                camera.move_local(WORLD_RIGHT.scaled(-SPEED));
                            }
                            KeyCode::Space => {
                                camera.move_global(WORLD_UP.scaled(SPEED));
                            }
                            KeyCode::ControlLeft => {
                                camera.move_global(WORLD_UP.scaled(-SPEED));
                            }
                            KeyCode::ArrowUp => {
                                self.model_transform.rotate_global(
                                    Quat::unit_from_angle_axis(ANGLE, WORLD_RIGHT),
                                    self.model_transform.position,
                                );
                            }
                            KeyCode::ArrowDown => {
                                self.model_transform.rotate_global(
                                    Quat::unit_from_angle_axis(-ANGLE, WORLD_RIGHT),
                                    self.model_transform.position,
                                );
                            }
                            KeyCode::ArrowLeft => {
                                self.model_transform.rotate_global(
                                    Quat::unit_from_angle_axis(-ANGLE, WORLD_UP),
                                    self.model_transform.position,
                                );
                            }
                            KeyCode::ArrowRight => {
                                self.model_transform.rotate_global(
                                    Quat::unit_from_angle_axis(ANGLE, WORLD_UP),
                                    self.model_transform.position,
                                );
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                }
                // println!("Keyboard Input!");
            }
            WindowEvent::Moved(_) => {
                // println!("Moved!");
            }
            WindowEvent::Focused(b) => {
                if b {
                    self.focused_window = Some(*window_id);
                }
                // tracing::trace!("Focused!");
            }
            WindowEvent::MouseInput { button, .. } => {
                use winit::event::MouseButton;

                // tracing::trace!("Mouse Input!");
                match button {
                    MouseButton::Left => {
                        self.active_window = self.focused_window;
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .or_else(|_| {
                                window.set_cursor_grab(winit::window::CursorGrabMode::Confined)
                            }).inspect_err(|e| tracing::error!("{e}"))?;
                        window.set_cursor_visible(false);
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { .. } => {
                // tracing::trace!("Cursor Moved!: {} {}", position.x, position.y);
            }
            WindowEvent::AxisMotion { .. } => {
                // println!("AxisMotion");
            }
            WindowEvent::ActivationTokenDone { .. } => {
                // tracing::info!("Activation Token Done");
            }
            WindowEvent::CursorLeft { .. } => {
                // println!("CursorLeft!");
            }
            WindowEvent::MouseWheel { .. } => {
                // println!("MouseWheel!");
            }
            WindowEvent::Occluded(_) => {
                // println!("Occluded!");
            }
            WindowEvent::DroppedFile(_) => {
                // println!("Dropped file!");
            }
            WindowEvent::HoveredFile(_) => {
                // println!("HoveredFile");
            }
            WindowEvent::Ime(_) => {
                // println!("Ime!");
            }
            WindowEvent::CursorEntered { .. } => {
                // println!("CursorEntered");
            }
            WindowEvent::Destroyed { .. } => {
                // tracing::trace!("Destroyed!");
            }
            WindowEvent::HoveredFileCancelled => {
                // tracing::info!("HoveredFileCancelled");
            }
            WindowEvent::ModifiersChanged(_) => {
                // println!("ModifiersChanged");
            }
            WindowEvent::TouchpadPressure { .. } => {
                // println!("TouchpadPressure");
            }
            WindowEvent::PinchGesture { .. } => {
                // println!("PinchGesture");
            }
            WindowEvent::DoubleTapGesture { .. } => {
                // println!("DoubleTapGesture");
            }
            WindowEvent::PanGesture { .. } => {
                // println!("PanGesture");
            }
            WindowEvent::RotationGesture { .. } => {
                // println!("RotationGesture");
            }
            WindowEvent::Touch(_) => {
                // println!("Touch");
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                // println!("ScaleFactorChanged");
            }
            WindowEvent::ThemeChanged(_) => {
                // println!("ThemeChanged");
            }
        }

        Ok(false)
    }
}
impl ApplicationHandler for Application {
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if self.exiting {
            return;
        }

        // tracing::trace!("Exiting!");
        self.exiting = true;

        return event_loop.exit();
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // tracing::trace!("Resumed!");

        if !self.windows.is_empty() {
            return;
        }

        let window_attributes =
            winit::window::WindowAttributes::default().with_title("dlk-objviewer");
        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("{}", e);
                return self.exiting(event_loop);
            }
        };
        let mut camera = {
            let s = window.inner_size();
            let (w, h) = (s.width as f32, s.height as f32);
            let aspect_ratio = w / h;

            Camera::new(
                80.0,
                aspect_ratio,
                self.model_transform
                    .position
                    .add(Vec3::ZERO.sub(WORLD_FORWARDS).add(WORLD_UP.scaled(0.5))),
                0.0,
                0.0,
            )
        };
        camera.look_at(self.model_transform.position);
        let window_id = window.id();

        let camera_ubo = renderer::CameraUBO {
            view: camera.get_view_matrix().into_2d_arr(),
            proj: camera.get_projection_matrix().into_2d_arr(),
            ..Default::default()
        };
        let mesh_ubos = [
            renderer::MeshUBO {
                model: self.plane_transform.as_mat4().into_2d_arr(),
                base_color: self.plane_base_color.into_arr(),
                flags: self.plane_flags,
            },
            renderer::MeshUBO {
                model: self.model_transform.as_mat4().into_2d_arr(),
                base_color: self.model_base_color.into_arr(),
                flags: self.model_flags,
            },
        ];
        let light_ubo = renderer::GlobalLightUBO {
            direction: self.global_light_direction.into_arr(),
            color: self.global_light_color.into_arr(),
            ambient: self.global_ambient_light,
            ..Default::default()
        };
        let context = match self.renderer.create_render_context(
            &camera_ubo,
            &mesh_ubos,
            &light_ubo,
            &window,
            self.image.clone(),
        ) {
            Ok(context) => context,
            Err(e) => {
                tracing::error!("{}", e);
                return self.exiting(event_loop);
            }
        };
        self.windows.insert(window_id, (context, window, camera));
    }

    #[allow(unused_variables)]
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        use winit::event::DeviceEvent;

        let (_, _, camera) = match self.active_window {
            Some(id) => match self.windows.get_mut(&id) {
                Some(x) => x,
                None => {
                    return;
                }
            },
            None => {
                return;
            }
        };
        match event {
            DeviceEvent::MouseMotion { delta } => {
                let dx = delta.0 * self.mouse_sensitivity;
                let dy = -delta.1 * self.mouse_sensitivity;

                camera.rotate(dx as f32, dy as f32);
            }
            _ => {
                // tracing::info!("Not implemented")
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self.exiting {
            return;
        }

        match self.handle_window_event(event, &window_id) {
            Ok(b) => {
                if b {
                    self.exiting(event_loop);
                }
            }
            Err(e) => {
                tracing::error!("{}", e);
                self.exiting(event_loop);
            }
        }
    }
}

fn main() -> Result<()> {
    // let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        //     .with_writer(non_blocking)
        .with_max_level(tracing::Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .init();

    let args: Box<[String]> = std::env::args().collect();

    let print_usage = || -> Result<()> {
        let name = format!("{}", std::env::current_exe()?.file_name().unwrap().display());
        println!("Invalid program arguments. Usage: {} <model> <options>", name);
        println!("To view all options type {} --help", name);
        return Ok(());
    };

    if args.len() < 2 {
        return print_usage();
    }

    if args[1] == "--help" {
        println!("Options:");
        println!("    -f Specifies the forwards direction of the model. Defaults to +z.");
        println!("        may be one of: <+x|-x|+y|-y|+z|-z>.");
        println!("    -r Specifies the right direction of the model. Defaults to +x.");
        println!("        may be one of: <+x|-x|+y|-y|+z|-z>");
        println!("    -u Specifies the up direction of the model. Defaults to +y.");
        println!("        may be one of: <+x|-x|+y|-y|+z|-z>");
        println!("    --derive-normals normals to be derived when missing. Defaults to true.");
        println!("        may be one of of: <true|false>");

        return Ok(())
    }

    let model_path = {
        let args: Vec<String> = std::env::args().collect();
        std::path::PathBuf::from(args[1].clone())
    };

    let derive_normals = {
        let idx = args.iter().enumerate().find_map(|(i, s)| {
            if s == "--derive-normals" {
                Some(i)
            } else {
                None
            }
        });
        if let Some(i) = idx {
            match args.get(i + 1).map(|x| x.as_str()) {
                Some("true") => true,
                Some("false") => false,
                _ => {
                    return print_usage();
                }
            }
        } else {
            true
        }
    };

    let (obj_r, obj_u, obj_f) = {
        let ri = args
            .iter()
            .enumerate()
            .find_map(|(i, s)| if s == "-r" { Some(i) } else { None });
        let ui = args
            .iter()
            .enumerate()
            .find_map(|(i, s)| if s == "-u" { Some(i) } else { None });
        let fi = args
            .iter()
            .enumerate()
            .find_map(|(i, s)| if s == "-f" { Some(i) } else { None });

        let rs = if let Some(i) = ri {
            args.get(i + 1).map(|x| x.as_str())
        } else {
            Some("+x")
        };
        let us = if let Some(i) = ui {
            args.get(i + 1).map(|x| x.as_str())
        } else {
            Some("+y")
        };
        let fs = if let Some(i) = fi {
            args.get(i + 1).map(|x| x.as_str())
        } else {
            Some("+z")
        };

        let str_to_vec = |s: Option<&str>| -> Option<math::Vec3<f32>> {
            match s {
                Some("+x") => Some(Vec3::new(1.0, 0.0, 0.0)),
                Some("-x") => Some(Vec3::new(-1.0, 0.0, 0.0)),
                Some("+y") => Some(Vec3::new(0.0, 1.0, 0.0)),
                Some("-y") => Some(Vec3::new(0.0, -1.0, 0.0)),
                Some("+z") => Some(Vec3::new(0.0, 0.0, 1.0)),
                Some("-z") => Some(Vec3::new(0.0, 0.0, -1.0)),
                _ => None,
            }
        };

        match (str_to_vec(rs), str_to_vec(us), str_to_vec(fs)) {
            (Some(rv), Some(uv), Some(fv)) => (rv, uv, fv),
            _ => {
                return print_usage();
            }
        }
    };

    if obj_r.cross(obj_u) != obj_f && obj_u.cross(obj_r) != obj_f {
        println!("Invalid input. Right, left, and up must form a valid coordinate system.");
        return Ok(());
    }

    let obj_to_world = math::Mat3::<f32>::from_cols(
        math::Vec3::new(
            obj_r.dot(WORLD_RIGHT),
            obj_r.dot(WORLD_UP),
            obj_r.dot(WORLD_FORWARDS),
        ),
        math::Vec3::new(
            obj_u.dot(WORLD_RIGHT),
            obj_u.dot(WORLD_UP),
            obj_u.dot(WORLD_FORWARDS),
        ),
        math::Vec3::new(
            obj_f.dot(WORLD_RIGHT),
            obj_f.dot(WORLD_UP),
            obj_f.dot(WORLD_FORWARDS),
        ),
    );

    let event_loop = EventLoop::new().inspect_err(|e| tracing::error!("{e}"))?;

    let mut app = {
        let debug_enabled = cfg!(debug_assertions);
        let owned_display_handle = event_loop.owned_display_handle();
        let display_handle = owned_display_handle.display_handle()?;
        Application::new(
            derive_normals,
            obj_to_world,
            model_path.as_path(),
            debug_enabled,
            &display_handle,
        )?
    };

    event_loop
        .run_app(&mut app)
        .inspect_err(|e| tracing::error!("{e}"))?;

    Ok(())
}
