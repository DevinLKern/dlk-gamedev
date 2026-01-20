pub mod camera;
pub mod constants;
pub mod result;

use camera::Camera;

use ash::vk;
use result::Result;

use std::collections::HashMap;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowId},
};

use math::traits::Zero;
use math::vec2::Vec2;
use math::vec3::Vec3;

use crate::constants::{WORLD_RIGHT, WORLD_UP};

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
    windows: HashMap<WindowId, (renderer::render_context::RenderContext, Window, Camera)>,
    renderer: renderer::Renderer,
    vertex_buffer: Rc<vulkan::buffer::BufferView>,
    index_buffer: Rc<vulkan::buffer::BufferView>,
    image: Rc<vulkan::image::Image>,
    model_position: Vec3<f32>,
    model_angle: Vec3<f32>,
    model_scale: Vec3<f32>,
    exiting: bool,
}

impl Application {
    fn new(
        img_path: &std::path::Path,
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<Self> {
        let instance = vulkan::device::Instance::new(debug_enabled, display_handle)?;
        let device = vulkan::device::Device::new(instance)?;
        let renderer = renderer::Renderer::new(Rc::new(device))?;

        // const F: f32 = 0.75;
        const TR: Vec3<f32> = WORLD_UP.add(WORLD_RIGHT);
        const TL: Vec3<f32> = WORLD_UP.add(WORLD_RIGHT.scaled(-1.0));
        const BR: Vec3<f32> = WORLD_UP.scaled(-1.0).add(WORLD_RIGHT);
        const BL: Vec3<f32> = WORLD_UP.scaled(-1.0).add(WORLD_RIGHT.scaled(-1.0));
        let vertex_buffer_data = vec![
            renderer::render_context::Vertex {
                position: TL,
                tex_coord: Vec2::new(1.0, 0.0),
            },
            renderer::render_context::Vertex {
                position: TR,
                tex_coord: Vec2::new(0.0, 0.0),
            },
            renderer::render_context::Vertex {
                position: BR,
                tex_coord: Vec2::new(0.0, 1.0),
            },
            renderer::render_context::Vertex {
                position: BL,
                tex_coord: Vec2::new(1.0, 1.0),
            },
        ];
        let index_buffer_data = vec![0, 1, 2, 2, 3, 0];

        let vertex_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    vertex_buffer_data.as_ptr() as *const u8,
                    vertex_buffer_data.len()
                        * std::mem::size_of::<renderer::render_context::Vertex>(),
                )
            };

            renderer.create_vertex_buffer(data, vertex_buffer_data.len() as u32, 0)?
        };
        let index_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    index_buffer_data.as_ptr() as *const u8,
                    index_buffer_data.len()
                        * std::mem::size_of::<renderer::render_context::Vertex>(),
                )
            };

            renderer.create_index_buffer(
                data,
                vk::IndexType::UINT32,
                index_buffer_data.len() as u32,
                0,
            )?
        };

        let (image, model_scale) = {
            let image_data = image::open(img_path)?;

            let image = renderer.create_image(image_data)?;
            let scale = Vec3::new(image.width as f32, image.height as f32, 0.0).normalized();

            (image, scale)
        };

        Ok(Self {
            mouse_sensitivity: 0.001,
            focused_window: None,
            active_window: None,
            renderer,
            windows: std::collections::HashMap::new(),
            vertex_buffer,
            index_buffer,
            image,
            exiting: false,
            model_position: constants::WORLD_FORWARDS.scaled(0.75),
            model_angle: Vec3::ZERO,
            model_scale,
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
        let (context, window, camera) = self.windows.get_mut(window_id).unwrap();

        match event {
            winit::event::WindowEvent::CloseRequested => {
                println!("close requested!!");
                // unsafe { self.renderer.destroy_render_context(context) };
                return Ok(true);
            }
            winit::event::WindowEvent::Resized(_) => {
                {
                    let s = window.inner_size();
                    let (w, h) = (s.width as f32, s.height as f32);
                    let aspect_ratio = w / h;

                    camera.set_aspect_ratio(aspect_ratio);
                }

                let camera_ubo =
                    camera.calculate_ubo(self.model_position, self.model_scale, self.model_angle);
                let new_context =
                    self.renderer
                        .create_render_context(&camera_ubo, window, self.image.clone())?;

                *context = new_context;
            }
            winit::event::WindowEvent::RedrawRequested => {
                // println!("Redraw requested!");

                // the the orthographic view volume's cetner is (0, 0, 0)
                let camera_ubo = camera.calculate_ubo(
                    self.model_position,
                    self.model_scale,
                    self.model_angle.clone(),
                );
                context.update_current_camera_ubo(&camera_ubo);
                let vertex_buffer = self.vertex_buffer.clone();
                let index_buffer = self.index_buffer.clone();
                let record_draw_commands = |command_buffer: ash::vk::CommandBuffer| unsafe {
                    vertex_buffer.bind(command_buffer);
                    index_buffer.bind(command_buffer);
                    index_buffer.draw(command_buffer);
                };
                unsafe {
                    context.draw(record_draw_commands)?;
                }
                window.request_redraw();
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                // const ANGLE: f32 = 0.025;
                const SPEED: f32 = 0.025;
                match event {
                    winit::event::KeyEvent { physical_key, .. } => match physical_key {
                        winit::keyboard::PhysicalKey::Code(c) => match c {
                            winit::keyboard::KeyCode::Escape => {
                                self.active_window = None;
                                match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                                    Err(e) => {
                                        println!("{}", e);
                                    }
                                    _ => {}
                                }
                                window.set_cursor_visible(true);
                            }
                            winit::keyboard::KeyCode::KeyE => {
                                camera.move_local(constants::WORLD_FORWARDS.scaled(SPEED));
                            }
                            winit::keyboard::KeyCode::KeyD => {
                                camera.move_local(constants::WORLD_FORWARDS.scaled(-SPEED));
                            }
                            winit::keyboard::KeyCode::KeyF => {
                                camera.move_local(constants::WORLD_RIGHT.scaled(SPEED));
                            }
                            winit::keyboard::KeyCode::KeyS => {
                                camera.move_local(constants::WORLD_RIGHT.scaled(-SPEED));
                            }
                            winit::keyboard::KeyCode::Space => {
                                camera.move_global(constants::WORLD_UP.scaled(SPEED));
                            }
                            winit::keyboard::KeyCode::ControlLeft => {
                                // this is fine
                                camera.move_global(constants::WORLD_UP.scaled(-SPEED));
                            }
                            // winit::keyboard::KeyCode::ArrowUp => {
                            //     *self.model_angle.z_mut() += ANGLE;
                            // }
                            // winit::keyboard::KeyCode::ArrowDown => {
                            //     *self.model_angle.z_mut() -= ANGLE;
                            // }
                            // winit::keyboard::KeyCode::ArrowLeft => {
                            //     *self.model_angle.x_mut() += ANGLE;
                            // }
                            // winit::keyboard::KeyCode::ArrowRight => {
                            //     *self.model_angle.x_mut() -= ANGLE;
                            // }
                            _ => {}
                        },
                        _ => {}
                    },
                }
                // println!("Keyboard Input!");
            }
            winit::event::WindowEvent::Moved(_) => {
                println!("Moved!");
            }
            winit::event::WindowEvent::Focused(b) => {
                if b {
                    self.focused_window = Some(*window_id);
                }
                println!("Focused!");
            }
            winit::event::WindowEvent::MouseInput { button, .. } => {
                println!("Mouse Input!");
                match button {
                    winit::event::MouseButton::Left => {
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
            winit::event::WindowEvent::CursorMoved { .. } => {
                // println!("Cursor Moved!: {} {}", position.x, position.y);
            }
            winit::event::WindowEvent::AxisMotion { .. } => {
                // println!("AxisMotion");
            }
            winit::event::WindowEvent::ActivationTokenDone { .. } => {
                println!("Activation Token Done");
            }
            winit::event::WindowEvent::CursorLeft { .. } => {
                println!("CursorLeft!");
            }
            winit::event::WindowEvent::MouseWheel { .. } => {
                println!("MouseWheel!");
            }
            winit::event::WindowEvent::Occluded(_) => {
                println!("Occluded!");
            }
            winit::event::WindowEvent::DroppedFile(_) => {
                println!("Dropped file!");
            }
            winit::event::WindowEvent::HoveredFile(_) => {
                println!("HoveredFile");
            }
            winit::event::WindowEvent::Ime(_) => {
                println!("Ime!");
            }
            winit::event::WindowEvent::CursorEntered { .. } => {
                println!("CursorEntered");
                // window.set_cursor_visible(false);
            }
            winit::event::WindowEvent::Destroyed { .. } => {
                println!("Destroyed!");
            }
            winit::event::WindowEvent::HoveredFileCancelled => {
                println!("HoveredFileCancelled");
            }
            winit::event::WindowEvent::ModifiersChanged(_) => {
                println!("ModifiersChanged");
            }
            winit::event::WindowEvent::TouchpadPressure { .. } => {
                println!("TouchpadPressure");
            }
            winit::event::WindowEvent::PinchGesture { .. } => {
                println!("PinchGesture");
            }
            winit::event::WindowEvent::DoubleTapGesture { .. } => {
                println!("DoubleTapGesture");
            }
            winit::event::WindowEvent::PanGesture { .. } => {
                println!("PanGesture");
            }
            winit::event::WindowEvent::RotationGesture { .. } => {
                println!("RotationGesture");
            }
            winit::event::WindowEvent::Touch(_) => {
                println!("Touch");
            }
            winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                println!("ScaleFactorChanged");
            }
            winit::event::WindowEvent::ThemeChanged(_) => {
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

            camera::Camera::new(80.0, aspect_ratio, Vec3::new(0.0, 0.0, 0.0), 0.0, 0.0)
        };
        let window_id = window.id();
        let camera_ubo =
            camera.calculate_ubo(self.model_position, self.model_scale, self.model_angle);
        let context =
            match self
                .renderer
                .create_render_context(&camera_ubo, &window, self.image.clone())
            {
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
        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                if let Some(window_id) = self.active_window {
                    let (_, _, camera) = self.windows.get_mut(&window_id).unwrap();

                    let dx = -delta.0 * self.mouse_sensitivity;
                    let dy = -delta.1 * self.mouse_sensitivity;

                    camera.rotate(dx as f32, dy as f32);
                }
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
    let img_path = {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 3 {
            for arg in args.iter() {
                println!("{}", arg);
            }
            let e = result::Error::IncorrectProgramUsage;
            println!("{}", e);
            return Err(e);
        }

        std::env::set_current_dir(args[args.len() - 2].clone())?;

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
