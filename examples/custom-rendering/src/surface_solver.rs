use hotham::{
    components::LocalTransform,
    glam::{Mat4, Vec3},
    hecs::{Entity, World},
    nalgebra::{ArrayStorage, Matrix, U1, U10, U3},
    util::na_vector_from_glam,
    Engine,
};

use crate::hologram::Hologram;

type Matrix10x10 = Matrix<f32, U10, U10, ArrayStorage<f32, 10, 10>>;
type Matrix10x3 = Matrix<f32, U10, U3, ArrayStorage<f32, 10, 3>>;
type Vector10 = Matrix<f32, U10, U1, ArrayStorage<f32, 10, 1>>;
type RowVector10 = Matrix<f32, U1, U10, ArrayStorage<f32, 1, 10>>;

pub struct ControlPoints {
    pub entities: Vec<Entity>,
}

pub struct HologramBackside {
    pub entity: Entity,
}

pub fn surface_solver_system(engine: &mut Engine) {
    let world = &mut engine.world;
    surface_solver_system_inner(world);
}

fn surface_solver_system_inner(world: &mut World) {
    for (_, (hologram, control_points, local_transform)) in world
        .query::<(&mut Hologram, &mut ControlPoints, &mut LocalTransform)>()
        .iter()
    {
        let local_from_global = local_transform.to_affine().inverse();

        #[allow(non_snake_case)]
        let mut AtA: Matrix10x10 = Default::default();
        #[allow(non_snake_case)]
        let mut BtB: Matrix10x10 = Default::default();
        #[allow(non_snake_case)]
        let mut BtN: Vector10 = Default::default();

        for e in &control_points.entities {
            let t = world.get::<&LocalTransform>(*e).unwrap();
            let global_from_control = t.to_affine();
            let local_from_control = local_from_global * global_from_control;
            let point_in_local = local_from_control.transform_point3(Vec3::ZERO);
            let arrow_in_local = local_from_control.transform_vector3(Vec3::Y);
            let p = na_vector_from_glam(point_in_local);
            let d = na_vector_from_glam(arrow_in_local);

            let (x, y, z) = (p.x, p.y, p.z);
            let a_row: RowVector10 =
                [x * x, y * y, z * z, x * y, x * z, y * z, x, y, z, 1.0].into();
            AtA += a_row.transpose() * a_row;
            let b_rows_t = Matrix10x3::from_columns(&[
                [2.0 * x, 0.0, 0.0, y, z, 0.0, 1.0, 0.0, 0.0, 0.].into(),
                [0.0, 2.0 * y, 0.0, x, 0.0, z, 0.0, 1.0, 0.0, 0.].into(),
                [0.0, 0.0, 2.0 * z, 0.0, x, y, 0.0, 0.0, 1.0, 0.].into(),
            ]);
            BtB += b_rows_t * b_rows_t.transpose();
            BtN += b_rows_t * d;
        }
        // let eigen_decomposition = AtA.symmetric_eigen();
        // eigen_decomposition
        let eps = 1.0e-6;
        let svd = AtA.svd(false, true);
        let rank = svd.rank(eps).min(9);
        let v_t = svd.v_t.unwrap();
        let nullspace = v_t.rows(rank, 10 - rank);
        let svd_subspace = (nullspace * BtB * nullspace.transpose()).svd(true, true);
        let solution_in_subspace = svd_subspace.pseudo_inverse(eps).unwrap() * nullspace * BtN;
        let q = nullspace.transpose() * solution_in_subspace;
        hologram.hologram_data.surface_q_in_local = Mat4::from_cols_array_2d(&[
            [2.0 * q[0], q[3], q[4], q[6]],
            [q[3], 2.0 * q[1], q[5], q[7]],
            [q[4], q[5], 2.0 * q[2], q[8]],
            [q[6], q[7], q[8], 2.0 * q[9]],
        ]);
    }

    for (_, (target, source)) in world.query::<(&mut Hologram, &HologramBackside)>().iter() {
        target.hologram_data.surface_q_in_local = world
            .get::<&Hologram>(source.entity)
            .unwrap()
            .hologram_data
            .surface_q_in_local
            * -1.0;
    }
}
