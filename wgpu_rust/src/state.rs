use std::sync::Arc;
use winit::window::Window;
use crate::graphics;

// THE ENGINE
// GPU context. Live inside APP, holds device, queue, surface, config, translates logic into
// binary commands for GPU
pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    clear_color: wgpu::Color,
    is_surface_configured: bool,

    pub(crate) window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    num_vertices: u32,
    num_indices: u32,

    vertex_buffer_2: wgpu::Buffer,
    index_buffer_2: wgpu::Buffer,

    num_vertices_2: u32,
    num_indices_2: u32,

    active_shape: usize,

    diffuse_bind_group: wgpu::BindGroup,
}

// Defined methods for the Window we create
impl State {
    // Handshake with GPU to see what it supports and create device/queue
    // Constructor to initialize State
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        // Instance is "The Manager" knows every GPU backend available
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Part of the window that we can draw to
        // Take this window handle and prepare it to receive raw pixel data from GPU
        let surface = instance.create_surface(window.clone())?;

        // Handler for graphics card, to get info about it and create device/queue
        // The actual selected GPU
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface), // Find adapter compatible with our surface
                force_fallback_adapter: false, // If true will use software rendering
            })
            .await?;

        // Device is connection to GPU, Queue is needed to send commands since
        // We cannot say to gpu "Draw now" we send commands and wait for gpu to process them
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesnt support all wgpu features
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        // Config for surface. This will define how surface creates SurfaceTextures
        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Config where we define how large image is and if we are using vsync etc
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, // how surface textures will be used
            format: surface_format, // how SurfaceTextures will be stored
            width: size.width, // in pixels, usually matches window size
            height: size.height,
            present_mode: surface_caps.present_modes[0], // how to sync surface with display
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Load texture bytes
        let diffuse_bytes = include_bytes!("../assets/happy-tree.png");

        let texture_bind_group_layout =
            graphics::texture::create_texture_bind_group_layout(&device);
        let diffuse_bind_group =
            graphics::texture::load_texture_from_bytes(
                &device,
                &queue,
                &texture_bind_group_layout,
                diffuse_bytes,
            )?;

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        // Buffers creation
        let vertex_buffer = graphics::buffers::create_vertex_buffer(&device, graphics::vertex::PENT_VERTICES);
        let index_buffer = graphics::buffers::create_index_buffer(&device, graphics::vertex::PENT_INDICES);

        let num_vertices = graphics::vertex::PENT_VERTICES.len() as u32;
        let num_indices = graphics::vertex::PENT_INDICES.len() as u32;

        // 2nd Buffer (different shape)
        let vertex_buffer_2 = graphics::buffers::create_vertex_buffer(&device, graphics::vertex::COMPLEX_SHAPE_VERTICES);
        let index_buffer_2 = graphics::buffers::create_index_buffer(&device, graphics::vertex::COMPLEX_SHAPE_INDICES);

        let num_vertices_2 = graphics::vertex::COMPLEX_SHAPE_VERTICES.len() as u32;
        let num_indices_2 = graphics::vertex::COMPLEX_SHAPE_INDICES.len() as u32;


        let render_pipeline = graphics::pipeline::create_render_pipeline(&device, &config, &texture_bind_group_layout);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            clear_color,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_vertices,
            num_indices,
            vertex_buffer_2,
            index_buffer_2,
            num_vertices_2,
            num_indices_2,
            active_shape: 0,
            diffuse_bind_group,
        })
    }

    // Method to resize the surface when window size changes
    // Surface is a collection of buffers that need the right memory size to store the needed
    // amount of pixels, and that amount changes when window is resized
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn set_clear_color(&mut self, clear_color: wgpu::Color) {
        self.clear_color = clear_color;
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn toggle_shape(&mut self) {
        // Toggle logic: if 0 and method called, set to 1
        self.active_shape = if self.active_shape == 0 { 1 } else { 0 };
    }

    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    pub fn update(&mut self) {
        // TODO
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // Cant render if surface is not configured
        if !self.is_surface_configured {
            return Ok(());
        }

        // Get the next frame to render to
        let output = self.surface.get_current_texture()?;
        // Control how the render interacts with the texture
        // A texture is the 2D array of pixels that we will draw to and then present to screen
        // Texture view is how we going to use that texture in the render pass
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create actual commands to send to GPU. Builds a command buffer
        // Modern graphics expect commands to be stored in a command buffer before being sent
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // RenderPass has all the methods for actual drawing.
        // Here we populate with shaders, buffers, textures, etc
        {
            // Begin a render pass borrows the encoder mutably so thats why
            // we have this nested scope so later we can call encoder.finish()
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, // specific texture memory to draw to
                    resolve_target: None, // anti-aliasing resolve target
                    depth_slice: None, //
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color), // Clear color before drawing
                        store: wgpu::StoreOp::Store, // Store the result in memory after render pass
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            // Here we set the pipeline (shaders + fixed function state) and issue draw commands
            render_pass.set_pipeline(&self.render_pipeline);

            // Set the bind group for the texture
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);

            // Buffer selection based on active shape
            // If active_shape is 0, use first buffers, else use second buffers
            let (vertex_buffer, index_buffer, num_indices) = if self.active_shape == 0 {
                (&self.vertex_buffer, &self.index_buffer, self.num_indices)
            } else {
                (&self.vertex_buffer_2, &self.index_buffer_2, self.num_indices_2)
            };



            // Set the vertex buffer to use
            // Method 1st param, is what buffer slot to use for this vertex buffer
            // We can have multiple vertex buffers bound at once (positions, colors, uvs, etc)
            // Second param, slice of the buffer to use, we can store multiple meshes in one buffer
            // (..) means use full buffer
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));


            // Index buffer is a memory optimization to reuse vertices for multiple triangles
            // We create a matrix of indices saying what vertices are shared between triangles
            // This way we dont have to duplicate vertex data in memory
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        } // Scope ends here, so render_pass is dropped and encoder can be used again

        // Submit commands to GPU queue for execution
        // Submit will accept anything that implements IntoIterator<Item=&CommandBuffer>
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
