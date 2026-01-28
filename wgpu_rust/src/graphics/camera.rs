// Conversion matrix from OpenGL to WGPU coordinate system
// OpenGL (cgmath) Z axis ranges from -1 to 1
// WGPU (DirectX/Vulkan/Metal) Z axis ranges from 0 to 1
// This matrix scales Z by 0.5 and translates it by 0.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);
pub struct CameraConfig {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}


impl Camera {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            eye: config.eye,
            target: config.target,
            up: config.up,
            aspect: config.aspect,
            fovy: config.fovy,
            znear: config.znear,
            zfar: config.zfar,
        }
    }

    // Setters/getters
    // ..

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {

        // GPUs dont actually move the camera, instead we move and rotate the entire scene inversely to simulate camera movement
        // the view matrix offsets every vertex so that they are relative to the camera position and orientation
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);

        // The projection matrix defines how 3D points are projected onto the 2D screen
        // making farther objects appear smaller to create depth perception X and Y divided by Z
        let proj = cgmath::perspective(
            cgmath::Deg(self.fovy),
            self.aspect,
            self.znear,
            self.zfar,
        );
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}
// Rust by default rearranges struct fields to make it as small as possible in memory
// This can cause issues when sending data to GPU which expects a specific memory layout
// So we use #[repr(C)] to tell Rust to use C-style memory layout (no rearranging)
// Derive tells Rust to write boilerplate code for us Debug (for printing), Clone, Copy (for copying values)
// bytemuck::Pod and bytemuck::Zeroable are traits from bytemuck
// Pod means Plain Old Data, can be safely converted to/from byte arrays
// Zeroable means the struct can be initialized to all zeros safely
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // Cant use cgmath with bytemuck so we convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }


    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("Camera Bind Group Layout"),
        })
    }

    pub fn create_bind_group(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("Camera Bind Group"),
        })
    }
}