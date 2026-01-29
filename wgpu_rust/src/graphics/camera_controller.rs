use winit::keyboard::KeyCode;
use crate::graphics::camera::Camera;

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    // We use booleans so the movement is smooth while key is held down
    pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;

        // In 3D if we subtract two points we get a vector pointing from one to the other
        // So here we get a vector pointing from the camera position to the target position
        let forward = camera.target - camera.eye;
        // Normalize the vector so speed is consistent regardless of distance
        // else moving forward when close to target would be slower than when far away
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to center scene
        // If eye and target are the same we cant get a direction to move in
        // So we only move forward if the distance is greater than speed
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        // If we do a cross product of two vectors we get a vector perpendicular to both
        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case fwrd/bckwrd changed it
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale distance between the target and the eye so
            // that it does not change. The eye still lies on the circle made by target and eye.
            // We orbit around the target in the right direction
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            // Orbit around target to the left keeping same distance because
            // we add left/right vector to the forward vector before normalizing and scaling
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}