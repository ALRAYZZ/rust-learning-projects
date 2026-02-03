// File creates raw vertex data for models to be sent to GPU
// Unused since we load models from files now, but kept for reference and testing
// model.rs handles model loading from files now


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
    tex_coords: [f32; 2],
}


// Changing the Y text cords doing 1-y flips the texture vertically
pub const PENT_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43031354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732914], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], }, // E
];

// Indices define how vertices are connected to form triangles
// Each group of 3 indices represents a triangle
// So we save memory by reusing vertices for multiple triangles
pub const PENT_INDICES: &[u16] = &[
    0, 1, 4, // Triangle ABE
    1, 2, 4, // Triangle BCE
    2, 3, 4, // Triangle CDE
];

pub const COMPLEX_SHAPE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0], tex_coords: [0.0, 0.0], }, // Bottom-left
    Vertex { position: [0.0, -0.5, 0.0], tex_coords: [0.5, 0.0], },  // Bottom-center
    Vertex { position: [0.5, -0.5, 0.0], tex_coords: [1.0, 0.0], },  // Bottom-right
    Vertex { position: [-0.5, 0.0, 0.0], tex_coords: [0.0, 0.5], },  // Middle-left
    Vertex { position: [0.0, 0.5, 0.0], tex_coords: [0.5, 1.0], },   // Top-center peak
    Vertex { position: [0.5, 0.0, 0.0], tex_coords: [1.0, 0.5], },   // Middle-right
    Vertex { position: [0.75, -0.25, 0.0], tex_coords: [1.25, 0.25], }, // Small tip at far right
];

pub const COMPLEX_SHAPE_INDICES: &[u16] = &[
    0, 1, 3, // First triangle (Bottom-left area)
    1, 2, 3, // Second triangle (Fills the center square)
    3, 2, 4, // Third triangle (Top-left peak)
    2, 5, 4, // Fourth triangle (Top-right peak)
    2, 6, 5, // Fifth triangle (Small tip at the far right)
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
                    format: wgpu::VertexFormat::Float32x2, // 2 floats for tex_coords
                },
            ]
        }
    }
}