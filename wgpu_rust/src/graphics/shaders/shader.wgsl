// Defines how GPU should read the instance buffer and map those bytes into the shader
// A struct in a shader tells the GPU: For every instances i draw, look at the current memory address
// and extract these four vectors
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}



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
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // WGPU cant handle mat4x4 as input, so we reconstruct it here from 4 vec4s
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    // Actual transformation from model space to clip space (single poing from 3D file space to 2D screen space)
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
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