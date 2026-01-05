use pixels::{Error, Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

struct App {
    window: Option<Arc<Box<dyn Window>>>,
    pixels: Option<Pixels<'static>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            pixels: None,
        }
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Event loop has started, we can initialize our window now

        // Create simple window with default attributes
        let window_attributes = WindowAttributes::default()
            .with_surface_size(LogicalSize::new(WIDTH, HEIGHT))
            .with_title("Rust Video Player");

        let window = event_loop.create_window(window_attributes).unwrap();
        let window = Arc::new(window);
        let window_size = window.surface_size();

        let surface_texture = SurfaceTexture::new(
            window_size.width,
            window_size.height,
            window.clone()
        );

        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId, event: WindowEvent
    ) {
        //  Called by "EventLoop::run_app" when new event happens on window
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::RedrawRequested =>  {
                // Redraw the app

                self.window.as_ref().unwrap().request_redraw();
            },
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create new event loop
    let event_loop = EventLoop::new()?;

    // Configure settings

    // Continue polling even when there are no events
    // ControlFlow::Wait would sleep the thread when there are no events wait for user input
    event_loop.set_control_flow(ControlFlow::Poll);

    // Launch and begin running the event loop
    event_loop.run_app(App::default())?;

    Ok(())
}
