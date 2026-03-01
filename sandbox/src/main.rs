mod camera;
mod constants;
mod result;

use camera::Camera;
use constants::{WORLD_FORWARDS, WORLD_RIGHT, WORLD_UP};
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

use math::{Quat, Zero};
use math::Vec3;
use math::Vec4;
use math::Identity;

macro_rules! trace_error {
    ($e:expr) => {
        println!(
            "[ERROR] LINE: {}, FILE \'{}\', ERROR: \'{}\'",
            line!(),
            file!(),
            $e
        )
    };
}

#[allow(dead_code)]
struct Application {
    mouse_sensitivity: f64,
    focused_window: Option<WindowId>,
    active_window: Option<WindowId>,
    windows: HashMap<WindowId, (renderer::RenderContext, Window, Camera)>,
    renderer: renderer::Renderer,
    plane_vertex_buffer: Rc<vulkan::VertexBV>,
    plane_index_buffer: Rc<vulkan::IndexBV>,
    model_vertex_buffer: Rc<vulkan::VertexBV>,
    model_index_buffer: Rc<vulkan::IndexBV>,
    image: Rc<vulkan::Image>,
    model_transform: math::AffineTransform,
    model_base_color: Vec4<f32>,
    model_flags: u32,
    plane_transform: math::AffineTransform,
    plane_base_color: Vec4<f32>,
    plane_flags: u32,
    exiting: bool,
}

impl Application {
    fn new(
        img_path: &std::path::Path,
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<Self> {
        let instance = vulkan::Instance::new(debug_enabled, display_handle)?;
        let device = vulkan::Device::new(instance)?;
        let renderer = renderer::Renderer::new(device)?;

        const CUBE_VERTEX_BUFFER_DATA: [renderer::ShaderVertVertex; 24] = {
            const U: Vec3<f32> = WORLD_UP;
            const D: Vec3<f32> = WORLD_UP.scaled(-1.0);
            const R: Vec3<f32> = WORLD_RIGHT;
            const L: Vec3<f32> = WORLD_RIGHT.scaled(-1.0);
            const F: Vec3<f32> = WORLD_FORWARDS;
            const B: Vec3<f32> = WORLD_FORWARDS.scaled(-1.0);

            [
                // FRONT
                renderer::ShaderVertVertex {
                    position: U.add(L).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: F.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(R).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: F.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(R).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: F.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(L).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: F.into_arr(),
                },
                // BACK
                renderer::ShaderVertVertex {
                    position: U.add(R).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: B.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(L).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: B.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(L).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: B.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(R).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: B.into_arr(),
                },
                // LEFT
                renderer::ShaderVertVertex {
                    position: U.add(L).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: L.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(L).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: L.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(L).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: L.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(L).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: L.into_arr(),
                },
                // RIGHT
                renderer::ShaderVertVertex {
                    position: U.add(R).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: R.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(R).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: R.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(R).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: R.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(R).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: R.into_arr(),
                },
                // TOP
                renderer::ShaderVertVertex {
                    position: U.add(L).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: U.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(R).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: U.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(R).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: U.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: U.add(L).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: U.into_arr(),
                },
                // BOTTOM
                renderer::ShaderVertVertex {
                    position: D.add(L).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: D.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(R).add(F).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: D.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(R).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: D.into_arr(),
                },
                renderer::ShaderVertVertex {
                    position: D.add(L).add(B).into_arr(),
                    tex_coord: [0.0, 0.0],
                    normal: D.into_arr(),
                },
            ]
        };

        const CUBE_INDEX_BUFFER_DATA: [u32; 36] = [
            0, 1, 2, 2, 3, 0, // Front
            4, 5, 6, 6, 7, 4, // Back
            8, 9, 10, 10, 11, 8, // Left
            12, 13, 14, 14, 15, 12, // Right
            16, 17, 18, 18, 19, 16, // Top
            20, 21, 22, 22, 23, 20, // Bottom
        ];
        let model_vertex_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    CUBE_VERTEX_BUFFER_DATA.as_ptr() as *const u8,
                    CUBE_VERTEX_BUFFER_DATA.len() * std::mem::size_of::<renderer::ShaderVertVertex>(),
                )
            };

            renderer.create_vertex_buffer(data, CUBE_VERTEX_BUFFER_DATA.len() as u32)?
        };
        let model_index_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    CUBE_INDEX_BUFFER_DATA.as_ptr() as *const u8,
                    CUBE_INDEX_BUFFER_DATA.len() * std::mem::size_of::<u32>(),
                )
            };

            renderer.create_index_buffer(
                data,
                vk::IndexType::UINT32,
                CUBE_INDEX_BUFFER_DATA.len() as u32,
                0,
            )?
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
                    PLANE_VERTEX_BUFFER_DATA.len() * std::mem::size_of::<renderer::ShaderVertVertex>(),
                )
            };

            renderer.create_vertex_buffer(data, PLANE_VERTEX_BUFFER_DATA.len() as u32)?
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
            )?
        };

        let (image, model_scale) = {
            let image_data = image::open(img_path)?;

            let image = renderer.create_image(image_data)?;
            // let scale = Vec3::new(image.width as f32, image.height as f32, 0.0).normalized();

            (image, Vec3::new(1.0, 1.0, 1.0))
        };

        Ok(Self {
            mouse_sensitivity: 0.001,
            focused_window: None,
            active_window: None,
            renderer,
            windows: std::collections::HashMap::new(),
            plane_vertex_buffer,
            plane_index_buffer,
            model_vertex_buffer,
            model_index_buffer,
            image,
            exiting: false,
            model_transform: math::AffineTransform {
                position: WORLD_FORWARDS.scaled(0.75),
                orientation: Quat::IDENTITY,
                scalar: model_scale,
            },
            model_base_color: Vec4::new(1.0, 0.1, 0.4, 1.0),
            model_flags: 0,
            plane_transform: math::AffineTransform {
                position: WORLD_UP.scaled(-4.0),
                orientation: math::Quat::unit_from_angle_axis(-std::f32::consts::FRAC_PI_2, WORLD_RIGHT),
                scalar: Vec3::new(20.0, 20.0, 20.0),
            },
            plane_base_color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            plane_flags: 1
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
                println!("close requested!!");
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
                let mesh_ubos = [renderer::MeshUBO {
                    model: self.plane_transform.as_mat4().into_2d_arr(),
                    base_color: self.plane_base_color.into_arr(),
                    flags: self.plane_flags,
                }, renderer::MeshUBO {
                    model: self.model_transform.as_mat4().into_2d_arr(),
                    base_color: self.model_base_color.into_arr(),
                    flags: self.model_flags,
                }];

                let light_ubo = renderer::GlobalLightUBO {
                    direction: WORLD_UP.scaled(-1.0).into_arr(),
                    color: [1.0, 1.0, 1.0, 1.0],
                    ambient: 0.15,
                    ..Default::default()
                };
                let new_context = self.renderer.create_render_context(
                    &camera_ubo,
                    &mesh_ubos,
                    &light_ubo,
                    window,
                    self.image.clone(),
                )?;

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
                self.renderer.update_uniform_buffer(
                    camera_ubo_ptr,
                    std::mem::size_of::<renderer::CameraUBO>(),
                    current_buffer,
                )?;

                let current_ds = context.get_current_per_frame_descriptor_set();
                let obj_ds = context.get_per_obj_descriptor_set();
                let other_ds = context.get_other_descriptor_set();

                let pipeline = context.get_pipeline();

                let plane_dynamic_offset: [u32; 1] = [
                    context.get_per_obj_dynamic_uniform_buffers()[0].offset as u32,
                ];
                let plane_vertex_buffer = self.plane_vertex_buffer.clone();
                let plane_index_buffer = self.plane_index_buffer.clone();

                let model_dynamic_offset: [u32; 1] = [
                    context.get_per_obj_dynamic_uniform_buffers()[1].offset as u32,
                ];
                let model_vertex_buffer = self.model_vertex_buffer.clone();
                let model_index_buffer = self.model_index_buffer.clone();

                let record_draw_commands = |command_buffer: vk::CommandBuffer| unsafe {
                    current_ds.bind(command_buffer, &[]);
                    other_ds.bind(command_buffer, &[]);

                    pipeline.bind(command_buffer);

                    obj_ds.bind(command_buffer, &plane_dynamic_offset);
                    plane_vertex_buffer.bind(command_buffer);
                    plane_index_buffer.bind(command_buffer);
                    plane_index_buffer.draw(command_buffer);

                    obj_ds.bind(command_buffer, &model_dynamic_offset);
                    model_vertex_buffer.bind(command_buffer);
                    model_index_buffer.bind(command_buffer);
                    model_index_buffer.draw(command_buffer);
                };
                unsafe {
                    context.draw(record_draw_commands)?;
                }
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::event::KeyEvent;
                use winit::keyboard::KeyCode;

                // const ANGLE: f32 = 0.025;
                const SPEED: f32 = 0.025;
                match event {
                    KeyEvent { physical_key, .. } => match physical_key {
                        winit::keyboard::PhysicalKey::Code(c) => match c {
                            KeyCode::Escape => {
                                self.active_window = None;
                                match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                                    Err(e) => {
                                        println!("{}", e);
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
                            // KeyCode::ArrowUp => {
                            //     *self.model_angle.z_mut() += ANGLE;
                            // }
                            // KeyCode::ArrowDown => {
                            //     *self.model_angle.z_mut() -= ANGLE;
                            // }
                            // KeyCode::ArrowLeft => {
                            //     *self.model_angle.x_mut() += ANGLE;
                            // }
                            // KeyCode::ArrowRight => {
                            //     *self.model_angle.x_mut() -= ANGLE;
                            // }
                            _ => {}
                        },
                        _ => {}
                    },
                }
                // println!("Keyboard Input!");
            }
            WindowEvent::Moved(_) => {
                println!("Moved!");
            }
            WindowEvent::Focused(b) => {
                if b {
                    self.focused_window = Some(*window_id);
                }
                println!("Focused!");
            }
            WindowEvent::MouseInput { button, .. } => {
                use winit::event::MouseButton;

                println!("Mouse Input!");
                match button {
                    MouseButton::Left => {
                        self.active_window = self.focused_window;
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .or_else(|_| {
                                window.set_cursor_grab(winit::window::CursorGrabMode::Confined)
                            })?;
                        window.set_cursor_visible(false);
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { .. } => {
                // println!("Cursor Moved!: {} {}", position.x, position.y);
            }
            WindowEvent::AxisMotion { .. } => {
                // println!("AxisMotion");
            }
            WindowEvent::ActivationTokenDone { .. } => {
                println!("Activation Token Done");
            }
            WindowEvent::CursorLeft { .. } => {
                println!("CursorLeft!");
            }
            WindowEvent::MouseWheel { .. } => {
                println!("MouseWheel!");
            }
            WindowEvent::Occluded(_) => {
                println!("Occluded!");
            }
            WindowEvent::DroppedFile(_) => {
                println!("Dropped file!");
            }
            WindowEvent::HoveredFile(_) => {
                println!("HoveredFile");
            }
            WindowEvent::Ime(_) => {
                println!("Ime!");
            }
            WindowEvent::CursorEntered { .. } => {
                println!("CursorEntered");
                // window.set_cursor_visible(false);
            }
            WindowEvent::Destroyed { .. } => {
                println!("Destroyed!");
            }
            WindowEvent::HoveredFileCancelled => {
                println!("HoveredFileCancelled");
            }
            WindowEvent::ModifiersChanged(_) => {
                println!("ModifiersChanged");
            }
            WindowEvent::TouchpadPressure { .. } => {
                println!("TouchpadPressure");
            }
            WindowEvent::PinchGesture { .. } => {
                println!("PinchGesture");
            }
            WindowEvent::DoubleTapGesture { .. } => {
                println!("DoubleTapGesture");
            }
            WindowEvent::PanGesture { .. } => {
                println!("PanGesture");
            }
            WindowEvent::RotationGesture { .. } => {
                println!("RotationGesture");
            }
            WindowEvent::Touch(_) => {
                println!("Touch");
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                println!("ScaleFactorChanged");
            }
            WindowEvent::ThemeChanged(_) => {
                println!("ThemeChanged");
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

        println!("Exiting!");
        self.exiting = true;

        return event_loop.exit();
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed!");

        if !self.windows.is_empty() {
            return;
        }

        let window_attributes =
            winit::window::WindowAttributes::default().with_title("My Application!");
        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => w,
            Err(e) => {
                println!("Could not create window: {e:?}");
                return self.exiting(event_loop);
            }
        };
        let camera = {
            let s = window.inner_size();
            let (w, h) = (s.width as f32, s.height as f32);
            let aspect_ratio = w / h;

            Camera::new(80.0, aspect_ratio, Vec3::new(0.0, 0.0, 0.0), 0.0, 0.0)
        };
        let window_id = window.id();

        let camera_ubo = renderer::CameraUBO {
            view: camera.get_view_matrix().into_2d_arr(),
            proj: camera.get_projection_matrix().into_2d_arr(),
            ..Default::default()
        };
        let mesh_ubos = [renderer::MeshUBO {
            model: self.plane_transform.as_mat4().into_2d_arr(),
            base_color: self.plane_base_color.into_arr(),
            flags: self.plane_flags,
        }, renderer::MeshUBO {
            model: self.model_transform.as_mat4().into_2d_arr(),
            base_color: self.model_base_color.into_arr(),
            flags: self.model_flags,
        }];
        let light_ubo = renderer::GlobalLightUBO {
            direction: WORLD_UP.scaled(-1.0).into_arr(),
            color: [1.0, 1.0, 1.0, 1.0],
            ambient: 0.15,
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
                trace_error!(e);
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
                let dx = -delta.0 * self.mouse_sensitivity;
                let dy = -delta.1 * self.mouse_sensitivity;

                camera.rotate(dx as f32, dy as f32);
            }
            _ => {
                // println!("Not implemented")
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
                trace_error!(e);
                self.exiting(event_loop);
            }
        }
    }
}

fn main() -> Result<()> {
    if std::env::args().len() != 2 {
        return Err(Error::IncorrectProgramUsage);
    }

    let img_path = {
        let args: Vec<String> = std::env::args().collect();
        std::path::PathBuf::from(args[args.len() - 1].clone())
    };
    let event_loop = EventLoop::new()?;

    let mut app = {
        let owned_display_handle = event_loop.owned_display_handle();
        let display_handle = owned_display_handle.display_handle()?;
        Application::new(img_path.as_path(), true, &display_handle)?
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
