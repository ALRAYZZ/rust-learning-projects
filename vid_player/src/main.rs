use pixels::{Error, Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use image::{DynamicImage, ImageBuffer, Rgba};
use std::path::Path;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

struct App {
    window: Option<Arc<Box<dyn Window>>>, // We use Arc because window is shared with Pixels and App
    pixels: Option<Pixels<'static>>,
    frame_data: Option<Vec<u8>> // Store preloaded RGBA bytes
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            pixels: None,
            frame_data: None
        }
    }
}

// ApplicationHandler is how winit talks back to the app when events happen
impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            // Initial redraw after window creation
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Event loop has started, we can initialize our window now

        // Create simple window with default attributes
        let window_attributes = WindowAttributes::default()
            .with_surface_size(LogicalSize::new(WIDTH, HEIGHT))
            .with_title("Rust Video Player");

        // We are not creating the window, but asking for winit to create it now that is safe
        let window = event_loop.create_window(window_attributes).unwrap();
        let window = Arc::new(window);
        let window_size = window.surface_size();

        let surface_texture = SurfaceTexture::new(
            window_size.width,
            window_size.height,
            window.clone()
        );

        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();

        // Load and prepare image ONCE
        let img_path = Path::new("test_image.png");
        let img: DynamicImage = image::open(img_path).expect("Failed to open test image");

        // Resize image
        let resized_img = img.resize_exact(
            WIDTH, HEIGHT, image::imageops::FilterType::Lanczos3);

        // Convert to RGBA8
        let  rgba: ImageBuffer<Rgba<u8>, Vec<u8>> = resized_img.to_rgba8();
        let bytes: Vec<u8> = rgba.into_raw();

        self.frame_data = Some(bytes);


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
            WindowEvent::SurfaceResized(new_size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    let _ = pixels.resize_surface(new_size.width, new_size.height);
                }
            }
            WindowEvent::RedrawRequested =>  {
                // Redraw the app
                if let (Some(pixels),
                    Some(window),
                    Some(frame_data)) = (
                    &mut self.pixels, &self.window, &self.frame_data
                ) {

                    let frame = pixels.frame_mut();

                    // Copy preloaded frame data into pixel buffer every frame
                    frame.copy_from_slice(frame_data);

                    if let Err(err) = pixels.render() {
                        eprintln!("pixels.render() failed: {:?}", err);
                        event_loop.exit();
                        return;
                    }

                    window.request_redraw(); // Queue next frame
                }

            }
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
    // We give control to winit, and it will manage the calls of the implemented methods
    event_loop.run_app(App::default())?;

    Ok(())
}
