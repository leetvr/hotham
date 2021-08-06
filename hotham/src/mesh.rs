use anyhow::{anyhow, Result};
use ash::vk;
use libktx_rs::{sources::StreamSource, RustKtxStream, TextureCreateFlags, TextureSource};

use crate::{buffer::Buffer, texture::Texture, vulkan_context::VulkanContext, Vertex};
use cgmath::{vec2, vec3, vec4};
use itertools::izip;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[cfg(target_os = "android")]
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Mesh {
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub(crate) index_buffer: Buffer<u32>,
    pub(crate) vertex_buffer: Buffer<Vertex>,
    pub num_indices: u32,
}

impl Mesh {
    pub(crate) fn load(
        mesh_data: &gltf::Mesh,
        buffers: &Vec<&[u8]>,
        vulkan_context: &VulkanContext,
        mesh_descriptor_set_layout: vk::DescriptorSetLayout,
        ubo_buffer: vk::Buffer,
    ) -> Result<Mesh> {
        let name = mesh_data.name().unwrap_or("");
        let mut indices = Vec::new();
        let mut positions = Vec::new();
        let mut tex_coords = Vec::new();
        let mut normals = Vec::new();
        let mut tangents = Vec::new();
        let mut joint_indices = Vec::new();
        let mut joint_weights = Vec::new();

        let mut normal_texture = None;
        let mut base_color_texture = None;

        for primitive in mesh_data.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            for v in reader
                .read_positions()
                .ok_or(anyhow!("Mesh {} has no positions!"))?
            {
                positions.push(vec3(v[0], v[1], v[2]));
            }

            if let Some(iter) = reader.read_normals() {
                for v in iter {
                    normals.push(vec3(v[0], v[1], v[2]));
                }
            }

            if let Some(iter) = reader.read_tex_coords(0) {
                for v in iter.into_f32() {
                    tex_coords.push(vec2(v[0], v[1]));
                }
            }

            if let Some(iter) = reader.read_indices() {
                for i in iter.into_u32() {
                    indices.push(i);
                }
            }

            if let Some(iter) = reader.read_tangents() {
                for t in iter {
                    tangents.push(vec4(t[0], t[1], t[2], t[3]));
                }
            }

            if let Some(iter) = reader.read_joints(0) {
                for t in iter.into_u16() {
                    joint_indices.push(vec4(t[0] as f32, t[1] as f32, t[2] as f32, t[3] as f32));
                }
            }

            if let Some(iter) = reader.read_weights(0) {
                for t in iter.into_f32() {
                    joint_weights.push(vec4(t[0] as f32, t[1] as f32, t[2] as f32, t[3] as f32));
                }
            }

            let material = primitive.material();
            let pbr = material
                .pbr_metallic_roughness()
                .base_color_texture()
                .unwrap()
                .texture()
                .source()
                .source();

            match pbr {
                gltf::image::Source::Uri { uri, .. } => {
                    let mut path = Path::new(uri).to_path_buf();
                    path.set_extension("ktx2");
                    let (buf, width, height) = parse_ktx(&path)?;
                    base_color_texture = Texture::new(&vulkan_context, &buf, width, height).ok();
                }
                _ => {}
            }

            let normal = material
                .normal_texture()
                .unwrap()
                .texture()
                .source()
                .source();

            match normal {
                gltf::image::Source::Uri { uri, .. } => {
                    let mut path = Path::new(uri).to_path_buf();
                    path.set_extension("ktx2");
                    let (buf, width, height) = parse_ktx(&path)?;
                    normal_texture = Texture::new(&vulkan_context, &buf, width, height).ok();
                }
                _ => {}
            }
        }

        if base_color_texture.is_none() {
            return Err(anyhow!("No colour texture found for {}", name));
        }

        if normal_texture.is_none() {
            return Err(anyhow!("No normal texture found for {}", name));
        }

        if tex_coords.is_empty() {
            for _ in 0..positions.len() {
                tex_coords.push(vec2(0.0, 0.0));
            }
        }

        if normals.is_empty() {
            for _ in 0..positions.len() {
                normals.push(vec3(0.0, 0.0, 0.0));
            }
        }

        if tangents.is_empty() {
            for _ in 0..positions.len() {
                tangents.push(vec4(0.0, 0.0, 0.0, 0.0));
            }
        }

        if joint_indices.is_empty() {
            for _ in 0..positions.len() {
                joint_indices.push(vec4(0.0, 0.0, 0.0, 0.0));
            }
        }

        if joint_weights.is_empty() {
            for _ in 0..positions.len() {
                joint_weights.push(vec4(1.0, 0.0, 0.0, 0.0));
            }
        }

        let vertices: Vec<Vertex> = izip!(
            positions,
            tex_coords,
            normals,
            tangents,
            joint_indices,
            joint_weights
        )
        .into_iter()
        .map(Vertex::from_zip)
        .collect();

        println!("[HOTHAM_MODEL] {} parsed! Creating vertex buffers..", name);
        // Create buffers
        let vertex_buffer = Buffer::new_from_vec(
            &vulkan_context,
            &vertices,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = Buffer::new_from_vec(
            &vulkan_context,
            &indices,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;
        println!("[HOTHAM_MODEL] ..done!");

        // Create descriptor sets
        println!("[HOTHAM_MODEL] Creating descriptor sets for {}", name);
        let descriptor_sets = vulkan_context.create_mesh_descriptor_set(
            mesh_descriptor_set_layout,
            ubo_buffer,
            &base_color_texture.unwrap(),
            &normal_texture.unwrap(),
        )?;
        println!("[HOTHAM_MODEL] ..done!");

        Ok(Mesh {
            vertex_buffer,
            index_buffer,
            descriptor_sets,
            num_indices: indices.len() as u32,
        })
    }
}

pub fn parse_ktx(path: &PathBuf) -> Result<(Vec<u8>, u32, u32)> {
    let file = get_ktx_file(path)?;
    let stream = RustKtxStream::new(file).map_err(|e| anyhow!("Couldn't create stream: {}", e))?;
    let source = Arc::new(Mutex::new(stream));
    let texture = StreamSource::new(source, TextureCreateFlags::LOAD_IMAGE_DATA)
        .create_texture()
        .unwrap();

    let image_buf = texture.data().to_vec();
    let (image_height, image_width, _size) = unsafe {
        let ktx_texture = texture.handle();
        (
            (*ktx_texture).baseHeight,
            (*ktx_texture).baseWidth,
            (*ktx_texture).dataSize,
        )
    };

    Ok((image_buf, image_width, image_height))
}

#[cfg(target_os = "windows")]
fn get_ktx_file(file_name: &PathBuf) -> Result<Box<std::fs::File>> {
    use anyhow::Context;
    use std::fs::OpenOptions;
    let file_name = file_name
        .to_str()
        .ok_or(anyhow!("Unable to convert {:?} to string", file_name))?;
    let path = format!(
        "C:\\Users\\kanem\\Development\\hotham\\test_assets\\{}",
        file_name
    );
    let path = Path::new(&path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .context(format!("{:?}", path))?;
    Ok(Box::new(file))
}

#[cfg(target_os = "android")]
fn get_ktx_file(path: &PathBuf) -> Result<Box<Cursor<Vec<u8>>>> {
    use crate::util::get_asset_from_path;
    let path = path
        .to_str()
        .ok_or(anyhow!("Unable to convert {:?} to string", path))?;
    let asset = get_asset_from_path(path)?;

    // delicious
    Ok(Box::new(Cursor::new(asset)))
}
