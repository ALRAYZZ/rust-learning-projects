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
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
];

// Indices define how vertices are connected to form triangles
// Each group of 3 indices represents a triangle
// So we save memory by reusing vertices for multiple triangles
pub const INDICES: &[u16] = &[
    0, 1, 4, // Triangle ABE
    1, 2, 4, // Triangle BCE
    2, 3, 4, // Triangle CDE
];

// Since we convert all vertex data into a single byte array, we need to specify
// how the GPU should interpret that byte array back into our Vertex struct
// Like how long is the position array, where does color start, etc
// This is done using a VertexBufferLayout
impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress, // Size of one vertex in bytes
            // step mode defines when to move to the next vertex
            // Vertex(default) means move to next vertex after each vertex
            // Instance(copy-paste) means move to next vertex after each instance (for instanced rendering)
            // Like if we wanna draw 1000 trees, we can use same vertex data so GPU doesnt have to load it 1000 times
            // Then use another buffer to define positions of each tree instance
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[ // Describe individual attributes in the vertex, 1:1 mapping to struct fields
                // Each vertex attribute is essentially a field in our Vertex struct
                // We need to tell GPU where each attribute starts (offset), which location it maps to
                // in the shader (shader_location), and what format it is (format)
                wgpu::VertexAttribute {
                    offset: 0, // Position starts at byte 0
                    shader_location: 0, // Location 0 in shader (layout(location = 0) in GLSL)
                    format: wgpu::VertexFormat::Float32x3, // 3 floats for position
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress, // Offset based on size of position
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3, // 3 floats for color
                },
            ]
        }
    }
}