use pixels::{Error, Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use image::{DynamicImage, Frame, ImageBuffer, Rgba};
use std::path::Path;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

// Enum abstraction for different frame sources
enum FrameSource {
    StaticImage {
        frame: Vec<u8>,
    },
    Video {
        frames: Vec<Vec<u8>>,
        current: usize,
        fps: f32,
        last_frame_time: std::time::Instant,
    }
}

struct App {
    window: Option<Arc<Box<dyn Window>>>, // We use Arc because window is shared with Pixels and App
    pixels: Option<Pixels<'static>>,
    frame_source: Option<FrameSource> // Frame source (image or video)
}

impl App {
    // This method determines the current frame to display based on the frame source
    // We avoid passing the whole app struct to prevent multiple mutable borrows
    // We just pass the frame source mutable reference
    fn current_frame(frame_source: &mut Option<FrameSource>) -> Option<&[u8]> {
        match frame_source.as_mut()? {
            FrameSource::StaticImage {frame} => {
                Some(frame)
            }

            FrameSource::Video {
                frames,
                current,
                fps,
                last_frame_time,
            } => {
                let frame_duration = std::time::Duration::from_secs_f32(1.0 / *fps);

                if last_frame_time.elapsed() >= frame_duration {
                    *current = (*current + 1) % frames.len();
                    *last_frame_time = std::time::Instant::now();
                }

                Some(&frames[*current])
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            pixels: None,
            frame_source: None
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

        // Load GIF file
        let gif_path = Path::new("test_animation.gif");
        let gif_file = std::fs::File::open(gif_path).expect("Failed to open GIF file");
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(gif_file).expect("Failed to read GIF info");

        // Extract all frames
        let mut frames = Vec::new();
        while let Some(frame) = decoder.read_next_frame().expect("Failed to read frame") {
            // Resize frame to target dimensions
            let img = image::RgbaImage::from_raw(
                frame.width as u32,
                frame.height as u32,
                frame.buffer.to_vec()
            ).expect("Invalid frame data");

            let resized = image::imageops::resize(
                &img,
                WIDTH,
                HEIGHT,
                image::imageops::FilterType::Lanczos3
            );

            frames.push(resized.into_raw());
        }

        self.frame_source = Some(FrameSource::Video {
            frames,
            current: 0,
            fps: 10.0,  // Adjust
            last_frame_time: std::time::Instant::now(),
        });


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
            // Event that fires every frame when the window needs to be redrawn
            // 1. Get data (What should I draw now?)
            // 2. Check if tools are ready (Can I draw now?) (Pixels and Window)
            // 3. Load: Put the image data into GPU memory -> Copy bytes to buffer
            // 4. Render -> GPU renders the buffer to window
            WindowEvent::RedrawRequested =>  {
                // Redraw the window contents

                // Get current frame data (only borrow self.frame_source mutably)
                // Asking: What image data should I draw now?
                // Static image will always return the same data
                // Video will return next frame based on timing
                let frame_data = Self::current_frame(&mut self.frame_source);

                // Borrow pixels and window
                // Check if  we have rendering tools
                if let (Some(pixels),
                    Some(window)) = (&mut self.pixels, &self.window) {
                    // Copy frame data to pixel buffer (GPU buffer)
                    if let Some(frame_data) = frame_data {
                        let frame = pixels.frame_mut(); // Mutable access to pixel buffer
                        frame.copy_from_slice(frame_data); // Copy image data to pixel buffer
                    }

                    // Render to screen
                    if pixels.render().is_err() {
                        event_loop.exit();
                        return;
                    }

                    // This creates the continuous rendering loop
                    // Else only when resizing or OS calls it we would redraw
                    window.request_redraw();
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
