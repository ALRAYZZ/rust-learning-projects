use std::collections::VecDeque;
use std::thread;
use crossbeam_channel::{bounded, Receiver, Sender};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use std::path::Path;

// const WIDTH: u32 = 320;
// const HEIGHT: u32 = 240;
const FRAME_BUFFER_SIZE: usize = 30; // Buffer 30 frames ahead to avoid stutter


// Enum abstraction for different frame sources
enum FrameSource {
    StaticImage {
        frame: Vec<u8>,
    },
    Video {
        frame_receiver: Receiver<Vec<u8>>, // Receives decoded frames from worker thread
        frame_buffer: VecDeque<Vec<u8>>, // Ring buffer to hold decoded frames
        current_frame: Vec<u8>, // Current frame being displayed
        fps: f32,
        last_frame_time: std::time::Instant,
    }
}

struct App {
    window: Option<Arc<Box<dyn Window>>>, // We use Arc because window is shared with Pixels and App
    pixels: Option<Pixels<'static>>,
    frame_source: Option<FrameSource>, // Frame source (image or video)
    video_width: u32,
    video_height: u32,
}

impl App {
    // Determines current frame to display based on frame source
    // Static: Returns the same frame
    // Video: Advances frame when enough time has passed based on fps
    fn current_frame(frame_source: &mut Option<FrameSource>) -> Option<&[u8]> {
        match frame_source.as_mut()? {
            FrameSource::StaticImage {frame} => {
                Some(frame)
            }

            FrameSource::Video {
                frame_receiver,
                frame_buffer,
                current_frame,
                fps,
                last_frame_time,
            } => {
                // Refill buffer from decoder thread if space available
                while frame_buffer.len() < FRAME_BUFFER_SIZE {
                    match frame_receiver.try_recv() {
                        Ok(new_frame) => frame_buffer.push_back(new_frame),
                            Err(_) => break,
                    }
                }

                // Check if enough time has passed to advance next frame
                let frame_duration = std::time::Duration::from_secs_f32(1.0 / *fps);

                if last_frame_time.elapsed() > frame_duration {
                    // Try to get next frame from buffer
                    if let Some(next_frame) = frame_buffer.pop_front() {
                        *current_frame = next_frame;
                        *last_frame_time = std::time::Instant::now();
                    }
                }

                Some(current_frame)
            }
        }
    }

    // Spawns a background thread that decodes video frames and sends them to main thread
    // This architecture solves lifetime issues by:
    // 1. Creating all FFmpeg objects in worker thread
    // 2. Decoding frames sequentially with packed feeding
    // 3. Converting YUV -> RGBA using FFmpeg scaler
    // 4. Sending fully-owned Vec<u8> through channel
    fn spawn_video_decoder(video_path: &Path, sender: Sender<Vec<u8>>,
                           target_width: u32, target_height: u32) {

        let video_path = video_path.to_path_buf();

        // Spawning new thread to handle video decoding and avoiding window freezes
        thread::spawn(move || {
            // Initialize FFmpeg
            ffmpeg_next::init().expect("Failed to initialize FFmpeg");

            // Open video file and find video stream (input context)
            let mut ictx = ffmpeg_next::format::input(&video_path)
                .expect("Failed to open video file");

            let video_stream = ictx
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .expect("No video stream found");

            let video_stream_index = video_stream.index();

            // Create decoder for the video stream
            let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(
                video_stream.parameters()
            ).expect("Failed to create codec context");

            let mut decoder = context_decoder
                .decoder()
                .video()
                .expect("Failed to create video decoder");

            // Create scaler to convert YUV -> RGBA and resize to target dimensions
            // YUV is a format to allow efficient compression and storage of color data
            // We need RGBA for actual rendering on screen
            let mut scaler = ffmpeg_next::software::scaling::Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                ffmpeg_next::format::Pixel::RGBA,
                target_width,
                target_height,
                ffmpeg_next::software::scaling::flag::Flags::BILINEAR
            ).expect("Failed to create scaler");

            // Decode loop: read packets -> send decoder -> receive frames -> convert -> send
            for (stream, packet) in ictx.packets() {
                // Only process packets from video stream
                if stream.index() == video_stream_index {
                    // Send packet to decoder
                    decoder.send_packet(&packet).expect("Failed to send packet");

                    // Receive all available decoded frames
                    let mut decoded_frame = ffmpeg_next::util::frame::video::Video::empty();
                    // Loop while there are frames to receive
                    while decoder.receive_frame(&mut decoded_frame).is_ok() {
                        // Create empty frame for scaled output
                        let mut rgb_frame = ffmpeg_next::util::frame::video::Video::empty();

                        // Scale and convert to RGBA
                        scaler.run(&decoded_frame, &mut rgb_frame)
                            .expect("Failed to scale frame");

                        // Copy RGBA data to owned Vec
                        let data = rgb_frame.data(0);
                        let frame_data = data.to_vec();

                        // Send to main thread (blocks if buffer full)
                        if sender.send(frame_data).is_err() {
                            // Main thread deopped receiver, stop decoding
                            return;
                        }
                    }
                }
            }

            // Flush decoder to get remaing frames
            decoder.send_eof().ok();
            let mut decoded_frame = ffmpeg_next::util::frame::video::Video::empty();
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                let mut rgb_frame = ffmpeg_next::util::frame::video::Video::empty();
                scaler.run(&decoded_frame, &mut rgb_frame).ok();
                let data = rgb_frame.data(0);
                sender.send(data.to_vec()).ok();
            }
        });
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            pixels: None,
            frame_source: None,
            video_height: 0,
            video_width: 0,
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

    // This method is called by winit when the evnet loop has started
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Set up video playback
        // Get video file path
        let video_path = Path::new("sample_video.mp4");
        // Initialize ffmpeg
        ffmpeg_next::init().ok();

        // Get video metadata
        let ictx = ffmpeg_next::format::input(&video_path)
            .expect("Failed to open video file for metadata");
        let video_stream = ictx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .expect("No video stream found");

        let audio_stream = ictx
            .streams()
            .best(ffmpeg_next::media::Type::Audio)
            .expect("No audio stream found");

        // Create ffmpeg context from stream parameters
        let video_context = ffmpeg_next::codec::context::Context::from_parameters(
            video_stream.parameters()
        ).expect("Failed to create codec context from parameters");

        let audio_context = ffmpeg_next::codec::context::Context::from_parameters(
            audio_stream.parameters()
        ).expect("Failed to create audio codec context from parameters");

        let mut audio_decoder = audio_context.decoder()
            .audio()
            .expect("Failed to create audio decoder");

        // Creating 2nd decoder just for metadata extraction
        // REFACTOR LATER: Decoder thread; opens decoder reads info and sends a message of videoinfo
        // Main thread creates window and pixels after receiving dimensions
        // Using context to create decoder
        let video_decoder = video_context.decoder().video()
            .expect("Failed to create video decoder for metadata");

        // Store video dimensions
        self.video_width = video_decoder.width();
        self.video_height = video_decoder.height();

        // Calculate FPS from time base
        let fps_ratio = video_stream.avg_frame_rate();
        let fps = fps_ratio.numerator() as f32 / fps_ratio.denominator() as f32;

        let window_attributes = WindowAttributes::default()
            .with_surface_size(LogicalSize::new(self.video_width, self.video_height))
            .with_title("Rust Video Player");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let window_size = window.surface_size();

        let surface_texture = SurfaceTexture::new(
            window_size.width,
            window_size.height,
            window.clone(),
        );
        let pixels = Pixels::new(self.video_width, self.video_height, surface_texture)
            .expect("Failed to create Pixels");

        // Create channel for decoder thread to send frames to main thread
        let (sender, receiver) = bounded(FRAME_BUFFER_SIZE);

        // Spawn decoder thread
        Self::spawn_video_decoder(video_path, sender, self.video_width, self.video_height);

        // Initialzie frame source with video config
        self.frame_source = Some(FrameSource::Video {
            frame_receiver: receiver,
            frame_buffer: VecDeque::with_capacity(FRAME_BUFFER_SIZE),
            current_frame: vec![0; (self.video_width * self.video_height * 4) as usize], // Black initial frame
            fps,  // Adjust
            last_frame_time: std::time::Instant::now(),
        });


        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent
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
