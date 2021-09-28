use crate::{image::Image, resources::VulkanContext};
use anyhow::{anyhow, Result};
use ash::vk;
use image::io::Reader as ImageReader;
use libktx_rs::{sources::StreamSource, RustKtxStream, TextureCreateFlags, TextureSource};
use std::{
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct Texture {
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor: vk::DescriptorImageInfo,
}

const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

impl Texture {
    pub fn new(
        name: &str,
        vulkan_context: &VulkanContext,
        image_buf: &Vec<u8>,
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Result<Self> {
        let (image, sampler) = vulkan_context.create_texture_image(
            name,
            image_buf,
            width,
            height,
            format,
            1,
            1,
            vec![0],
        )?;
        let descriptor = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image.view)
            .sampler(sampler)
            .build();

        Ok(Texture {
            image,
            sampler,
            descriptor,
        })
    }

    pub fn load(
        mesh_name: &str,
        texture: gltf::texture::Texture,
        vulkan_context: &VulkanContext,
        images: &Vec<gltf::image::Data>,
    ) -> Option<Self> {
        let texture_name = &format!(
            "Texture {} for mesh {}",
            texture.name().unwrap_or(""),
            mesh_name
        );
        match texture.source().source() {
            gltf::image::Source::Uri { uri, .. } => {
                let (buf, width, height) =
                    parse_image(&uri).expect(&format!("Unable to load image! URI: {}", uri));
                Some(
                    Texture::new(
                        texture_name,
                        &vulkan_context,
                        &buf,
                        width,
                        height,
                        TEXTURE_FORMAT,
                    )
                    .unwrap(),
                )
            }
            gltf::image::Source::View { .. } => {
                let index = texture.source().index();
                let image = &images[index];
                let pixels = add_alpha_channel(&image);
                Texture::new(
                    texture_name,
                    &vulkan_context,
                    &pixels,
                    image.width,
                    image.height,
                    TEXTURE_FORMAT,
                )
                .map_err(|e| eprintln!("Failed to load texture {} - {:?}", index, e))
                .ok()
            }
        }
    }

    pub fn empty(vulkan_context: &VulkanContext) -> Result<Self> {
        Self::new(
            "Empty Texture",
            vulkan_context,
            &EMPTY_KTX.to_vec(),
            1,
            1,
            TEXTURE_FORMAT,
        )
    }

    pub fn from_ktx2(name: &str, buf: &[u8], vulkan_context: &VulkanContext) -> Result<Self> {
        let buf = Cursor::new(buf.to_vec());
        let buf = Box::new(buf);
        let (buf, width, height, format, array_layers, mip_levels, offsets) = parse_ktx(buf)?;
        println!(
            "Creating texture image with format {:?}, array layers {} and mip_levels {}",
            format, array_layers, mip_levels
        );

        let (image, sampler) = vulkan_context.create_texture_image(
            name,
            &buf,
            width,
            height,
            format,
            array_layers,
            mip_levels,
            offsets,
        )?;
        let descriptor = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image.view)
            .sampler(sampler)
            .build();

        Ok(Texture {
            image,
            sampler,
            descriptor,
        })
    }
}

#[cfg(target_os = "windows")]
fn parse_image(path: &str) -> Result<(Vec<u8>, u32, u32)> {
    let path = format!(r#"..\test_assets\\{}"#, path);
    let img = ImageReader::open(path)?.decode()?;
    let img = img.to_rgba8();
    let width = img.width();
    let height = img.height();
    return Ok((img.into_raw(), width, height));
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

#[cfg(target_os = "windows")]
fn _get_ktx_file(file_name: &PathBuf) -> Result<Box<std::fs::File>> {
    use anyhow::Context;
    use std::{fs::OpenOptions, path::Path};
    let file_name = file_name
        .to_str()
        .ok_or(anyhow!("Unable to convert {:?} to string", file_name))?;
    let path = format!(r#"..\test_assets\\{}"#, file_name);
    let path = Path::new(&path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .context(format!("{:?}", path))?;
    Ok(Box::new(file))
}

pub fn parse_ktx(
    buf: Box<Cursor<Vec<u8>>>,
) -> Result<(Vec<u8>, u32, u32, vk::Format, u32, u32, Vec<vk::DeviceSize>)> {
    let stream = RustKtxStream::new(buf).map_err(|e| anyhow!("Couldn't create stream: {}", e))?;
    let source = Arc::new(Mutex::new(stream));
    let mut texture = StreamSource::new(source, TextureCreateFlags::LOAD_IMAGE_DATA)
        .create_texture()
        .unwrap();

    let image_buf = texture.data().to_vec();
    let mut offsets = Vec::new();
    let (image_height, image_width, layer_count, mip_count) = unsafe {
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

    Ok((
        image_buf,
        image_width,
        image_height,
        format,
        layer_count,
        mip_count,
        offsets,
    ))
}

#[cfg(target_os = "android")]
fn _get_ktx_file(path: &PathBuf) -> Result<Box<Cursor<Vec<u8>>>> {
    use crate::util::get_asset_from_path;
    let path = path
        .to_str()
        .ok_or(anyhow!("Unable to convert {:?} to string", path))?;
    let asset = get_asset_from_path(path)?;

    // delicious
    Ok(Box::new(Cursor::new(asset)))
}

fn add_alpha_channel(image: &gltf::image::Data) -> Vec<u8> {
    let final_size = (image.height * image.width) * 4;
    let mut final_image = vec![0; final_size as _];
    let original_image = &image.pixels;
    let mut original_index = 0;
    let mut final_index = 0;
    while original_index < original_image.len() {
        for i in 0..3 {
            final_image[final_index + i] = original_image[original_index + i];
        }
        final_image[final_index + 3] = 1;

        original_index += 3;
        final_index += 4;
    }

    final_image
}

const EMPTY_KTX: [u8; 104] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x31, 0x31, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A, 0x01, 0x02, 0x03, 0x04,
    0x01, 0x14, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x19, 0x00, 0x00, 0x58, 0x80, 0x00, 0x00,
    0x08, 0x19, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
    0x1B, 0x00, 0x00, 0x00, 0x4B, 0x54, 0x58, 0x4F, 0x72, 0x69, 0x65, 0x6E, 0x74, 0x61, 0x74, 0x69,
    0x6F, 0x6E, 0x00, 0x53, 0x3D, 0x72, 0x2C, 0x54, 0x3D, 0x64, 0x2C, 0x52, 0x3D, 0x69, 0x00, 0x00,
    0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
];
