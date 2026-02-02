use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

pub struct InputHandler;

pub enum InputAction {
    None,
    Exit,
    ToggleShape,
    ToggleDepthVisualization,
}

impl InputHandler {

    // Handle keyboard input events
    pub fn handle_key(event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) -> InputAction {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => {
                event_loop.exit();
                InputAction::Exit
            }
            (KeyCode::Space, true) => InputAction::ToggleShape,
            (KeyCode::KeyV, true) => InputAction::ToggleDepthVisualization,
            _ => InputAction::None,
        }
    }

    pub fn calculate_color_from_mouse(x: f64, y: f64, width: u32, height: u32) -> wgpu::Color {
        // Get window dimensions
        let width = width as f64;
        let height = height as f64;

        // Normalize mouse position to [0, 1] range and update clear color
        // clamp as a safety net in case fast movements report out of bounds values
        wgpu::Color {
            r: (x / width).clamp(0.0, 1.0),
            g: (y / height).clamp(0.0, 1.0),
            b: 0.3,
            a: 1.0,
        }
    }
}