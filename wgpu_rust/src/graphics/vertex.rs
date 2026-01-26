#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]

pub struct Vertex {
    position: [f32; 3], // Fixed size array for position (x, y, z)
    color: [f32; 3], // Fixed size array for color (r, g, b)
}


pub const VERTICES: &[Vertex] = &[
    Vertex {position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex {position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex {position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] }
];