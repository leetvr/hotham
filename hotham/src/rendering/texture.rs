use std::io::{Cursor, Read};

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
    /// How the texture will be used
    pub texture_usage: TextureUsage,
}

/// Describes how this texture will be used by the fragment shader.
/// Corresponds to the glTF PBR model: https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#materials
#[derive(Debug, Clone)]
pub enum TextureUsage {
    /// The base color of the material
    BaseColor,
    /// A tangent space normal texture
    Normal,
    /// The color and intensity of the light being emitted by the material
    Emission,
    /// The metalness and roughness of the material
    MetallicRoughness,
    /// Indicates areas that receive less less indirect light from ambient sources
    Occlusion,
    /// Indicates this texture is used for Image Based Lighting (IBL)
    IBL,
    /// A non PBR texture
    Other,
}

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
        array_layers: u32,
        format: vk::Format,
        texture_usage: TextureUsage,
    ) -> Self {
        let component_mapping = get_component_mapping(&format, &texture_usage);

        let image = vulkan_context
            .create_image_with_component_mapping(
                format,
                extent,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                array_layers,
                1,
                component_mapping,
            )
            .unwrap();

        let index = render_context
            .create_texture_image(
                name,
                vulkan_context,
                image_buf,
                1,
                vec![image_buf.len() as u64 / array_layers as u64],
                &image,
            )
            .unwrap();

        Texture {
            image,
            index,
            texture_usage,
        }
    }

    /// Load a texture from a glTF document. Returns the texture ID
    pub(crate) fn load(
        texture: gltf::texture::Texture,
        texture_usage: TextureUsage,
        import_context: &mut ImportContext,
    ) -> u32 {
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
                        texture_usage,
                    ),
                    _ => Texture::from_uncompressed(
                        texture_name,
                        mime_type,
                        import_context.vulkan_context,
                        import_context.render_context,
                        bytes,
                        texture_usage,
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

        Texture {
            image,
            index,
            texture_usage: TextureUsage::Other,
        }
    }

    /// Create a texture from a ktx2 container.
    ///
    /// This is the preferred way of using textures in Hotham. By compressing the image in a GPU friendly way we can drastically reduce the amount of
    /// bandwidth used to sample from it.
    ///
    /// Cube arrays are unimplemented.
    pub fn from_ktx2(
        name: &str,
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        ktx2_data: &[u8],
        texture_usage: TextureUsage,
    ) -> Self {
        let ktx2_image = parse_ktx2(ktx2_data);

        Texture::new(
            name,
            vulkan_context,
            render_context,
            &ktx2_image.image_buf,
            &ktx2_image.extent,
            ktx2_image.array_layers.max(1) * ktx2_image.faces,
            ktx2_image.format,
            texture_usage,
        )
    }

    /// Create a texture from an uncompressed image, like JPG or PNG.
    ///
    /// This is slow because we have to extract the image on the CPU before we can upload it to the GPU: hardly ideal. It is necessary
    /// when testing in the simulator because compressing images into desktop friendly formats like BCn would be overkill for testing.
    pub fn from_uncompressed(
        name: &str,
        mime_type: &str,
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        data: &[u8],
        texture_usage: TextureUsage,
    ) -> Self {
        #[cfg(target_os = "android")]
        println!("[HOTHAM_TEXTURE] - @@ WARNING: Non-optimal image format detected. For best performance, compress your images into ktx2 using Squisher: https://github.com/leetvr/squisher. @@");

        print!("[HOTHAM_TEXTURE] - Decompressing image. This may take some time..");
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

        let format = match texture_usage {
            TextureUsage::BaseColor | TextureUsage::Emission => vk::Format::R8G8B8A8_SRGB,
            _ => vk::Format::R8G8B8A8_UNORM,
        };

        println!(" ..done!");

        Texture::new(
            name,
            vulkan_context,
            render_context,
            &image.into_raw(),
            &extent,
            1,
            format,
            texture_usage,
        )
    }
}

// Thin wrapper containing the information we need from a KTX2 file.
#[derive(Debug, Clone)]
pub(crate) struct KTX2Image {
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub image_buf: Vec<u8>,
    pub offsets: Vec<vk::DeviceSize>,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub faces: u32,
}

pub(crate) fn parse_ktx2(ktx2_data: &[u8]) -> KTX2Image {
    let ktx2_reader = ktx2::Reader::new(ktx2_data).unwrap();
    let header = ktx2_reader.header();
    let extent = vk::Extent2D {
        width: header.pixel_width,
        height: header.pixel_height,
    };
    let mut image_buf = Vec::new();
    let mut offsets = Vec::new();

    println!(
        "[HOTHAM_TEXTURE] Importing KTX2 texture in {:?} format.",
        header.format
    );
    for level in ktx2_reader.levels() {
        let len = match header.supercompression_scheme {
            // Lifted from Bevy, with Love:
            // https://github.com/bevyengine/bevy/blob/05e5008624b35f51cd6418acc745236be2cddd28/crates/bevy_render/src/texture/ktx2.rs#L62
            Some(ktx2::SupercompressionScheme::Zstandard) => {
                let mut cursor = std::io::Cursor::new(level);
                let mut decoder = ruzstd::StreamingDecoder::new(&mut cursor).unwrap();
                decoder.read_to_end(&mut image_buf).unwrap()
            }
            None => {
                image_buf.extend(level);
                level.len()
            }
            s => panic!(
                "Unable to parse KTX2 file, unsupported supercompression scheme: {:?}",
                s
            ),
        };

        let offset_increment = len as u32 / header.face_count;
        offsets.push(offset_increment as _);
    }

    KTX2Image {
        format: get_format_from_ktx2(header.format),
        extent,
        image_buf,
        offsets,
        mip_levels: header.level_count,
        array_layers: header.layer_count,
        faces: header.face_count,
    }
}

fn get_component_mapping(
    format: &vk::Format,
    texture_usage: &TextureUsage,
) -> vk::ComponentMapping {
    let uncompressed =
        *format == vk::Format::R8G8B8A8_SRGB || *format == vk::Format::R8G8B8A8_UNORM;
    match (uncompressed, texture_usage) {
        (true, TextureUsage::Normal) => vk::ComponentMapping {
            r: vk::ComponentSwizzle::ZERO,
            g: vk::ComponentSwizzle::R,
            b: vk::ComponentSwizzle::ZERO,
            a: vk::ComponentSwizzle::G,
        },
        (true, TextureUsage::MetallicRoughness) => vk::ComponentMapping {
            r: vk::ComponentSwizzle::ZERO,
            g: vk::ComponentSwizzle::G,
            b: vk::ComponentSwizzle::ZERO,
            a: vk::ComponentSwizzle::B,
        },
        (true, TextureUsage::Occlusion) => vk::ComponentMapping {
            r: vk::ComponentSwizzle::ZERO,
            g: vk::ComponentSwizzle::R,
            b: vk::ComponentSwizzle::ZERO,
            a: vk::ComponentSwizzle::ZERO,
        },
        _ => DEFAULT_COMPONENT_MAPPING,
    }
}

/// The identity swizzle
pub const DEFAULT_COMPONENT_MAPPING: vk::ComponentMapping = vk::ComponentMapping {
    r: vk::ComponentSwizzle::IDENTITY,
    g: vk::ComponentSwizzle::IDENTITY,
    b: vk::ComponentSwizzle::IDENTITY,
    a: vk::ComponentSwizzle::IDENTITY,
};

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
pub(crate) fn get_format_from_ktx2(format: Option<ktx2::Format>) -> vk::Format {
    let raw = format.expect("No format specified").0;
    vk::Format::from_raw(raw.get() as _)
}
