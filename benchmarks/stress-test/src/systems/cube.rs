use hotham::{
    asset_importer::{add_model_to_world, Models},
    components::Transform,
    hecs::World,
    nalgebra::Vector3,
};

pub fn setup_cubes(world: &mut World, resolution: usize, models: &Models) {
    let step = 2. / resolution as f32;
    let scale_factor = 3.;
    let scale = Vector3::repeat(step / scale_factor);
    let half_resolution = resolution as f32 / 2.;
    let x_offset = half_resolution / scale_factor;

    for floor in 0..resolution {
        for row in 0..resolution {
            for column in 0..resolution {
                let c = add_model_to_world("Cube", models, world, None).unwrap();
                let mut t = world.get_mut::<Transform>(c).unwrap();
                t.scale = scale;
                t.translation.y = floor as f32 / scale_factor;
                t.translation.x = (column as f32 / scale_factor) - x_offset;
                t.translation.z = (row as f32 / scale_factor) - half_resolution - 2.0;
            }
        }
    }
}

// Different ways of arranging items
// fn circle(u: f32, v: f32, _t: f32) -> Vector3<f32> {
//     let x = (PI * u).sin();
//     let y = v;
//     let z = (PI * u).cos();
//     [x, y, z].into()
// }

// fn sphere(u: f32, v: f32, _t: f32) -> Vector3<f32> {
//     let r = 1.;
//     let s = r * (0.5 * PI * v).cos();

//     let x = s * (PI * u).sin();
//     let y = r * (PI * 0.5 * v).sin();
//     let z = s * (PI * u).cos();
//     [x, y, z].into()
// }

// fn _torus(u: f32, v: f32, _t: f32) -> Vector3<f32> {
//     let r1 = 0.75;
//     let r2 = 0.25;
//     let s = r1 + r2 * (PI * v).cos();

//     let x = s * (PI * u).sin();
//     let y = r2 * (PI * v).sin();
//     let z = s * (PI * u).cos();
//     [x, y, z].into()
// }
