// Vertex shader (positioning)
// Logic performed for each vertex

struct CameraUniform {
    view_proj: mat4x4<f32>, // View-projection matrix for transforming vertices
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform; // Uniform buffer for camera data

// Data comes from vertex buffer
struct VertexInput {
    // Location means the layout location of the attribute in the vertex buffer
    @location(0) position: vec3<f32>, // Input attribute for vertex position
    @location(1) tex_coords: vec2<f32>,  // Input attribute for texture coordinates
}

// Data into rasterizer and fragment shader
struct VertexOutput {
    // Vertex position in clip space(meaning inside our viewport)
    // builtin position is in framebuffer coordinates, meaning (0,0) is bottom-left
    // while clip space is normalized device coordinates where (-1,-1) is bottom-left
    @builtin(position) clip_position: vec4<f32>, // Tells GPU about clip space position of vertex
    @location(0) tex_coords: vec2<f32>, // Pass texture coordinates to fragment shader
};

@vertex // Signals its an entry point for the vertex shader
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput; // Variable to hold our output data based on the struct
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0); // Transform vertex position to clip space
    return out;
}

// Fragment shader (coloring) fragment shaders runs per pixel
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>; // 2D texture bound to group 0 binding 0
@group(0) @binding(1)
var s_diffuse: sampler; // Sampler bound to group 0 binding 1

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}