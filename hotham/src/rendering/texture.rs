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
        extent: &vk::Extent2D,
        format: vk::Format,
    ) -> Self {
        let image = vulkan_context
            .create_image(
                format,
                &extent,
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
            // HACK
            // This is a *hack*. Storing ktx2 images in the source field without the KHR_texture_basisu extension
            // is *not allowed*. But, such is life.
            //
            // See https://github.com/leetvr/hotham/issues/237 for more details.
            gltf::image::Source::View { view, mime_type } => {
                let start = view.offset();
                let end = start + view.length();
                let bytes = &import_context.buffer[start..end];
                match mime_type {
                    "image/ktx2" => Texture::from_ktx2(
                        texture_name,
                        import_context.vulkan_context,
                        import_context.render_context,
                        bytes,
                    ),
                    _ => Texture::from_uncompressed(
                        texture_name,
                        mime_type,
                        import_context.vulkan_context,
                        import_context.render_context,
                        bytes,
                    ),
                }
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

    /// Create a texture from a ktx2 container.
    ///
    /// This is the preferred way of using textures in Hotham. By compressing the image in a GPU friendly way we can drastically reduce the amount of
    /// bandwidth used to sample from it.
    pub fn from_ktx2(
        name: &str,
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        ktx2_data: &[u8],
    ) -> Self {
        let ktx2_reader = ktx2::Reader::new(ktx2_data).unwrap();
        let header = ktx2_reader.header();
        if header.supercompression_scheme.is_some() {
            panic!("[HOTHAM_TEXTURE] ktx2 supercompression is currently unsupported");
        }

        let extent = vk::Extent2D {
            width: header.pixel_width,
            height: header.pixel_height,
        };
        let format = get_format_from_ktx2(header.format);

        // We don't really support mipmaps with Hotham yet. So, we assume there is ONLY ONE mipmap level.
        let image_bytes = ktx2_reader.levels().next().unwrap();
        println!(
            "[HOTHAM_TEXTURE] Importing ktx2 texture in {:?} format",
            format
        );

        Texture::new(
            name,
            vulkan_context,
            render_context,
            image_bytes,
            &extent,
            format,
        )
    }

    /// Create a texture from an uncompressed image, like JPG or PNG.
    ///
    /// This is slow because we have to extract the image on the CPU before we can upload it to the GPU: hardly ideal. It is neccessary
    /// when testing in the simulator because compressing images into desktop friendly formats like BCn would be overkill for testing.
    pub fn from_uncompressed(
        name: &str,
        mime_type: &str,
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        data: &[u8],
    ) -> Self {
        #[cfg(not(target_os = "android"))]
        println!("[HOTHAM_TEXTURE] - @@ WARNING: Non-optimal image format detected. For best performance, compress your images into ktx2 using Squisher: https://github.com/leetvr/squisher. @@");

        print!("[HOTHAM_TEXTURE] - Decompresing image. This may take some time..");
        let decompressed_format = get_format_from_mime_type(mime_type);
        let asset = Cursor::new(data);
        let mut image = ImageReader::new(asset);
        image.set_format(decompressed_format);
        let image = image.decode().expect("Unable to decompress image!");
        let image = image.to_rgba8();
        let extent = vk::Extent2D {
            width: image.width(),
            height: image.height(),
        };

        println!(" ..done!");

        Texture::new(
            name,
            vulkan_context,
            render_context,
            &image.into_raw(),
            &extent,
            TEXTURE_FORMAT,
        )
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
