// Defines how GPU should read the instance buffer and map those bytes into the shader
// A struct in a shader tells the GPU: For every instances i draw, look at the current memory address
// and extract these four vectors
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct RenderModeUniform {
    mode: u32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
};

@group(3) @binding(0)
var<uniform> render_mode: RenderModeUniform;

@group(2) @binding(0)
var depth_tex: texture_depth_2d;
@group(2) @binding(1)
var depth_sampler: sampler;

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
    @location(2) normal: vec3<f32>, // Input attribute for vertex normal (for lighting calculations)
}

// Data into rasterizer and fragment shader
struct VertexOutput {
    // Vertex position in clip space(meaning inside our viewport)
    // builtin position is in framebuffer coordinates, meaning (0,0) is bottom-left
    // while clip space is normalized device coordinates where (-1,-1) is bottom-left
    @builtin(position) clip_position: vec4<f32>, // Tells GPU about clip space position of vertex
    @location(0) tex_coords: vec2<f32>, // Pass texture coordinates to fragment shader
    @location(1) world_normal: vec3<f32>, // Pass normal to fragment shader for lighting calculations
    @location(2) world_position: vec3<f32>, // Pass world position to fragment shader for lighting calculations
};

// Need the light position data in this shader to actually do light calculations based on its position and color
struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(4) @binding(0)
var<uniform> light: Light;

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
    // Passing data from vertex shader to fragment shader so it can do texturing and lighting calculations
    out.tex_coords = model.tex_coords;
    out.world_normal = model.normal;

    // Converting to World Space (Model position is relative to itself, bringing model matrix moves vertex to the world)
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;

    // Converting to Clip Space (this is where the Camera happens)
    out.clip_position = camera.view_proj * world_position;
    return out;
}

// Fragment shader (coloring) fragment shaders runs per pixel
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>; // 2D texture bound to group 0 binding 0
@group(0) @binding(1)
var s_diffuse: sampler; // Sampler bound to group 0 binding 1

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // If render mode is 1, visualize the depth buffer instead of the texture
    if (render_mode.mode == 1u) {
        let coords = vec2<i32>(in.clip_position.xy);
            let d = textureLoad(depth_tex, coords, 0);

            // If d is 0.99, this becomes 0.01.
            // If d is 1.0 (background), this becomes 0.0.
            let visualize = 1.0 - d;

            // Multiply by a huge number to force it to show up
            return vec4<f32>(vec3<f32>(visualize * 100.0), 1.0);
    }

    // normal textured rendering
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    // Simple ambient light
    let ambient_strenght = 0.1;
    let ambient_color = light.color * ambient_strenght;

    // Diffuse light
    let light_dir = normalize(light.position - in.world_position);
    let diffuse_strenght = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strenght;

    let result = (ambient_color + diffuse_color) * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}