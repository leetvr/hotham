use hotham::{
    components::{GlobalTransform, LocalTransform, Mesh, Visible},
    contexts::RenderContext,
    glam::{vec2, DVec3, Vec3},
    hecs::World,
    rendering::{
        material::Material,
        mesh_data::MeshData,
        primitive::{calculate_bounding_sphere, Primitive},
        vertex::Vertex,
    },
};

pub(crate) fn create_mesh(
    render_context: &mut RenderContext,
    world: &mut World,
    points: &[DVec3],
    nx: usize,
) -> Mesh {
    puffin::profile_function!();
    let n: u32 = nx as _;
    let m: u32 = n - 1;
    let num_points: usize = (n * n * n) as _;
    assert_eq!(points.len(), num_points);
    let num_vertices: usize = (6 * n * n) as _;
    let num_indices: usize = (6 * m * m * 2) as _;
    let positions: Vec<Vec3> = vec![Default::default(); num_vertices];
    let vertices: Vec<Vertex> = vec![Default::default(); num_vertices];
    let mut indices: Vec<u32> = vec![Default::default(); num_indices];

    for side in 0..6 {
        for i in 0..m {
            for j in 0..m {
                indices.push(side * n * n + i * n + j);
                indices.push(side * n * n + i * n + j + 1);
                indices.push(side * n * n + i * n + j + 1 + n);
                indices.push(side * n * n + i * n + j);
                indices.push(side * n * n + i * n + j + 1 + n);
                indices.push(side * n * n + i * n + j + n);
            }
        }
    }

    let material_id = unsafe {
        render_context
            .resources
            .materials_buffer
            .push(&Material::gltf_default())
    };
    let mesh = Mesh::new(
        MeshData::new(vec![Primitive::new(
            positions.as_slice(),
            vertices.as_slice(),
            indices.as_slice(),
            material_id,
            render_context,
        )]),
        render_context,
    );
    update_mesh(&mesh, render_context, points, nx);
    let local_transform = LocalTransform {
        translation: [0., 0., 0.].into(),
        ..Default::default()
    };

    world.spawn((
        Visible {},
        mesh.clone(),
        local_transform,
        GlobalTransform::default(),
    ));

    mesh
}

pub(crate) fn update_mesh(
    mesh: &Mesh,
    render_context: &mut RenderContext,
    points: &[DVec3],
    nx: usize,
) {
    puffin::profile_function!();
    let n: i32 = nx as _;
    let m: i32 = n - 1;
    let num_vertices: usize = (6 * n * n) as _;
    let mut positions: Vec<Vec3> = Vec::<Vec3>::with_capacity(num_vertices);
    let mut vertices: Vec<Vertex> = Vec::<Vertex>::with_capacity(num_vertices);

    for side in 0..6 {
        for i in 0..n {
            for j in 0..n {
                let (x, y, z, dxdi, dydi, dzdi, dxdj, dydj, dzdj) = match side {
                    0 => (m, i, m - j, 0, 1, 0, 0, 0, -1),
                    1 => (j, m, m - i, 0, 0, -1, 1, 0, 0),
                    2 => (j, i, m, 0, 1, 0, 1, 0, 0),
                    3 => (0, j, m - i, 0, 0, -1, 0, 1, 0),
                    4 => (j, 0, i, 0, 0, 1, 1, 0, 0),
                    5 => (m - j, i, 0, 0, 1, 0, -1, 0, 0),
                    i32::MIN..=-1_i32 | 6_i32..=i32::MAX => todo!(),
                };
                let center = points[(z * n * n + y * n + x) as usize];
                positions.push(center.as_vec3());
                let x0 = (x - dxdi).clamp(0, m);
                let y0 = (y - dydi).clamp(0, m);
                let z0 = (z - dzdi).clamp(0, m);
                let x1 = (x + dxdi).clamp(0, m);
                let y1 = (y + dydi).clamp(0, m);
                let z1 = (z + dzdi).clamp(0, m);
                let x2 = (x - dxdj).clamp(0, m);
                let y2 = (y - dydj).clamp(0, m);
                let z2 = (z - dzdj).clamp(0, m);
                let x3 = (x + dxdj).clamp(0, m);
                let y3 = (y + dydj).clamp(0, m);
                let z3 = (z + dzdj).clamp(0, m);
                let p_down = points[(z0 * n * n + y0 * n + x0) as usize];
                let p_up = points[(z1 * n * n + y1 * n + x1) as usize];
                let p_left = points[(z2 * n * n + y2 * n + x2) as usize];
                let p_right = points[(z3 * n * n + y3 * n + x3) as usize];
                let normal = (p_right - p_left).cross(p_up - p_down).normalize_or_zero();
                let texture_coords = vec2(j as f32 / m as f32, i as f32 / m as f32);
                vertices.push(Vertex {
                    normal: normal.as_vec3(),
                    texture_coords,
                    ..Default::default()
                });
            }
        }
    }

    let mesh = render_context
        .resources
        .mesh_data
        .get_mut(mesh.handle)
        .unwrap();
    mesh.primitives[0].bounding_sphere = calculate_bounding_sphere(&positions);

    unsafe {
        std::ptr::copy_nonoverlapping(
            positions.as_ptr(),
            render_context
                .resources
                .position_buffer
                .memory_address
                .as_ptr()
                .offset(mesh.primitives[0].vertex_buffer_offset as _),
            positions.len(),
        );
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr(),
            render_context
                .resources
                .vertex_buffer
                .memory_address
                .as_ptr()
                .offset(mesh.primitives[0].vertex_buffer_offset as _),
            vertices.len(),
        );
    }
}
