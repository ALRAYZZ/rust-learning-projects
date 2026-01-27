
pub struct TextureBundle {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}


// Bind group layout defines the interface/contract: what types of resources (texture, sampler, etc.)
// the shader expects at which binding slots. This allows the GPU driver to optimize memory layout
// and validate that the actual bind group matches what the shader needs.
// IT CONTAINS THE SHAPE OF THE DATA, NOT THE DATA ITSELF
pub fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("texture_bind_group_layout"),
    })
}

pub fn load_texture_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    bytes: &[u8],
) -> anyhow::Result<wgpu::BindGroup> {

    // TEXTURE LOADING
    let diffuse_image = image::load_from_memory(bytes)?;
    let diffuse_rgba = diffuse_image.to_rgba8();

    use image::GenericImageView;
    let dimensions = diffuse_image.dimensions();

    // Create Texture from image data
    let texture_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        // All textures are stored as 3d, setting depth 1 to represent it as 2d
        depth_or_array_layers: 1,
    };
    // Tell GPU to find memory space for texture (ALLOCATION ON GPU)
    let diffuse_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Most images stores using sRGB
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // Texture binding tells wgpu that we wanna use this texture in shaders
            // COPY_DST means we will copy data to it
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Diffuse Texture"),
            // Specifies what texture formats can be used to create TextureViews for this texture.
            view_formats: &[],
        }
    );

    // Actual command to move diffuse_rgba bytes from RAM to GPU memory over PCIe bus
    // We use a queue because we cannot send commands directly to GPU, when GPU is ready
    // it will process commands in the queue
    queue.write_texture(
        // Tells wgpu where to copy the pixel data
        wgpu::TexelCopyTextureInfo{
            texture: &diffuse_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // Actual pixel data
        &diffuse_rgba,
        // Layout of texture
        wgpu::TexelCopyBufferLayout{
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        texture_size,
    );

    // If the Texture is the raw film, the TextureView is the lens focusing on a specific part of that film
    // and the sampler as the projector settings that defines how it looks on screen
    // A Texture is a heavy fixed objetc in GPU memory while a TextureView is a lightweight window
    // into that texture, allowing us to see and use specific parts or aspects of the texture
    // Sampler stores instructions on how to read texture data (filtering, wrapping, etc)
    let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge, // what to do when uv coords are outside 0.0-1.0
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });


    // The bind group is the actual binding of resources to the layout's slots.
    // It connects concrete GPU resources (our texture view and sampler) to the binding points
    // defined in the layout. This separation allows you to swap different resources
    // (e.g., different textures) without changing the pipeline, as long as they match the layout.
    // HERE IS THE ACTUAL DATA (EG: TEXTURE FOR BINDING SLOT 0 AND SAMPLER FOR BINDING SLOT 1)
    let diffuse_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                }
            ],
            label: Some("diffuse_bind_group"),
        }
    );

    Ok(diffuse_bind_group)
}