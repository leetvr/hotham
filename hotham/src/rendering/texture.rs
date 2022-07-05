use crate::{
    asset_importer::ImportContext,
    rendering::image::Image,
    resources::{RenderContext, VulkanContext},
    COLOR_FORMAT,
};
use anyhow::{anyhow, Result};
use ash::vk;
use gltf::image::Format;
use image::io::Reader as ImageReader;
use libktx_rs::{sources::StreamSource, RustKtxStream, TextureCreateFlags, TextureSource};
use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
/// A texture that can be accessed in a fragment shader on the GPU
pub struct Texture {
    /// Handle to the underlying image
    pub image: Image,
    /// Index in the shader
    pub index: u32,
}

const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

/// Texture index to indicate to the shader that this material does not have a texture of the given type
pub static NO_TEXTURE: u32 = std::u32::MAX;

impl Texture {
    /// Creates a new texture
    pub fn new(
        name: &str,
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        image_buf: &[u8],
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Self {
        let image = vulkan_context
            .create_image(
                format,
                &vk::Extent2D { width, height },
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                1,
                1,
            )
            .unwrap();

        let index = render_context
            .create_texture_image(name, vulkan_context, image_buf, 1, vec![0], &image)
            .unwrap();

        Texture { image, index }
    }

    /// Load a texture from a glTF document. Returns the texture ID
    pub(crate) fn load(texture: gltf::texture::Texture, import_context: &mut ImportContext) -> u32 {
        let texture_name = &format!("Texture {}", texture.name().unwrap_or(""));
        let texture = match texture.source().source() {
            gltf::image::Source::Uri { uri, .. } => {
                let (buf, width, height) = parse_image(uri)
                    .unwrap_or_else(|_| panic!("Unable to load image! URI: {}", uri));
                Texture::new(
                    texture_name,
                    import_context.vulkan_context,
                    import_context.render_context,
                    &buf,
                    width,
                    height,
                    TEXTURE_FORMAT,
                )
            }
            // TODO: Fix this
            gltf::image::Source::View { .. } => {
                let index = texture.source().index();
                let image = &import_context.images[index];
                if image.format != Format::R8G8B8A8 {
                    let pixels = add_alpha_channel(image);
                    Texture::new(
                        texture_name,
                        import_context.vulkan_context,
                        import_context.render_context,
                        &pixels,
                        image.width,
                        image.height,
                        TEXTURE_FORMAT,
                    )
                } else {
                    Texture::new(
                        texture_name,
                        import_context.vulkan_context,
                        import_context.render_context,
                        &image.pixels,
                        image.width,
                        image.height,
                        TEXTURE_FORMAT,
                    )
                }
            }
        };

        texture.index
    }

    /// Create an empty texture. Useful for obtaining a texture you want to write to later on.
    pub fn empty(
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        resolution: vk::Extent2D,
    ) -> Self {
        let image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &resolution,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                1,
                1,
            )
            .unwrap();
        let index = render_context
            .create_texture_image("Empty Texture", vulkan_context, &[], 1, vec![0], &image)
            .unwrap();

        Texture { image, index }
    }

    /// Load a texture from a KTX2 buffer
    pub fn from_ktx2(
        name: &str,
        buf: &[u8],
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
    ) -> Result<Self> {
        let buf = Cursor::new(buf.to_vec());
        let buf = Box::new(buf);
        let (buf, image, mip_levels, offsets) = parse_ktx(buf, vulkan_context)?;
        println!(
            "Creating texture image with format {:?}, array layers {} and mip_levels {}",
            image.format, image.layer_count, mip_levels
        );

        let index = render_context.create_texture_image(
            name,
            vulkan_context,
            &buf,
            mip_levels,
            offsets,
            &image,
        )?;

        Ok(Texture { image, index })
    }
}

#[cfg(not(target_os = "android"))]
fn parse_image(path: &str) -> Result<(Vec<u8>, u32, u32)> {
    let path = format!(r#"..\test_assets\\{}"#, path);
    let img = ImageReader::open(path)?.decode()?;
    let img = img.to_rgba8();
    let width = img.width();
    let height = img.height();
    Ok((img.into_raw(), width, height))
}

#[cfg(target_os = "android")]
fn parse_image(path: &str) -> Result<(Vec<u8>, u32, u32)> {
    use crate::util::get_asset_from_path;
    let bytes = get_asset_from_path(path)?;
    let format = image::guess_format(&bytes).unwrap();
    let asset = Cursor::new(bytes);
    let mut img = ImageReader::new(asset);
    img.set_format(format);
    let img = img.decode()?;
    let img = img.to_rgba8();
    let width = img.width();
    let height = img.height();
    return Ok((img.into_raw(), width, height));
}

/// Parse the contents of a KTX2 buffer
pub fn parse_ktx(
    buf: Box<Cursor<Vec<u8>>>,
    vulkan_context: &VulkanContext,
) -> Result<(Vec<u8>, Image, u32, Vec<vk::DeviceSize>)> {
    let stream = RustKtxStream::new(buf).map_err(|e| anyhow!("Couldn't create stream: {}", e))?;
    let source = Arc::new(Mutex::new(stream));
    let mut texture = StreamSource::new(source, TextureCreateFlags::LOAD_IMAGE_DATA)
        .create_texture()
        .unwrap();

    let image_buf = texture.data().to_vec();
    let mut offsets = Vec::new();
    let (height, width, layer_count, mip_count) = unsafe {
        let ktx_texture = &(*texture.handle());
        let layers = if ktx_texture.isCubemap {
            6
        } else {
            ktx_texture.numLayers
        };
        let mip_levels = ktx_texture.numLevels;

        for face in 0..ktx_texture.numFaces {
            for mip_level in 0..mip_levels {
                let offset = texture.get_image_offset(mip_level, 0, face)?;
                offsets.push(offset as vk::DeviceSize);
            }
        }

        (
            ktx_texture.baseHeight,
            ktx_texture.baseWidth,
            layers,
            mip_levels,
        )
    };
    let ktx2 = texture.ktx2().unwrap();
    let format = vk::Format::from_raw(ktx2.vk_format() as _);

    let image = vulkan_context.create_image(
        format,
        &vk::Extent2D { width, height },
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        layer_count,
        mip_count,
    )?;

    Ok((image_buf, image, mip_count, offsets))
}

fn add_alpha_channel(image: &gltf::image::Data) -> Vec<u8> {
    let final_size = (image.height * image.width) * 4;
    let mut final_image = vec![0; final_size as _];
    let original_image = &image.pixels;
    let mut original_index = 0;
    let mut final_index = 0;
    while original_index < original_image.len() {
        final_image[final_index..(3 + final_index)]
            .clone_from_slice(&original_image[original_index..(3 + original_index)]);
        final_image[final_index + 3] = 1;

        original_index += 3;
        final_index += 4;
    }

    final_image
}
