use std::io::Cursor;

use crate::{
    asset_importer::ImportContext,
    rendering::image::Image,
    resources::{RenderContext, VulkanContext},
    COLOR_FORMAT,
};
use ash::vk;
use image::io::Reader as ImageReader;

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

        // HACK
        // This is a *hack*. We don't actually support basisu, we're just using the extension to smuggle pre-compressed ktx2 files
        // into our GLB files. We do *eventually* want to support basisu, but we're both blocked on a couple of upstream crates
        // and this may not be the optimal thing to do for performance reasons.
        //
        // See https://github.com/leetvr/hotham/issues/237 for more details.
        if let (_, Some(basis_u)) = texture.basisu_sources() {
            match basis_u.source() {
                gltf::image::Source::View { view, .. } => {
                    let ktx2_data = &import_context.buffer[view.offset()..view.buffer().length()];
                    let ktx2_reader = ktx2::Reader::new(ktx2_data).unwrap();
                    let header = ktx2_reader.header();
                    if header.supercompression_scheme.is_some() {
                        panic!("[HOTHAM_TEXTURE] ktx2 supercompression is currently unsupported");
                    }

                    // We don't really support mipmaps with Hotham yet. So, we assume there is ONLY ONE mipmap level.
                    let image_bytes = ktx2_reader.levels().next().unwrap();
                    let format = get_format_from_ktx2(header.format);
                    let texture = Texture::new(
                        "",
                        import_context.vulkan_context,
                        import_context.render_context,
                        &image_bytes,
                        header.pixel_width,
                        header.pixel_height,
                        format,
                    );
                    return texture.index;
                }

                _ => panic!("Not supported"),
            }
        }

        let texture = match texture.source().source() {
            gltf::image::Source::View { view, mime_type } => {
                println!("[HOTHAM_TEXTURE] - WARNING: Non-optimal image format detected. For best performance, compress your images into ktx2.");
                println!("[HOTHAM_TEXTURE] - Decompresing image. This may take some time..");
                let bytes = &import_context.buffer[view.offset()..view.buffer().length()];
                let format = get_format_from_mime_type(mime_type);
                let asset = Cursor::new(bytes);
                let mut image = ImageReader::new(asset);
                image.set_format(format);
                let image = image.decode().expect("Unable to decode image!");
                let image = image.to_rgba8();
                let width = image.width();
                let height = image.height();

                println!("[HOTHAM_TEXTURE] - ..done!");

                Texture::new(
                    texture_name,
                    import_context.vulkan_context,
                    import_context.render_context,
                    &image.into_raw(),
                    width,
                    height,
                    TEXTURE_FORMAT,
                )
            }
            _ => panic!(
                "[HOTHAM_TEXTURE] - Unable to import image - URI references are not supported"
            ),
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
}

fn get_format_from_mime_type(mime_type: &str) -> image::ImageFormat {
    match mime_type {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        _ => panic!(
            "Unable to import image - unsupported MIME type {}",
            mime_type
        ),
    }
}

// This is legal.. with some caveats. But if it's wrong it'll blow up when the texture gets imported anyway
fn get_format_from_ktx2(format: Option<ktx2::Format>) -> vk::Format {
    let raw = format.expect("No format specified").0;
    vk::Format::from_raw(raw.get() as _)
}
