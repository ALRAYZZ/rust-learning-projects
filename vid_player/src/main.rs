use pixels::{Pixels, SurfaceTexture};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// Constants for the window size
const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;


fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rust Video Player (raw)")
        .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();
}
