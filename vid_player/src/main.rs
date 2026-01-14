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
use std::mem::size_of;
use ffmpeg_next::codec::Audio;
use ffmpeg_next::Packet;

const FRAME_BUFFER_SIZE: usize = 30; // Buffer 30 frames ahead to avoid stutter
const AUDIO_BUFFER_SIZE: usize = 1000; // Buffer 1000 audio frames ahead

// Struct to hold video frame data and timestamp
struct VideoFrame {
    pts: f64, // Timestamp in seconds
    video_data: Vec<u8>, // RGBA pixel data
}

struct AudioFrame {
    pts: f64,
    samples: Vec<f32>, // Interleaved samples
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
    }
}


struct App {
    window: Option<Arc<Box<dyn Window>>>, // We use Arc because window is shared with Pixels and App
    pixels: Option<Pixels<'static>>,
    frame_source: Option<FrameSource>, // Frame source (image or video)
    audio_stream: Option<cpal::Stream>,
    audio_clock: Option<Arc<AudioClock>>,
    video_width: u32,
    video_height: u32,
}

fn video_frame_to_rgba_packed(
    rgb_frame: &ffmpeg_next::util::frame::Video,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let stride = rgb_frame.stride(0) as usize;
    let src = rgb_frame.data(0);

    let row_bytes = (width as usize) * 4;
    let mut out = vec![0u8; row_bytes * (height as usize)];

    for y in 0..(height as usize) {
        let src_off = y * stride;
        let dst_off = y * row_bytes;
        out[dst_off..dst_off + row_bytes]
            .copy_from_slice(&src[src_off..src_off + row_bytes]);
    }
    out
}

impl App {
    // Determines current frame to display based on frame source
    // Static: Returns the same frame
    // Video: Advances frame when enough time has passed based on fps
    fn current_frame<'a>(frame_source: &'a mut Option<FrameSource>, audio_clock: Option<&Arc<AudioClock>>) -> Option<&'a [u8]> {
        match frame_source.as_mut()? {
            FrameSource::StaticImage {frame} => {
                Some(frame)
            }

            FrameSource::Video {
                frame_receiver,
                frame_buffer,
                current_frame,
            } => {
                // Refill buffer from decoder thread if space available
                while frame_buffer.len() < FRAME_BUFFER_SIZE {
                    if let Ok(frame) = frame_receiver.try_recv() {
                        frame_buffer.push_back(frame);
                    } else {
                        break;
                    }
                }

                let audio_time = audio_clock
                    .as_ref()
                    .map(|c| c.time())
                    .unwrap_or(0.0);

                // Select latest frame whose pts <= audio_time
                while let Some(front) = frame_buffer.front() {
                    if front.pts <= audio_time {
                        let frame = frame_buffer.pop_front().unwrap();
                        *current_frame = frame.video_data;
                    } else {
                        break;
                    }
                }

                Some(current_frame)
            }
        }
    }

    // Spawns background thread that demuxes and decodes video and audio packets.
    // This architecture solves lifetime issues by:
    // 1. Creating all FFmpeg objects in worker thread
    // 2. Decoding frames sequentially with packed feeding
    // 3. Converting YUV -> RGBA using FFmpeg scaler
    // 4. Sending fully-owned Vec<u8> through channel
    fn spawn_demux_decode_thread(
        video_path: &Path,
        v_sender: Sender<VideoFrame>,
        a_sender: Sender<AudioFrame>,
        target_width: u32,
        target_height: u32,
        sample_rate: u32,
        target_channels: u16,
    ) {
        let video_path = video_path.to_owned();

        // Channels for packets
        let (video_packet_sender, video_packet_receiver) = bounded::<Option<Packet>>(100);
        let (audio_packet_sender, audio_packet_receiver) = bounded::<Option<Packet>>(100);

        // Demux thread: Reads packets and dispatches to video/audio decoders
        let demux_path= video_path.clone();
        let demux_handle = thread::spawn(move || {
            ffmpeg_next::init().ok();
            let mut ictx = ffmpeg_next::format::input(&demux_path)
                .expect("Failed to open video file");

            let video_stream = ictx.streams().best(ffmpeg_next::media::Type::Video)
                .expect("Failed to get video stream");
            let audio_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio)
                .expect("Failed to get audio stream");

            let v_index = video_stream.index();
            let a_index = audio_stream.index();

            // Loop over packets
            for (stream, packet) in ictx.packets() {
                if stream.index() == v_index {
                    if video_packet_sender.send(Some(packet)).is_err() {
                        return;
                    }
                } else if stream.index() == a_index {
                    if audio_packet_sender.send(Some(packet)).is_err() {
                        return;
                    }
                }
            }

            // Send EOF signals to decoders
            let _ = video_packet_sender.send(None);
            let _ = audio_packet_sender.send(None);
        });

        // Video decode thread
        let video_path_clone = video_path.clone();
        let v_sender_clone = v_sender.clone();
        let video_decode_handle = thread::spawn(move || {
            ffmpeg_next::init().ok();
            let ictx = ffmpeg_next::format::input(&video_path_clone)
                .expect("Failed to open video file for video decoding");

            let v_stream = ictx.streams().best(ffmpeg_next::media::Type::Video)
                .expect("No video stream found");
            let v_time_base = v_stream.time_base();

            let v_context = ffmpeg_next::codec::context::Context::from_parameters(v_stream.parameters())
                .expect("Failed to create video codec context");
            let mut v_decoder = v_context.decoder().video().unwrap();

            let mut scaler = ffmpeg_next::software::scaling::Context::get(
                v_decoder.format(),
                v_decoder.width(),
                v_decoder.height(),
                ffmpeg_next::format::Pixel::RGBA,
                target_width,
                target_height,
                ffmpeg_next::software::scaling::flag::Flags::BILINEAR,
            ).unwrap();

            loop {
                let packet_opt = video_packet_receiver.recv().ok();

                let is_eof = matches!(packet_opt, Some(None) | None);
                match packet_opt {
                    Some(Some(packet)) => {
                        v_decoder.send_packet(&packet).ok();
                    }
                    Some(None) | None =>  {
                        v_decoder.send_eof().ok();
                    }
                }

                let mut frame = ffmpeg_next::util::frame::Video::empty();
                while v_decoder.receive_frame(&mut frame).is_ok() {
                    let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                    scaler.run(&frame, &mut rgb_frame).ok();

                    let pts = frame
                        .pts()
                        .unwrap_or(0) as f64
                        * f64::from(v_time_base);

                    let video_data = video_frame_to_rgba_packed(&rgb_frame, target_width, target_height);

                    if v_sender_clone.send(VideoFrame { pts, video_data }).is_err() {
                        return;
                    }
                }

                if is_eof {
                    break;
                }
            }
        });

        // Audio decode thread
        let audio_path_clone = video_path.clone();
        let a_sender_clone = a_sender.clone();
        let audio_decode_handle = thread::spawn(move || {
            ffmpeg_next::init().ok();
            let ictx = ffmpeg_next::format::input(&audio_path_clone)
                .expect("Failed to open video file for audio decoding");

            let a_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio)
                .expect("No audio stream found");
            let a_time_base = a_stream.time_base();

            let a_context = ffmpeg_next::codec::context::Context::from_parameters(a_stream.parameters())
                .expect("Failed to create audio codec context");

            let mut a_decoder = a_context.decoder().audio().unwrap();

            let target_layout = ffmpeg_next::channel_layout::ChannelLayout::default(target_channels as i32);

            let mut resampler = ffmpeg_next::software::resampling::Context::get(
                a_decoder.format(),
                a_decoder.channel_layout(),
                a_decoder.rate(),
                ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
                target_layout,
                sample_rate,
            ).expect("Failed to create audio resampler");

            loop {
                let packet_opt = audio_packet_receiver.recv().ok();

                // Capture EOF state
                let is_eof = matches!(packet_opt, Some(None) | None);

                match packet_opt {
                    Some(Some(packet)) => {
                        a_decoder.send_packet(&packet).ok();
                    }
                    Some(None) | None =>  {
                        a_decoder.send_eof().ok();
                    }
                }

                let mut frame = ffmpeg_next::util::frame::Audio::empty();
                while a_decoder.receive_frame(&mut frame).is_ok() {
                    let mut out = ffmpeg_next::util::frame::audio::Audio::empty();

                    if resampler.run(&frame, &mut out).is_err() {
                        continue;
                    }

                    let pts = frame.pts().unwrap_or(0) as f64 * f64::from(a_time_base);

                    let channels = target_channels as usize;
                    let total_f32 = out.samples() * channels;

                    let bytes = out.data(0);
                    let need_bytes = total_f32 * size_of::<f32>();

                    if bytes.len() < need_bytes {
                        continue;
                    }

                    let mut samples = vec![0f32; total_f32];
                    let src = &bytes[..need_bytes];

                    for (i, chunk) in src.chunks_exact(4).take(total_f32).enumerate() {
                        samples[i] = f32::from_ne_bytes(chunk.try_into().unwrap());
                    }

                    if a_sender_clone.send(AudioFrame { pts, samples }).is_err() {
                        return;
                    }
                }

                if is_eof {
                    // Drain resampler
                    loop {
                        let mut out = ffmpeg_next::util::frame::audio::Audio::empty();
                        let res = resampler
                            .run(&ffmpeg_next::util::frame::audio::Audio::empty(), &mut out);
                        if let Ok(samples_produced) = res {
                            if samples_produced.is_none() {
                                break;
                            }
                        } else {
                            break;
                        }

                        let pts = 0.0; // Flush pts

                        let channels = target_channels as usize;
                        let total_f32 = out.samples() * channels;

                        let bytes = out.data(0);
                        let need_bytes = total_f32 * size_of::<f32>();

                        if bytes.len() < need_bytes {
                            continue;
                        }

                        let mut samples = vec![0f32; total_f32];
                        let src = &bytes[..need_bytes];

                        for (i, chunk) in src.chunks_exact(4).take(total_f32).enumerate() {
                            samples[i] = f32::from_ne_bytes(chunk.try_into().unwrap());
                        }

                        if a_sender_clone.send(AudioFrame { pts, samples }).is_err() {
                            return;
                        }
                    }
                    break;
                }
            }
        });

        // Keep handles alive
        let _ = (demux_handle, video_decode_handle, audio_decode_handle);
    }

    fn get_audio_config() -> (cpal::Device, cpal::StreamConfig, cpal::SampleFormat) {
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
                if best_is_stereo { break; }
            }
        }

        let supported = best.expect("No supported output config found");
        let mut config: cpal::StreamConfig = supported.clone().into();

        // Request fixed buffer size for lower latency
        config.buffer_size = cpal::BufferSize::Fixed(1024);

        println!(
            "Audio config: Channels={}, SampleRate={}, Format={:?}",
            config.channels,
            config.sample_rate,
            supported.sample_format());

        (device, config, supported.sample_format())
    }

    fn build_audio_stream(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        sample_format: cpal::SampleFormat,
        receiver: Receiver<AudioFrame>,
        clock: Arc<AudioClock>,
    ) -> cpal::Stream  {
        let channels_u64 = config.channels as u64;
        let err_fn = |err| eprintln!("Audio stream error: {}", err);

        match sample_format {
            cpal::SampleFormat::F32 => {
                let mut sample_queue: VecDeque<f32> = VecDeque::new();
                let receiver = receiver.clone();
                let clock = Arc::clone(&clock);

                let stream = device.build_output_stream(
                    config,
                    move |data: &mut [f32], _| {
                        while let Ok(chunk) = receiver.try_recv() {
                            sample_queue.extend(chunk.samples);
                        }

                        let mut underflow_count = 0u32;
                        for s in data.iter_mut() {
                            if let Some(v) = sample_queue.pop_front() {
                                *s = v;
                            } else {
                                *s = 0.0;
                                underflow_count += 1;
                            }
                        }
                        if underflow_count > 0 {
                            eprintln!("Audio underflow: {} samples", underflow_count);
                        }

                        // Advance audio clock by number of frames written
                        let frames_written = (data.len() as u64) / channels_u64;

                        clock.samples_played.fetch_add(frames_written, Ordering::Relaxed);

                    },
                    err_fn,
                    None,
                ).expect("Failed to build audio stream");

                stream.play().expect("Failed to play audio stream");
                stream
            }

            cpal::SampleFormat::I16 => {
                let mut sample_queue: VecDeque<f32> = VecDeque::new();
                let receiver = receiver.clone();
                let clock = Arc::clone(&clock);

                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            while let Ok(chunk) = receiver.try_recv() {
                                sample_queue.extend(chunk.samples);
                            }

                            for s in data.iter_mut() {
                                let f = sample_queue
                                    .pop_front()
                                    .unwrap_or(0.0)
                                    .clamp(-1.0, 1.0);
                                *s = (f * i16::MAX as f32) as i16;
                            }

                            let frames_written = (data.len() as u64) / channels_u64;
                            clock
                                .samples_played
                                .fetch_add(frames_written, Ordering::Relaxed);
                        },
                        err_fn,
                        None,
                    )
                    .expect("Failed to build audio stream");

                stream.play().expect("Failed to play audio stream");
                stream
            }

            cpal::SampleFormat::U16 => {
                let mut sample_queue: VecDeque<f32> = VecDeque::new();
                let receiver = receiver.clone();
                let clock = Arc::clone(&clock);

                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _| {
                            while let Ok(chunk) = receiver.try_recv() {
                                sample_queue.extend(chunk.samples);
                            }

                            for s in data.iter_mut() {
                                let f = sample_queue
                                    .pop_front()
                                    .unwrap_or(0.0)
                                    .clamp(-1.0, 1.0);
                                *s = (((f + 1.0) * 0.5) * u16::MAX as f32) as u16;
                            }

                            let frames_written = (data.len() as u64) / channels_u64;
                            clock
                                .samples_played
                                .fetch_add(frames_written, Ordering::Relaxed);
                        },
                        err_fn,
                        None,
                    )
                    .expect("Failed to build audio stream");

                stream.play().expect("Failed to play audio stream");
                stream
            }

            cpal::SampleFormat::I32 => {
                let mut sample_queue: VecDeque<f32> = VecDeque::new();
                let receiver = receiver.clone();
                let clock = Arc::clone(&clock);

                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i32], _| {
                            while let Ok(chunk) = receiver.try_recv() {
                                sample_queue.extend(chunk.samples);
                            }

                            let mut underflow_count = 0u32;
                            for s in data.iter_mut() {
                                if let Some(f) = sample_queue.pop_front() {
                                    let f_clamped = f.clamp(-1.0, 1.0);
                                    *s = (f_clamped * i32::MAX as f32) as i32;
                                } else {
                                    *s = 0;
                                    underflow_count += 1;
                                }
                            }
                            if underflow_count > 0 {
                                eprintln!("Audio underflow: {} samples", underflow_count);
                            }

                            let frames_written = (data.len() as u64) / channels_u64;
                            clock
                                .samples_played
                                .fetch_add(frames_written, Ordering::Relaxed);
                        },
                        err_fn,
                        None,
                    )
                    .expect("Failed to build audio stream");

                stream.play().expect("Failed to play audio stream");
                stream
            },
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

        // Determine audio config
        let (audio_device, audio_config, audio_format) = Self::get_audio_config();
        let sample_rate = audio_config.sample_rate;

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

        let audio_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio).unwrap();
        let audio_params = audio_stream.parameters();
        let audio_ctx = ffmpeg_next::codec::context::Context::from_parameters(audio_params).unwrap();
        let audio_decoder = audio_ctx.decoder().audio().unwrap();
        println!("Input audio: Channels={}, SampleRate={}", audio_decoder.channels(), audio_decoder.rate());

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
        let (a_sender, a_receiver) = bounded::<AudioFrame>(1000);

        let clock = Arc::new(AudioClock::new(sample_rate));

        // Initialize CPAL audio stream
        let stream = Self::build_audio_stream(
            &audio_device,
            &audio_config,
            audio_format,
            a_receiver,
            Arc::clone(&clock));

        self.audio_stream = Some(stream);
        self.audio_clock = Some(clock);

        // Start worker thread to decode video frames
        Self::spawn_demux_decode_thread(
            video_path, v_sender, a_sender,
            self.video_width, self.video_height,
            sample_rate, audio_config.channels);

        // Initialzie frame source with video config
        self.frame_source = Some(FrameSource::Video {
            frame_receiver: v_receiver,
            frame_buffer: VecDeque::with_capacity(FRAME_BUFFER_SIZE),
            current_frame: vec![0; (self.video_width * self.video_height * 4) as usize], // Black initial frame
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
