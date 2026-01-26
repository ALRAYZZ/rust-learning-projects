// repr(C) ensures the struct has a predictable memory layout C style so no unexpected padding or
// reordering occurs, important for GPU data

// Pod guarantess that the struct can be safely treated as a plain byte array
// Zeroable allows the struct to be initialized to all zeros safely

// GPU doesnt understand Rust structs directly. For GPU a buffer is a long sequence of bytes u8
// Thats why we need the bytemuck crate to convert between Rust structs and byte arrays
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