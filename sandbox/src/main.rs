pub mod result;

use ash::vk;
use renderer::camera;
use result::Result;

use std::collections::HashMap;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowId},
};

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

#[repr(C)]
#[derive(Default)]
pub struct Vertex {
    position: [f32; 3],
    tex_coord: [f32; 2],
}

#[allow(dead_code)]
struct Application {
    windows: HashMap<WindowId, (renderer::render_context::RenderContext, Window)>,
    renderer: renderer::Renderer,
    vertex_buffer: Rc<vulkan::buffer::BufferView>,
    index_buffer: Rc<vulkan::buffer::BufferView>,
    image: Rc<vulkan::image::Image>,
    model_anlge: math::vectors::Vec3<f32>,
    camera: camera::Camera,
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

        const F: f32 = 0.75;
        let vertex_buffer_data = vec![
            Vertex {
                position: [-F, -F, 0.0],
                tex_coord: [1.0, 0.0],
            },
            Vertex {
                position: [F, -F, 0.0],
                tex_coord: [0.0, 0.0],
            },
            Vertex {
                position: [F, F, 0.0],
                tex_coord: [0.0, 1.0],
            },
            Vertex {
                position: [-F, F, 0.0],
                tex_coord: [1.0, 1.0],
            },
        ];
        let index_buffer_data = vec![0, 1, 2, 2, 3, 0];

        let vertex_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    vertex_buffer_data.as_ptr() as *const u8,
                    vertex_buffer_data.len() * std::mem::size_of::<Vertex>(),
                )
            };

            renderer.create_vertex_buffer(data, vertex_buffer_data.len() as u32, 0)?
        };
        let index_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    index_buffer_data.as_ptr() as *const u8,
                    index_buffer_data.len() * std::mem::size_of::<Vertex>(),
                )
            };

            renderer.create_index_buffer(
                data,
                vk::IndexType::UINT32,
                index_buffer_data.len() as u32,
                0,
            )?
        };

        let image = {
            let image_data = image::open(img_path)?;

            renderer.create_image(image_data)?
        };

        Ok(Self {
            renderer,
            windows: std::collections::HashMap::new(),
            vertex_buffer,
            index_buffer,
            image,
            exiting: false,
            model_anlge: math::vectors::Vec3::default(),
            camera: camera::Camera::new(),
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
        let (context, window) = self.windows.get_mut(window_id).unwrap();

        match event {
            winit::event::WindowEvent::CloseRequested => {
                println!("close requested!!");
                // unsafe { self.renderer.destroy_render_context(context) };
                return Ok(true);
            }
            winit::event::WindowEvent::Resized(_) => {
                let new_context = self
                    .renderer
                    .create_render_context(window, self.image.clone())?;
                *context = new_context;
            }
            winit::event::WindowEvent::RedrawRequested => {
                // println!("Redraw requested!");
                let camera_ubo = self.camera.calculate_ubo(
                    math::vectors::Vec3::new(0.0, 0.0, -1.0),
                    math::vectors::Vec3::new(1.0, 1.0, 1.0),
                    self.model_anlge.clone(),
                );
                context.update_current_camera(&camera_ubo);
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
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                const ANGLE: f32 = 0.1;
                match event {
                    winit::event::KeyEvent { physical_key, .. } => match physical_key {
                        winit::keyboard::PhysicalKey::Code(c) => match c {
                            winit::keyboard::KeyCode::ArrowUp => {
                                self.model_anlge[0] += ANGLE;
                            }
                            winit::keyboard::KeyCode::ArrowDown => {
                                self.model_anlge[0] -= ANGLE;
                            }
                            winit::keyboard::KeyCode::ArrowLeft => {
                                self.model_anlge[1] += ANGLE;
                            }
                            winit::keyboard::KeyCode::ArrowRight => {
                                self.model_anlge[1] -= ANGLE;
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                }
                window.request_redraw();
                // println!("Keyboard Input!");
            }
            winit::event::WindowEvent::Moved(_) => {
                println!("Moved!");
            }
            winit::event::WindowEvent::Focused(_) => {
                println!("Focused!");
            }
            winit::event::WindowEvent::MouseInput { .. } => {
                println!("Mouse Input!");
            }
            winit::event::WindowEvent::CursorMoved { .. } => {
                // println!("Cursor Moved!");
            }
            winit::event::WindowEvent::AxisMotion { .. } => {
                println!("AxisMotion");
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
        let window_id = window.id();
        let context = match self
            .renderer
            .create_render_context(&window, self.image.clone())
        {
            Ok(context) => context,
            Err(e) => {
                trace_error!(e);
                return self.exiting(event_loop);
            }
        };
        self.windows.insert(window_id, (context, window));
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
