use cgmath::{Matrix, SquareMatrix};
use crate::model;

// This module allows instancing, which is rendering multiple copies of the same object with different transformations
// --- Instance Data for "Draw Call" Optimization ---
// Goal: Render thousands of copies of the same mesh (Pentagon) in a single command.
// logic:
// 1. Instance: High-level Rust data (Position/Rotation).
// 2. InstanceRaw: GPU-ready 4x4 Model Matrix (collapses TRS into one step).
// 3. step_mode: Instance: Tells GPU "Use one matrix per object, not per vertex."
// 4. VertexAttributes: Splits the 4x4 matrix into 4 'slots' for the shader.
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>, // Quaternion is a math representation for 3D rotations
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4], // 4x4 matrix for model transformation
    normal: [[f32; 3]; 3], // 3x3 matrix for normal transformation (for lighting)
}

impl Instance {
    // Convert position and rotation matrix into a model matrix for the GPU
    // Model matrix combines translation, rotation, and scaling
    // Also we need to convert the Rust data into data that GPU understands (InstanceRaw)
    // We dont wanna give the GPU commands like move point here, then rotate there
    // Instead we give it a single model matrix that combines all transformations (Model Matrix = Translation * Rotation * Scale)
    // Then we need to translate our cgmath types into raw arrays of f32 that GPU understands
    pub fn to_raw(&self) -> InstanceRaw {
        let model_matrix = cgmath::Matrix4::from_translation(self.position) *
            cgmath::Matrix4::from(self.rotation);

        InstanceRaw {
            model: model_matrix.into(),
            normal: cgmath::Matrix3::from(self.rotation).into(),
        }
    }
}

impl InstanceRaw {
    // Descriptor methods are like the instruction manual for the GPU
    // Without this the GPU wouldnt know how to interpret the raw byte data in the buffer
    // Vertex Buffer Layour that we will send to the GPU to tell it how to interpret the instance data
    // We are defining the shape our data takes in the GPU memory, how big each instance is,
    // and how to split the model matrix into attributes for the shader
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // Need to switch from using a step mode of Vertex to Instance
            // This means our shaders will only change to use the next
            // instance when shader starts processing new instance
            step_mode: wgpu::VertexStepMode::Instance,
            // Shaders slots can only hold a max of vec4 (4 f32 values)
            // that's why we need many attributes to split the 4x4 model matrix into 4 vec4s
            // 5 to 8 for the model matrix, 9 to 11 for the normal matrix
            attributes: &[
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
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ]
        }
    }
}