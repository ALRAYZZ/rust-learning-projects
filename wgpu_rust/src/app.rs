use std::sync::Arc;
use instant::Instant;
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
    last_render_time: Option<Instant>,
}

impl App  {
    pub fn new() -> Self {
        Self {
            state: None,
            last_render_time: None,
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

        // Initialize last_render_time when the app is resumed
        self.last_render_time = Some(Instant::now());
    }

    // Handle window events like resize, close, redraw, keyboard input
    // called by the event loop when such events occur
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let mut state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = now - self.last_render_time.unwrap_or(now);
                self.last_render_time = Some(now);

                state.update(dt);
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
                // Handle camera movement input
                state.camera_controller.handle_key(code, key_state);

                // Handle application-level input
                let action = InputHandler::handle_key(event_loop, code, key_state.is_pressed());
                match action {
                    InputAction::ToggleShape => state.toggle_shape(),
                    InputAction::ToggleDepthVisualization => state.toggle_depth_visualization(),
                    InputAction::Exit => event_loop.exit(),
                    _ => {}
                }

            }
            WindowEvent::MouseWheel { delta, .. } => {
                state.camera_controller.handle_mouse_scroll(&delta);
            }
            // Tracks if user is holding down the left mouse button for use (e.g., dragging to rotate camera)
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: mouse_state,
                ..
            } => {
                state.mouse_pressed = mouse_state == ElementState::Pressed;
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        // Handles camera rotation only if mouse is moved and button pressed
        if let DeviceEvent::MouseMotion { delta } = event {
            if state.mouse_pressed {
                state.camera_controller.handle_mouse(delta.0, delta.1);
            }
        }
    }
}
