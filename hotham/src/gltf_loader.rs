use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ash::vk;
use cgmath::{vec2, vec3, vec4, Matrix4};
use itertools::izip;
use libktx_rs::{sources::StreamSource, RustKtxStream, TextureCreateFlags, TextureSource};

use crate::{
    buffer::Buffer, mesh::Mesh, node::Node, texture::Texture, vulkan_context::VulkanContext, Vertex,
};
use anyhow::{anyhow, Result};

pub(crate) fn load_gltf_nodes(
    gltf_bytes: &[u8],
    data_bytes: &[u8],
    vulkan_context: &VulkanContext,
    set_layouts: &[vk::DescriptorSetLayout],
    ubo_buffer: vk::Buffer,
) -> Result<HashMap<String, Node>> {
    let gtlf_buf = Cursor::new(gltf_bytes);
    let gltf = gltf::Gltf::from_reader(gtlf_buf)?;
    let document = gltf.document;
    let blob = data_bytes;

    let mut nodes = HashMap::new();

    for node_data in document.nodes() {
        let name = node_data.name().unwrap();
        let mut indices = Vec::new();
        let mut positions = Vec::new();
        let mut tex_coords = Vec::new();
        let mut normals = Vec::new();
        let mut tangents = Vec::new();
        let mut normal_texture = None;
        let mut base_color_texture = None;

        if let Some(mesh) = node_data.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive
                    .reader(|buffer| Some(&blob[buffer.index()..buffer.index() + buffer.length()]));

                for v in reader
                    .read_positions()
                    .ok_or(anyhow!("Mesh {} has no positions!"))?
                {
                    positions.push(vec3(v[0], v[1], v[2]));
                }

                for v in reader
                    .read_normals()
                    .ok_or(anyhow!("Mesh {} has no normals!"))?
                {
                    normals.push(vec3(v[0], v[1], v[2]));
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

                for t in reader
                    .read_tangents()
                    .ok_or(anyhow!("Mesh {} has no tangents!"))?
                {
                    tangents.push(vec4(t[0], t[1], t[2], t[3]));
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
                        base_color_texture =
                            Texture::new(&vulkan_context, &buf, width, height).ok();
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
        }

        if base_color_texture.is_none() {
            return Err(anyhow!("No colour texture found for {}", name));
        }

        if normal_texture.is_none() {
            return Err(anyhow!("No normal texture found for {}", name));
        }

        let vertices: Vec<Vertex> = izip!(positions, tex_coords, normals, tangents)
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
        let descriptor_sets = vulkan_context.create_descriptor_sets(
            set_layouts,
            ubo_buffer,
            &base_color_texture.unwrap(),
            &normal_texture.unwrap(),
        )?;
        println!("[HOTHAM_MODEL] ..done!");

        let transform = Matrix4::from(node_data.transform().matrix());

        let mesh = Mesh {
            vertex_buffer: vertex_buffer.handle,
            index_buffer: index_buffer.handle,
            descriptor_sets,
            num_indices: indices.len() as u32,
            transform,
        };

        // nodes.insert(name.to_string(), mesh);
    }

    Ok(nodes)
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
    use std::ffi::CStr;
    let native_activity = ndk_glue::native_activity();

    let asset_manager = native_activity.asset_manager();
    let path = path.to_str().ok_or(anyhow!("Can't parse string!"))?;
    let path_with_nul = format!("{}\0", path);
    let path = unsafe { CStr::from_bytes_with_nul_unchecked(path_with_nul.as_bytes()) };

    let mut asset = asset_manager
        .open(path)
        .ok_or(anyhow!("Can't open: {:?}", path))?;

    // delicious
    Ok(Box::new(Cursor::new(asset.get_buffer()?.to_vec())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{renderer::create_descriptor_set_layout, vulkan_context::VulkanContext};
    #[test]
    pub fn test_load_models() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let gltf = include_bytes!(
            "C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid.gltf"
        );
        let data = include_bytes!(
            "C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid_data.bin"
        );
        let set_layout = create_descriptor_set_layout(&vulkan_context).unwrap();
        let buffer = vk::Buffer::null();
        let models = load_gltf_nodes(gltf, data, &vulkan_context, &[set_layout], buffer).unwrap();
        assert!(models.len() != 0);
    }
}
