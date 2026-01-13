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
use std::sync::atomic::{AtomicU64, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ffmpeg_next::codec::Audio;

// const WIDTH: u32 = 320;
// const HEIGHT: u32 = 240;
const FRAME_BUFFER_SIZE: usize = 30; // Buffer 30 frames ahead to avoid stutter

// Struct to hold video frame data and timestamp
struct VideoFrame {
    pts: f64, // Timestamp in seconds
    video_data: Vec<u8>, // RGBA pixel data
}

struct AudioFrame {
    pts: f64,
    samples: Vec<f32>, // Interleaved stereo samples
}

struct AudioClock {
    samples_played: AtomicU64,
    sample_rate: u32,
}

impl  AudioClock {
    fn new(sample_rate: u32) -> Self {
        Self {
            samples_played: AtomicU64::new(0),
            sample_rate,
        }
    }

    fn time(&self) -> f64 {
        self.samples_played.load(Ordering::Relaxed) as f64 / self.sample_rate as f64
    }
}

// Enum abstraction for different frame sources
enum FrameSource {
    StaticImage {
        frame: Vec<u8>,
    },
    Video {
        frame_receiver: Receiver<VideoFrame>, // Receives decoded frames from worker thread
        frame_buffer: VecDeque<VideoFrame>, // Ring buffer to hold decoded frames
        current_frame: Vec<u8>, // Current frame being displayed
        fps: f32,
        last_frame_time: std::time::Instant,
        start_instant: std::time::Instant,
    }
}


struct App {
    window: Option<Arc<Box<dyn Window>>>, // We use Arc because window is shared with Pixels and App
    pixels: Option<Pixels<'static>>,
    frame_source: Option<FrameSource>, // Frame source (image or video)
    audio_receiver: Option<Receiver<Vec<f32>>>, // Audio samples receiver
    audio_stream: Option<cpal::Stream>,
    audio_clock: Option<Arc<AudioClock>>,
    video_width: u32,
    video_height: u32,
}

impl App {
    // Determines current frame to display based on frame source
    // Static: Returns the same frame
    // Video: Advances frame when enough time has passed based on fps
    fn current_frame<'a>(frame_source: &'a mut Option<FrameSource>, audio_clock: Option<&Arc<AudioClock>>) -> Option<&'a [u8]> {
        match frame_source.as_mut()? {
            FrameSource::StaticImage {frame} => {
                Some(frame.as_slice())
            }

            FrameSource::Video {
                frame_receiver,
                frame_buffer,
                current_frame,
                fps: _,
                last_frame_time: _,
                start_instant,
            } => {
                // Refill buffer from decoder thread if space available
                while frame_buffer.len() < FRAME_BUFFER_SIZE {
                    match frame_receiver.try_recv() {
                        Ok(new_frame) => frame_buffer.push_back(new_frame),
                            Err(_) => break,
                    }
                }

                // Prefer audio clock if available
                let now_time = if let Some(clock) = audio_clock {
                    clock.time()
                } else {
                    start_instant.elapsed().as_secs_f64()
                };

                // Display the newest frame whose pts <= now_time
                while let Some(front) = frame_buffer.front() {
                    if front.pts <= now_time {
                        let next = frame_buffer.pop_front().unwrap();
                        *current_frame = next.video_data;
                    } else {
                        break;
                    }
                }

                Some(current_frame.as_slice())
            }
        }
    }

    // Spawns background thread that demuxes and decodes video and audio packets.
    // This architecture solves lifetime issues by:
    // 1. Creating all FFmpeg objects in worker thread
    // 2. Decoding frames sequentially with packed feeding
    // 3. Converting YUV -> RGBA using FFmpeg scaler
    // 4. Sending fully-owned Vec<u8> through channel
    fn spawn_demux_decode_thread(video_path: &Path,
                           v_sender: Sender<VideoFrame>,
                           a_sender: Sender<AudioFrame>,
                           target_width: u32,
                           target_height: u32) {
        let video_path = video_path.to_owned();

        thread::spawn(move || {
           ffmpeg_next::init().ok();
            let mut ictx = ffmpeg_next::format::input(&video_path)
                .expect("Failed to open video file");

            // VIDEO SETUP
            let (v_index, v_time_base, mut v_decoder, mut scaler) = {
                let v_stream = ictx.streams().best(ffmpeg_next::media::Type::Video)
                    .expect("Failed to get video stream");

                let v_index = v_stream.index();
                let v_time_base = v_stream.time_base();

                let v_context = ffmpeg_next::codec::context::Context::from_parameters(
                    v_stream.parameters())
                    .expect("Failed to create video codec context");
                let mut v_decoder = v_context.decoder().video().unwrap();

                let scaler = ffmpeg_next::software::scaling::Context::get(
                    v_decoder.format(),
                    v_decoder.width(),
                    v_decoder.height(),
                    ffmpeg_next::format::Pixel::RGBA,
                    target_width,
                    target_height,
                    ffmpeg_next::software::scaling::flag::Flags::BILINEAR,
                ).unwrap();

                (v_index, v_time_base, v_decoder, scaler)
            };

            // AUDIO SETUP
            let (a_index, a_time_base, mut a_decoder, mut resampler) = {
                let a_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio)
                    .expect("Failed to get audio stream");

                let a_index = a_stream.index();
                let a_time_base = a_stream.time_base();

                let a_context = ffmpeg_next::codec::context::Context::from_parameters(a_stream.parameters())
                    .expect("Failed to create audio codec context");

                let mut a_decoder = a_context.decoder().audio().unwrap();

                let mut resampler = ffmpeg_next::software::resampling::Context::get(
                    a_decoder.format(),
                    a_decoder.channel_layout(),
                    a_decoder.rate(),
                    ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
                    ffmpeg_next::channel_layout::ChannelLayout::STEREO,
                    44100,
                ).expect("Failed to create audio resampler");

                (a_index, a_time_base, a_decoder, resampler)
            };




            // Unified LOOP to read packets and decode
            for (stream, packet) in ictx.packets() {
                if stream.index() == v_index {
                    // Handle video packet
                    v_decoder.send_packet(&packet).ok();
                    let mut frame = ffmpeg_next::util::frame::Video::empty();

                    while v_decoder.receive_frame(&mut frame).is_ok() {
                        let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                        scaler.run(&frame, &mut rgb_frame).ok();

                        // Convert FFmpeg PTS → seconds
                        let pts = frame
                            .pts()
                            .unwrap_or(0) as f64
                            * f64::from(v_time_base);

                        let video_data = rgb_frame.data(0).to_vec();

                        if v_sender.send(VideoFrame { pts, video_data }).is_err() {
                            return;
                        }
                    }
                } else if stream.index() == a_index {
                    // Handle audio packet
                    a_decoder.send_packet(&packet).ok();
                    let mut frame = ffmpeg_next::util::frame::Audio::empty();
                    while a_decoder.receive_frame(&mut frame).is_ok() {
                        let mut resampled_frame = ffmpeg_next::util::frame::audio::Audio::empty();

                        // Run resampler
                        resampler.run(&frame, &mut resampled_frame).ok();

                        let pts = frame
                            .pts()
                            .unwrap_or(0) as f64
                            * f64::from(stream.time_base());

                        // In 'Packed' format, all samples are in data(0)
                        let data = resampled_frame.data(0);

                        // Convert the raw byte slice intoo Vec<f32>
                        let samples: Vec<f32> = data
                            .chunks_exact(4)
                            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                            .collect();

                        // Send samples to main thread
                        if a_sender.send(AudioFrame {pts, samples}).is_err() { return; }
                    }
                }
            }

            // Potential refactor into "process video frame" and "process audio frame" functions
            // Flush decoders

            // Video flush
            v_decoder.send_eof().ok();
            let mut v_frame = ffmpeg_next::util::frame::Video::empty();
            while v_decoder.receive_frame(&mut v_frame).is_ok() {
                let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                scaler.run(&v_frame, &mut rgb_frame).ok();

                let pts = v_frame
                    .pts()
                    .unwrap_or(0) as f64
                    * f64::from(v_time_base);

                if v_sender.send(VideoFrame {
                    pts,
                    video_data: rgb_frame.data(0).to_vec(),
                }).is_err() { return; }
            }

            // Audio flush
            a_decoder.send_eof().ok();
            let mut a_frame = ffmpeg_next::util::frame::Audio::empty();
            while a_decoder.receive_frame(&mut a_frame).is_ok() {
                let mut resampled_frame = ffmpeg_next::util::frame::audio::Audio::empty();
                resampler.run(&a_frame, &mut resampled_frame).ok();


                let pts = a_frame
                    .pts()
                    .unwrap_or(0) as f64
                    * f64::from(a_time_base);

                let data = resampled_frame.data(0);
                let samples: Vec<f32> = data
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                if a_sender.send(AudioFrame {pts, samples}).is_err() { return};
            }
        });
    }

    // Sets up CPAL audio output stream with given sample receiver
    // Starts an audio output stream on the default output device
    fn setup_audio(receiver: Receiver<AudioFrame>, clock: Arc<AudioClock>) -> cpal::Stream {
        use cpal::traits::DeviceTrait;

        // Get platform default audio backend and default output device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device found");

        // Query and select supported stream config
        let supported_configs = device
            .supported_output_configs()
            .expect("Failed to query supported output configs");

        // Prefer stereo otherwise accept any config
        let mut best: Option<cpal::SupportedStreamConfig> = None;
        let mut best_is_stereo = false;

        for cfg in supported_configs {
            let is_stereo = cfg.channels() == 2;

            // Only replace current best if:
            // we dont have one yet, or
            // current is not stereo and this one is
            if best.is_none() || (is_stereo && !best_is_stereo) {
                // Choose 44100 if allowed, otherwise clamp
                let target_sr = 44_100u32;
                let min_sr = cfg.min_sample_rate();
                let max_sr = cfg.max_sample_rate();
                let chosen_sr = target_sr.clamp(min_sr, max_sr);

                let chosen = cfg.with_sample_rate(chosen_sr.into());

                best = Some(chosen);
                best_is_stereo = is_stereo;

                // Exit early if we found stereo
                if best_is_stereo { break; }
            }
        }

        let supported = best.expect("No supported stereo output config found");
        let config: cpal::StreamConfig = supported.clone().into();
        let sample_format = supported.sample_format();

        // Keep track of samples across callbacks
        // FIFO queue to hold audio samples
        // Crucial for real-time audio. The audio hardware calls back periodically asking for more samples.
        // Queueing decouples the production of samples from their consumption.
        let mut sample_queue: VecDeque<f32> = VecDeque::new();
        let err_fn = |err| eprintln!("Audio stream error: {}", err);

        // Build and run the output stream based on sample format
        match sample_format {
            cpal::SampleFormat::F32 => {
                let stream = device
                    .build_output_stream(
                        &config,
                        {
                            let receiver = receiver.clone();
                            let clock = Arc::clone(&clock);
                            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                while let Ok(chunk) = receiver.try_recv() {
                                    sample_queue.extend(chunk.samples);
                                }

                                for s in data.iter_mut() {
                                    *s = sample_queue.pop_front().unwrap_or(0.0);
                                    clock.samples_played.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        },
                        err_fn,
                        None,
                    ).expect("Failed to build audio stream");
                stream.play().expect("Failed to play audio stream");
                stream
            }

            cpal::SampleFormat::I16 => {
                let stream = device
                    .build_output_stream(
                        &config,
                        {
                            let receiver = receiver.clone();
                            let clock = Arc::clone(&clock);
                            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                                while let Ok(chunk) = receiver.try_recv() {
                                    sample_queue.extend(chunk.samples);
                                }

                                for s in data.iter_mut() {
                                    let f = sample_queue.pop_front()
                                        .unwrap_or(0.0)
                                        .clamp(-1.0, 1.0);
                                    *s = (f * i16::MAX as f32) as i16;
                                    clock.samples_played.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        },
                        err_fn,
                        None,
                    ).expect("Failed to build audio stream");
                stream.play().expect("Failed to play audio stream");
                stream
            }

            cpal::SampleFormat::U16 => {
                let stream = device
                    .build_output_stream(
                        &config,
                        {
                            let receiver = receiver.clone();
                            let clock = Arc::clone(&clock);
                            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                                while let Ok(chunk) = receiver.try_recv() {
                                    sample_queue.extend(chunk.samples);
                                }
                                for s in data.iter_mut() {
                                    let f = sample_queue.pop_front()
                                        .unwrap_or(0.0)
                                        .clamp(-1.0, 1.0);
                                    *s = (((f + 1.0) * 0.5) * u16::MAX as f32) as u16;
                                    clock.samples_played.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        },
                        err_fn,
                        None,
                    ).expect("Failed to build audio stream");
                stream.play().expect("Failed to play audio stream");
                stream
            }

            cpal::SampleFormat::I32 => {
                let stream = device.build_output_stream(
                    &config,
                    {
                        let receiver = receiver.clone();
                        let clock = Arc::clone(&clock);
                        move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                            while let Ok(chunk) = receiver.try_recv() {
                                sample_queue.extend(chunk.samples);
                            }

                            for s in data.iter_mut() {
                                let f = sample_queue.pop_front().unwrap_or(0.0).clamp(-1.0, 1.0);
                                *s = (f * i32::MAX as f32) as i32;
                                clock.samples_played.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    },
                    err_fn,
                    None,
                ).expect("Failed to build audio stream");
                stream.play().expect("Failed to play audio stream");
                stream
            }

            _ => panic!("Unsupported sample format: {:?}", sample_format),
        }
    }

}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            pixels: None,
            frame_source: None,
            audio_receiver: None,
            audio_stream: None,
            audio_clock: None,
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

        // Extract info for the Window and App state
        let video_params = video_stream.parameters();
        let decoder_ctx = ffmpeg_next::codec::context::Context::from_parameters(video_params).unwrap();
        let video_decoder = decoder_ctx.decoder().video()
            .expect("Failed to create video decoder for metadata");

        // Store video dimensions
        self.video_width = video_decoder.width();
        self.video_height = video_decoder.height();

        // Calculate FPS from time base
        let fps_ratio = video_stream.avg_frame_rate();
        let fps = fps_ratio.numerator() as f32 / fps_ratio.denominator() as f32;

        // Setup Window and Pixels
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

        // Setup channels (One for video, one for audio)
        let (v_sender, v_receiver) = bounded::<VideoFrame>(FRAME_BUFFER_SIZE);
        let (a_sender, a_receiver) = bounded::<AudioFrame>(200);

        let clock = Arc::new(AudioClock::new(44_100));

        // Initialize CPAL audio stream
        let stream = Self::setup_audio(a_receiver, Arc::clone(&clock));
        self.audio_clock = Some(clock);
        self.audio_stream = Some(stream);

        // Start worker thread to decode video frames
        Self::spawn_demux_decode_thread(video_path, v_sender, a_sender, self.video_width, self.video_height);

        // Initialzie frame source with video config
        self.frame_source = Some(FrameSource::Video {
            frame_receiver: v_receiver,
            frame_buffer: VecDeque::with_capacity(FRAME_BUFFER_SIZE),
            current_frame: vec![0; (self.video_width * self.video_height * 4) as usize], // Black initial frame
            fps,  // Adjust
            last_frame_time: std::time::Instant::now(),
            start_instant: std::time::Instant::now(),
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
                let frame_data = Self::current_frame(&mut self.frame_source, self.audio_clock.as_ref());

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
