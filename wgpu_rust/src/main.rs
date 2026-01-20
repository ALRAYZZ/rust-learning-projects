use std::sync::Arc;
use winit::{
    application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey}, window::Window
};


// If compiling to WebAssembly, include the wasm_bindgen crate
// wasm_is used for interoperability between Rust and JavaScript
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct State {
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self {
            window,
        })
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // Handle window resizing here
    }

    pub fn render(&mut self) {
        self.window.request_redraw();
    }
}

fn main() {
    println!("Test wgpu with winit!");
}
