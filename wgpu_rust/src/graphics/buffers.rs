use wgpu::util::DeviceExt;

// Create a vertex buffer from given vertices
pub fn create_vertex_buffer(device: &wgpu::Device, vertices: &[crate::graphics::vertex::Vertex])
    -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    )
}