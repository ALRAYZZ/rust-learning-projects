use crate::graphics;

pub fn create_render_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::RenderPipeline {

    // Takes the shader file and sends it to GPU driver
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
    });

    // What extra data can the shader access (external buffers, textures, etc)
    let render_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

    // Defines the fixed-function state and links shaders, tells GPU how to transform vertices
    // Combines shader logic (VS and FS) with hardware settings into a single object to use
    // Factory assembly line, from raw data to the final pixel color before we start
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        // 1st Programmable step, vertex shader
        // Transform raw input data into Clip Space positions
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"), // Shader function to use as entry point
            buffers: &[graphics::vertex::Vertex::desc()], // Describe the layout of vertex buffer
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        // 2nd Programmable step, determines the color of every pixel inside the triangles.
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        // Fixed function states not programmable by shaders
        // Switches and configurations for parts of the GPU pipeline
        // Takes individual points from vertex shader and assembles them into shapes
        // (triangles, lines, etc) also handles culling and front face definition
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None, // Used for special topologies like triangle strips
            front_face: wgpu::FrontFace::Ccw, // Counter-clockwise vertices are front face
            cull_mode: Some(wgpu::Face::Back), // Cull back faces
            polygon_mode: wgpu::PolygonMode::Fill, // Fill, Line, Point
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        // How to handle multi-sampling (anti-aliasing)
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}