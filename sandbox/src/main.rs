pub mod result;

use ash::vk;
use renderer::render_context;
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
    color: [f32; 3],
}

#[allow(dead_code)]
struct Application {
    windows: HashMap<WindowId, (Window, renderer::render_context::RenderContext)>,
    device: Rc<vulkan::device::Device>,
    vertex_buffer: Rc<vulkan::buffer::BufferView>,
    index_buffer: Rc<vulkan::buffer::BufferView>,
    exiting: bool,
}

impl Application {
    fn new(
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<Self> {
        let instance = vulkan::device::Instance::new(debug_enabled, display_handle)?;
        let device = vulkan::device::Device::new(instance)?;
        let device = Rc::new(device);

        const F: f32 = 0.75;
        let vertex_buffer_data = vec![
            Vertex {
                position: [-F, F, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [F, F, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.0, -F, 0.0],
                color: [0.0, 0.0, 1.0],
            },
        ];
        let index_buffer_data = vec![0, 1, 2];

        let vertex_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    vertex_buffer_data.as_ptr() as *const u8,
                    vertex_buffer_data.len() * std::mem::size_of::<Vertex>(),
                )
            };

            renderer::create_vertex_buffer(device.clone(), data, 3, 0)?
        };
        let index_buffer = {
            let data = unsafe {
                std::slice::from_raw_parts(
                    index_buffer_data.as_ptr() as *const u8,
                    index_buffer_data.len() * std::mem::size_of::<Vertex>(),
                )
            };

            renderer::create_index_buffer(device.clone(), data, vk::IndexType::UINT32, 3, 0)?
        };

        Ok(Self {
            device,
            windows: std::collections::HashMap::new(),
            vertex_buffer,
            index_buffer,
            exiting: false,
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
        let (window, context) = self.windows.get_mut(window_id).unwrap();

        match event {
            winit::event::WindowEvent::CloseRequested => {
                println!("close requested!!");
                // unsafe { self.renderer.destroy_render_context(context) };
                return Ok(true);
            }
            winit::event::WindowEvent::Resized(new_size) => {
                println!("Resizeing to {:?}", new_size);

                let new_context = render_context::RenderContext::new(self.device.clone(), window)?;
                *context = new_context;

                println!("Resized complete!");
            }
            winit::event::WindowEvent::RedrawRequested => {
                println!("Redraw requested!");
                let vertex_buffer = self.vertex_buffer.clone();
                let index_buffer = self.index_buffer.clone();
                let record_draw_commands = |command_buffer: ash::vk::CommandBuffer| {
                    unsafe {
                        vertex_buffer.bind(command_buffer);
                        index_buffer.bind(command_buffer);
                        index_buffer.draw(command_buffer);
                    }
                };
                unsafe {
                    context.draw(record_draw_commands)?;
                }
            }
            _ => {
                // println!("Not implemented!");
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

        // cleanup here?

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
        let context =
            match renderer::render_context::RenderContext::new(
                self.device.clone(),
                &window
            ) {
                Ok(context) => context,
                Err(e) => {
                    trace_error!(e);
                    return self.exiting(event_loop);
                }
            };
        self.windows.insert(window_id, (window, context));
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
    println!("Current dir: {}", std::env::current_dir()?.display());
    let event_loop = EventLoop::new()?;

    let mut app = {
        let owned_display_handle = event_loop.owned_display_handle();
        let display_handle = owned_display_handle.display_handle()?;
        Application::new(true, &display_handle)?
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
