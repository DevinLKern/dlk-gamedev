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
use math::Vec2;
use math::Vec3;
use math::Mat4;
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
    vertex_buffer: Rc<vulkan::BufferView>,
    index_buffer: Rc<vulkan::BufferView>,
    image: Rc<vulkan::Image>,
    model_transform: math::AffineTransform,
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

        const VERTEX_BUFFER_DATA: [renderer::Vertex; 4] = {
            const TR: Vec3<f32> = WORLD_UP.add(WORLD_RIGHT);
            const TL: Vec3<f32> = WORLD_UP.add(WORLD_RIGHT.scaled(-1.0));
            const BR: Vec3<f32> = WORLD_UP.scaled(-1.0).add(WORLD_RIGHT);
            const BL: Vec3<f32> = WORLD_UP.scaled(-1.0).add(WORLD_RIGHT.scaled(-1.0));
            [
                renderer::Vertex::new(TL, Vec2::new(1.0, 0.0)),
                renderer::Vertex::new(TR, Vec2::new(0.0, 0.0)),
                renderer::Vertex::new(BR, Vec2::new(0.0, 1.0)),
                renderer::Vertex::new(BL, Vec2::new(1.0, 1.0)),
            ]
        };
        const INDEX_BUFFER_DATA: [u32; 6] = [0, 1, 2, 2, 3, 0];

        let vertex_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    VERTEX_BUFFER_DATA.as_ptr() as *const u8,
                    VERTEX_BUFFER_DATA.len()
                        * std::mem::size_of::<renderer::Vertex>(),
                )
            };

            renderer.create_vertex_buffer(data, VERTEX_BUFFER_DATA.len() as u32, 0)?
        };
        let index_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    INDEX_BUFFER_DATA.as_ptr() as *const u8,
                    INDEX_BUFFER_DATA.len()
                        * std::mem::size_of::<renderer::Vertex>(),
                )
            };

            renderer.create_index_buffer(
                data,
                vk::IndexType::UINT32,
                INDEX_BUFFER_DATA.len() as u32,
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
            model_transform: math::AffineTransform{position: WORLD_FORWARDS.scaled(0.75), orientation: Quat::IDENTITY, scalar: model_scale},
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

                let camera_ubo = renderer::CameraUBO { model: self.model_transform.as_mat4(), view: camera.get_view_matrix(), proj: camera.get_projection_matrix() };
                let new_context =
                    self.renderer
                        .create_render_context(&camera_ubo, window, self.image.clone())?;

                *context = new_context;
            }
            WindowEvent::RedrawRequested => {
                // println!("Redraw requested!");

                let camera_ubo = renderer::CameraUBO { model: self.model_transform.as_mat4(), view: camera.get_view_matrix(), proj: camera.get_projection_matrix() };
                context.update_current_camera_ubo(&camera_ubo);
                let vertex_buffer = self.vertex_buffer.clone();
                let index_buffer = self.index_buffer.clone();
                let record_draw_commands = |command_buffer: vk::CommandBuffer| unsafe {
                    vertex_buffer.bind(command_buffer);
                    index_buffer.bind(command_buffer);
                    index_buffer.draw(command_buffer);
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
        let camera_ubo = renderer::CameraUBO { model: self.model_transform.as_mat4(), view: camera.get_view_matrix(), proj: camera.get_projection_matrix() };
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
