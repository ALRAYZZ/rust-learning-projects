use wgpu::util::DeviceExt;
use crate::graphics::camera::CameraUniform;

// Vertex buffer holds vertex data (positions, colors, texture coords, etc)
pub fn create_vertex_buffer(device: &wgpu::Device, vertices: &[crate::graphics::vertex::Vertex])
    -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    )
}

// Index buffer holds indices that define how vertices are connected to form triangles
pub fn create_index_buffer(device: &wgpu::Device, indices: &[u16])
    -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        }
    )
}

// Uniform buffer holds data that remains constant for entire draw calls
// while vertex data(position, color, uvs) change for every point drawn, a uniform buffer holds
// the data that stays the same for every part of the shape, camera position, light direction, etc
pub fn create_uniform_buffer(device: &wgpu::Device, camera_uniform: &CameraUniform) -> wgpu::Buffer {
    device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    )
}