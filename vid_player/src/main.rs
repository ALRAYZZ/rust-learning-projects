use std::collections::VecDeque;
use std::thread;
use crossbeam_channel::{bounded, Receiver, Sender};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use std::sync::{Arc, Mutex};
use winit::dpi::LogicalSize;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const VIDEO_BUFFER_FRAMES: usize = 60; // Buffer up to 60 video frames (~2 seconds at 30fps)
const AUDIO_CHANNEL_SIZE: usize = 100; // Channel can hold 100 audio chunks

// Video frame with timestamp
struct VideoFrame {
    pts: f64,
    data: Vec<u8>,
}

// Audio chunk with timestamp
struct AudioChunk {
    pts: f64,
    samples: Vec<f32>, // Stereo interleaved
}

// Thread-safe audio clock tracking playback position
struct AudioClock {
    samples_played: AtomicU64,
    sample_rate: u32,
}

impl AudioClock {
    fn new(sample_rate: u32) -> Self {
        Self {
            samples_played: AtomicU64::new(0),
            sample_rate,
        }
    }

    fn current_time(&self) -> f64 {
        self.samples_played.load(Ordering::Acquire) as f64 / self.sample_rate as f64
    }

    fn advance(&self, frames: u64) {
        self.samples_played.fetch_add(frames, Ordering::Release);
    }
}

// Blocking ring buffer for audio samples
struct AudioRingBuffer {
    buffer: Vec<f32>,
    read_pos: usize,
    write_pos: usize,
    filled: usize,
}

impl AudioRingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            read_pos: 0,
            write_pos: 0,
            filled: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.buffer.len()
    }

    fn available(&self) -> usize {
        self.filled
    }

    fn free_space(&self) -> usize {
        self.capacity() - self.filled
    }

    // Write samples to ring buffer (blocks if not enough space)
    fn write(&mut self, samples: &[f32]) -> usize {
        let to_write = samples.len().min(self.free_space());

        for i in 0..to_write {
            self.buffer[self.write_pos] = samples[i];
            self.write_pos = (self.write_pos + 1) % self.capacity();
            self.filled += 1;
        }

        to_write
    }

    // Read samples from ring buffer
    fn read(&mut self, output: &mut [f32]) -> usize {
        let to_read = output.len().min(self.available());

        for i in 0..to_read {
            output[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity();
            self.filled -= 1;
        }

        // Fill remainder with silence
        for i in to_read..output.len() {
            output[i] = 0.0;
        }

        to_read
    }
}

// Separate thread for video decoding
fn spawn_video_decoder(
    video_path: &Path,
    sender: Sender<VideoFrame>,
    target_width: u32,
    target_height: u32,
) {
    let path = video_path.to_owned();

    thread::Builder::new()
        .name("video-decoder".to_string())
        .spawn(move || {
            ffmpeg_next::init().unwrap();

            let mut input_ctx = ffmpeg_next::format::input(&path)
                .expect("Failed to open video file");

            let video_stream = input_ctx
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .expect("No video stream");

            let video_idx = video_stream.index();
            let time_base = video_stream.time_base();

            let ctx = ffmpeg_next::codec::context::Context::from_parameters(
                video_stream.parameters()
            ).unwrap();
            let mut decoder = ctx.decoder().video().unwrap();

            let mut scaler = ffmpeg_next::software::scaling::Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                ffmpeg_next::format::Pixel::RGBA,
                target_width,
                target_height,
                ffmpeg_next::software::scaling::flag::Flags::BILINEAR,
            ).unwrap();

            // Demux and decode video packets
            for (stream, packet) in input_ctx.packets() {
                if stream.index() != video_idx {
                    continue;
                }

                if decoder.send_packet(&packet).is_err() {
                    continue;
                }

                let mut frame = ffmpeg_next::util::frame::Video::empty();
                while decoder.receive_frame(&mut frame).is_ok() {
                    let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                    if scaler.run(&frame, &mut rgb_frame).is_err() {
                        continue;
                    }

                    let pts = frame.pts().unwrap_or(0) as f64 * f64::from(time_base);
                    let data = extract_rgba_data(&rgb_frame, target_width, target_height);

                    // This blocks if channel is full (backpressure)
                    if sender.send(VideoFrame { pts, data }).is_err() {
                        return; // Receiver dropped
                    }
                }
            }

            // Drain decoder
            let _ = decoder.send_eof();
            let mut frame = ffmpeg_next::util::frame::Video::empty();
            while decoder.receive_frame(&mut frame).is_ok() {
                let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                if scaler.run(&frame, &mut rgb_frame).is_ok() {
                    let pts = frame.pts().unwrap_or(0) as f64 * f64::from(time_base);
                    let data = extract_rgba_data(&rgb_frame, target_width, target_height);
                    let _ = sender.send(VideoFrame { pts, data });
                }
            }
        })
        .expect("Failed to spawn video decoder thread");
}

// Separate thread for audio decoding
fn spawn_audio_decoder(
    video_path: &Path,
    sender: Sender<AudioChunk>,
    target_sample_rate: u32,
) {
    let path = video_path.to_owned();

    thread::Builder::new()
        .name("audio-decoder".to_string())
        .spawn(move || {
            ffmpeg_next::init().unwrap();

            let mut input_ctx = ffmpeg_next::format::input(&path)
                .expect("Failed to open audio file");

            let audio_stream = input_ctx
                .streams()
                .best(ffmpeg_next::media::Type::Audio)
                .expect("No audio stream");

            let audio_idx = audio_stream.index();
            let time_base = audio_stream.time_base();

            let ctx = ffmpeg_next::codec::context::Context::from_parameters(
                audio_stream.parameters()
            ).unwrap();
            let mut decoder = ctx.decoder().audio().unwrap();

            let mut resampler = ffmpeg_next::software::resampling::Context::get(
                decoder.format(),
                decoder.channel_layout(),
                decoder.rate(),
                ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
                ffmpeg_next::channel_layout::ChannelLayout::STEREO,
                target_sample_rate,
            ).unwrap();

            // Demux and decode audio packets
            for (stream, packet) in input_ctx.packets() {
                if stream.index() != audio_idx {
                    continue;
                }

                if decoder.send_packet(&packet).is_err() {
                    continue;
                }

                let mut frame = ffmpeg_next::util::frame::Audio::empty();
                while decoder.receive_frame(&mut frame).is_ok() {
                    let mut resampled = ffmpeg_next::util::frame::Audio::empty();
                    if resampler.run(&frame, &mut resampled).is_err() {
                        continue;
                    }

                    let pts = frame.pts().unwrap_or(0) as f64 * f64::from(time_base);

                    let sample_count = resampled.samples() * 2; // Stereo
                    let bytes = resampled.data(0);

                    if sample_count == 0 {
                        continue;
                    }

                    let samples: Vec<f32> = unsafe {
                        std::slice::from_raw_parts(
                            bytes.as_ptr() as *const f32,
                            sample_count
                        ).to_vec()
                    };

                    // This blocks if channel is full (backpressure)
                    if sender.send(AudioChunk { pts, samples }).is_err() {
                        return; // Receiver dropped
                    }
                }
            }

            // Drain decoder
            let _ = decoder.send_eof();
            let mut frame = ffmpeg_next::util::frame::Audio::empty();
            while decoder.receive_frame(&mut frame).is_ok() {
                let mut resampled = ffmpeg_next::util::frame::Audio::empty();
                if resampler.run(&frame, &mut resampled).is_ok() {
                    let pts = frame.pts().unwrap_or(0) as f64 * f64::from(time_base);
                    let sample_count = resampled.samples() * 2;
                    let bytes = resampled.data(0);

                    if sample_count > 0 {
                        let samples: Vec<f32> = unsafe {
                            std::slice::from_raw_parts(
                                bytes.as_ptr() as *const f32,
                                sample_count
                            ).to_vec()
                        };
                        let _ = sender.send(AudioChunk { pts, samples });
                    }
                }
            }
        })
        .expect("Failed to spawn audio decoder thread");
}

// Thread that fills ring buffer from decoded audio chunks
fn spawn_audio_buffer_filler(
    receiver: Receiver<AudioChunk>,
    ring_buffer: Arc<Mutex<AudioRingBuffer>>,
) {
    thread::Builder::new()
        .name("audio-filler".to_string())
        .spawn(move || {
            while let Ok(chunk) = receiver.recv() {
                // Write to ring buffer (will write as much as fits)
                let mut written = 0;
                while written < chunk.samples.len() {
                    if let Ok(mut buffer) = ring_buffer.lock() {
                        let n = buffer.write(&chunk.samples[written..]);
                        written += n;

                        if n == 0 {
                            drop(buffer);
                            // Buffer full, wait a bit
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                    }
                }
            }
        })
        .expect("Failed to spawn audio filler thread");
}

fn extract_rgba_data(frame: &ffmpeg_next::util::frame::Video, width: u32, height: u32) -> Vec<u8> {
    let stride = frame.stride(0);
    let src = frame.data(0);
    let row_bytes = width as usize * 4;
    let mut data = vec![0u8; row_bytes * height as usize];

    for y in 0..height as usize {
        let src_offset = y * stride;
        let dst_offset = y * row_bytes;
        data[dst_offset..dst_offset + row_bytes]
            .copy_from_slice(&src[src_offset..src_offset + row_bytes]);
    }

    data
}

struct App {
    window: Option<Arc<Box<dyn Window>>>,
    pixels: Option<Pixels<'static>>,

    // Video state
    video_receiver: Option<Receiver<VideoFrame>>,
    video_buffer: VecDeque<VideoFrame>,
    current_frame: Vec<u8>,

    // Audio state
    audio_stream: Option<cpal::Stream>,
    audio_clock: Arc<AudioClock>,

    // Dimensions
    width: u32,
    height: u32,

    // Playback time
    duration_secs: f64,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            video_receiver: None,
            video_buffer: VecDeque::with_capacity(VIDEO_BUFFER_FRAMES),
            current_frame: Vec::new(),
            audio_stream: None,
            audio_clock: Arc::new(AudioClock::new(48000)),
            width: 0,
            height: 0,
            duration_secs: 0.0,
        }
    }

    fn process_next_frame(&mut self) {
        let video_receiver = match self.video_receiver.as_ref() {
            Some(r) => r,
            None => return,
        };

        // Refill buffer from decoder
        while self.video_buffer.len() < VIDEO_BUFFER_FRAMES {
            match video_receiver.try_recv() {
                Ok(frame) => self.video_buffer.push_back(frame),
                Err(_) => break,
            }
        }

        // Get current audio time
        let audio_time = self.audio_clock.current_time();

        // Display the latest frame whose PTS <= audio time
        while let Some(front) = self.video_buffer.front() {
            if front.pts <= audio_time {
                let frame = self.video_buffer.pop_front().unwrap();
                self.current_frame = frame.data;
            } else {
                break; // Future frame, wait
            }
        }
    }

    fn current_time_secs(&self) -> f64 {
        self.audio_clock.current_time()
    }

    // Calculate playback progress (0.0 to 1.0)
    fn playback_progress(&self) -> f64 {
        if self.duration_secs <= 0.0 {
            return 0.0;
        }

        let progress = self.current_time_secs() / self.duration_secs;
        progress.clamp(0.0, 1.0)
    }

    fn draw_rect(
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        x: u32,
        y: u32,
        rect_width: u32,
        rect_height: u32,
        color: [u8; 4],
    ) {
        let frame_width = frame_width as usize;
        let frame_height = frame_height as usize;

        // Draw solid rectangle into frame buffer
        for yy in y..(y + rect_height).min(frame_height as u32) {
            for xx in x..(x + rect_width).min(frame_width as u32) {
                let idx = ((yy as usize * frame_width) + xx as usize) * 4;
                frame[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _event_loop: &dyn ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let video_path = Path::new("sample_video.mp4");

        // Get video metadata
        ffmpeg_next::init().ok();
        let input_ctx = ffmpeg_next::format::input(video_path)
            .expect("Failed to open video");

        let duration = input_ctx.duration();

        if duration > 0 {
            self.duration_secs = duration as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64;
        } else {
            self.duration_secs = 0.0;
        }

        let video_stream = input_ctx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .expect("No video stream");

        let params = video_stream.parameters();
        let ctx = ffmpeg_next::codec::context::Context::from_parameters(params).unwrap();
        let decoder = ctx.decoder().video().unwrap();

        self.width = decoder.width();
        self.height = decoder.height();

        // Setup audio
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No audio device");

        let config = device.default_output_config().expect("No output config");
        let sample_rate = config.sample_rate();
        let sample_format = config.sample_format();

        self.audio_clock = Arc::new(AudioClock::new(sample_rate));

        // Create ring buffer (2 seconds of stereo audio)
        let ring_capacity = sample_rate as usize * 2 * 2;
        let ring_buffer = Arc::new(Mutex::new(AudioRingBuffer::new(ring_capacity)));

        // Setup channels
        let (video_tx, video_rx) = bounded(VIDEO_BUFFER_FRAMES);
        let (audio_tx, audio_rx) = bounded(AUDIO_CHANNEL_SIZE);

        // Start decoder threads
        spawn_video_decoder(video_path, video_tx, self.width, self.height);
        spawn_audio_decoder(video_path, audio_tx, sample_rate);

        // Start audio buffer filler
        spawn_audio_buffer_filler(audio_rx, Arc::clone(&ring_buffer));

        // Build audio stream
        let stream = build_audio_stream(
            &device,
            &config.into(),
            sample_format,
            Arc::clone(&ring_buffer),
            Arc::clone(&self.audio_clock),
        );

        stream.play().expect("Failed to play audio");

        self.video_receiver = Some(video_rx);
        self.audio_stream = Some(stream);
        self.current_frame = vec![0; (self.width * self.height * 4) as usize];

        // Create window
        let attrs = WindowAttributes::default()
            .with_surface_size(LogicalSize::new(self.width, self.height))
            .with_title("Rust Video Player");

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let size = window.surface_size();

        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = Pixels::new(self.width, self.height, surface)
            .expect("Failed to create pixels");

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::SurfaceResized(new_size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    let _ = pixels.resize_surface(new_size.width, new_size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                // Update frame state
                self.process_next_frame();

                let progress = self.playback_progress();
                println!("Playback progress: {:.2}%", progress * 100.0);

                // Get dimensions
                let w = self.width;
                let h = self.height;
                let bar_height: u32 = 8;
                let y = h.saturating_sub(bar_height);
                let filled_width = (w as f64 * progress) as u32;

                if let Some(pixels) = self.pixels.as_mut() {
                    let frame = pixels.frame_mut();

                    // Copy the video frame
                    if !self.current_frame.is_empty() {
                        frame.copy_from_slice(&self.current_frame);
                    }

                    // Draw the progress bar on top
                    Self::draw_rect(frame, w, h, 0, y, w, bar_height, [50, 50, 50, 255]);
                    Self::draw_rect(frame, w, h, 0, y, filled_width, bar_height, [0, 200, 0, 255]);

                    // Render to screen
                    if pixels.render().is_err() {
                        event_loop.exit();
                        return;
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn build_audio_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    ring_buffer: Arc<Mutex<AudioRingBuffer>>,
    clock: Arc<AudioClock>,
) -> cpal::Stream {
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("Audio error: {}", err);

    match format {
        cpal::SampleFormat::F32 => {
            device.build_output_stream(
                config,
                move |data: &mut [f32], _| {
                    let frames = data.len() / channels;
                    let mut stereo_data = vec![0.0f32; frames * 2];

                    if let Ok(mut buffer) = ring_buffer.lock() {
                        buffer.read(&mut stereo_data);
                    }

                    // Convert stereo to output channels
                    for frame in 0..frames {
                        let l = stereo_data[frame * 2];
                        let r = stereo_data[frame * 2 + 1];

                        for ch in 0..channels {
                            data[frame * channels + ch] = if ch % 2 == 0 { l } else { r };
                        }
                    }

                    clock.advance(frames as u64);
                },
                err_fn,
                None,
            ).expect("Failed to build audio stream")
        }
        cpal::SampleFormat::I32 => {
            device.build_output_stream(
                config,
                move |data: &mut [i32], _| {
                    let frames = data.len() / channels;
                    let mut stereo_data = vec![0.0f32; frames * 2];

                    if let Ok(mut buffer) = ring_buffer.lock() {
                        buffer.read(&mut stereo_data);
                    }

                    for frame in 0..frames {
                        let l = stereo_data[frame * 2];
                        let r = stereo_data[frame * 2 + 1];

                        for ch in 0..channels {
                            let sample = if ch % 2 == 0 { l } else { r };
                            data[frame * channels + ch] =
                                (sample.clamp(-1.0, 1.0) * i32::MAX as f32) as i32;
                        }
                    }

                    clock.advance(frames as u64);
                },
                err_fn,
                None,
            ).expect("Failed to build audio stream")
        }
        _ => panic!("Unsupported sample format"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let app = App::new();
    event_loop.run_app(app)?;

    Ok(())
}