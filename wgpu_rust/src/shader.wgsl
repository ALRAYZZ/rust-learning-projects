// Vertex shader
// Logic performed for each vertex

struct VertexOutput {
    // Vertex position in clip space(meaning inside our viewport)
    // builtin position is in framebuffer coordinates, meaning (0,0) is bottom-left
    // while clip space is normalized device coordinates where (-1,-1) is bottom-left
    @builtin(position) clip_position: vec4<f32>, // Tells GPU about clip space position of vertex
    @location(0) vert_pos: vec3<f32>, // Custom output to pass vertex position to fragment shader
};

@vertex // Signals its an entry point for the vertex shader
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, // We expect a u32 input representing the vertex index
) -> VertexOutput {
    // Variables with var can be modified but must specify a type
    // Variables with let can have their types inferred, but value cant change during shader
    var out: VertexOutput; // Variable to hold our output data based on the struct
    // Calculate x and y positions based on vertex index
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    // Save the calculated position into the output struct field
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0); // Set color to current fragment
}