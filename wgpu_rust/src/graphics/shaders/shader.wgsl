// Vertex shader
// Logic performed for each vertex

struct VertexInput {
    @location(0) position: vec3<f32>, // Input attribute for vertex position
    @location(1) color: vec3<f32>,    // Input attribute for vertex color
}

struct VertexOutput {
    // Vertex position in clip space(meaning inside our viewport)
    // builtin position is in framebuffer coordinates, meaning (0,0) is bottom-left
    // while clip space is normalized device coordinates where (-1,-1) is bottom-left
    @builtin(position) clip_position: vec4<f32>, // Tells GPU about clip space position of vertex
    @location(0) color: vec3<f32>, // Pass color to fragment shader
};

@vertex // Signals its an entry point for the vertex shader
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    // Variables with var can be modified but must specify a type
    // Variables with let can have their types inferred, but value cant change during shader
    var out: VertexOutput; // Variable to hold our output data based on the struct
    out.color = model.color;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0); // Set color to current fragment
}