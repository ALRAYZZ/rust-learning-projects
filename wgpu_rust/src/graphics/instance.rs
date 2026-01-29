use std::mem;

pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>, // Quaternion is a math representation for 3D rotations
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4], // 4x4 matrix for model transformation
}

impl Instance {
    // Convert position and rotation matrix into a model matrix for the GPU
    // Model matrix combines translation, rotation, and scaling
    // Also we need to convert the Rust data into data that GPU understands (InstanceRaw)
    // We dont wanna give the GPU commands like move point here, then rotate there
    // Instead we give it a single model matrix that combines all transformations (Model Matrix = Translation * Rotation * Scale)
    // Then we need to translate our cgmath types into raw arrays of f32 that GPU understands
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) *
                cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

impl InstanceRaw {
    // Descriptor methods are like the instruction manual for the GPU
    // Without this the GPU wouldnt know how to interpret the raw byte data in the buffer
    // Here we are telling the GPU that our InstanceRaw struct is made up of 4 vec4s (4 f32 arrays of length 4)
    // And each vec4 corresponds to a row of the model matrix
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // Need to switch from using a step mode of Vertex to Instance
            // This means our shaders will only change to use the next
            // instance when shader starts processing new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define
                // for each vec4. We will have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ]
        }
    }
}