use wgpu::util::DeviceExt;
use crate::graphics::camera::CameraUniform;
use crate::graphics::instance::InstanceRaw;
use crate::graphics::light;

// Vertex buffer holds vertex data (positions, colors, texture coords, etc)
pub fn create_vertex_buffer(device: &wgpu::Device, vertices: &[crate::graphics::vertex::Vertex])
    -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    )
}

// New implementation for model vertices struct used for the loading 3d models from obj files
pub fn create_model_vertex_buffer(device: &wgpu::Device, vertices: &[crate::model::ModelVertex])
    -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
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

// New implementation for model vertices struct used for the loading 3d models from obj files
pub fn create_model_index_buffer(device: &wgpu::Device, indices: &[u32])
    -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        }
    )
}

// Uniform buffer holds data that remains constant for entire draw calls
// while vertex data(position, color, uvs) change for every point drawn, a uniform buffer holds
// the data that stays the same for every part of the shape, camera position, light direction, etc
// METHOD made GENERIC to accept any type that implements bytemuck::Pod + bytemuck::Zeroable
pub fn create_uniform_buffer<T: bytemuck::Pod + bytemuck::Zeroable>(
    device: &wgpu::Device,
    data: &T
) -> wgpu::Buffer {
    device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    )
}

pub fn create_instance_buffer(device: &wgpu::Device, instance_data: Vec<InstanceRaw>) -> wgpu::Buffer {
    device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        }
    )
}
