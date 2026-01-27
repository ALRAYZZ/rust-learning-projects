use std::sync::Arc;
use winit::{
    application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop},
    keyboard::PhysicalKey, window::Window
};

use crate::{state::State, input::InputHandler};
use crate::input::InputAction;

// THE ORCHESTRATOR
// Manages OS lifecycle. Speaks to winit to create windows, handle events, etc
// Does not care about rendering, but that there is a window to render to
pub struct App {
    state: Option<State>,
}

impl App  {
    pub fn new() -> Self {
        Self {
            state: None,
        }
    }
}

// ApplicationHandler is a trait that allows us to handle application-level events
// like window creation, user events, and window events
// Brain of the app, OS to app interface. Manages window lifecycle and events.
// Servers as the controller that tells the WGPU engine when to update and render and redraw
impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)] // To avoid warnings on non-wasm32 targets
        let mut window_attributes = Window::default_attributes();

        // Create the window
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // If we are not on web use pollster
        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }

    // Handle window events like resize, close, redraw, keyboard input
    // called by the event loop when such events occur
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure surface if lost
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::CursorMoved {position, ..} => {
                let config = state.config();
                let color = InputHandler::calculate_color_from_mouse(
                    position.x,
                    position.y,
                    config.width,
                    config.height,
                );
                state.set_clear_color(color);
            }
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: key_state,
                    ..
                },
                ..
            } => {
                let action = InputHandler::handle_key(event_loop, code, key_state.is_pressed());
                match action {
                    InputAction::ToggleShape => state.toggle_shape(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
