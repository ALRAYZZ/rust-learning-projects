// Uniform buffers are meant for small amounts of data that stay constant across draw calls,
// to keep them fast, hardware requires very strict 16-byte alignment.


// This buffer represents a single light source in our scene, with its position and color.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    pub _padding: u32, // Padding to align to 16 bytes because uniform buffers require 16-byte alignment
    pub color: [f32; 3],
    pub _padding2: u32, // Additional padding to ensure the struct size is a multiple of 16 bytes
}


pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Light Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0, // This is the actual binding index used in shader
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ],
    })
}

pub fn create_bind_group_from_light(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    light_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }
        ],
        label: Some("Light Bind Group"),
    })
}