use std::time::Instant;

use hotham::{
    components::Mesh,
    glam::{dvec3, DVec3},
};

use crate::xpbd_shape_constraints::{create_points, create_shape_constraints, ShapeConstraint};

pub struct XpbdState {
    pub points_curr: Vec<DVec3>,
    pub velocities: Vec<DVec3>,
    pub shape_constraints: Vec<ShapeConstraint>,
    pub audio_emitter_indices: Vec<usize>,
    pub mesh: Option<Mesh>,

    pub simulation_time_epoch: Instant,
    pub simulation_time_hare: Instant,
    pub simulation_time_hound: Instant,
}

impl XpbdState {
    pub fn new(
        center: DVec3,
        size: DVec3,
        nx: usize,
        ny: usize,
        nz: usize,
        simulation_time: Instant,
    ) -> Self {
        let points_curr = create_points(center, size, nx, ny, nz);
        let shape_constraints = create_shape_constraints(&points_curr, nx, ny, nz);
        let velocities = vec![dvec3(0.0, 0.0, 0.0); points_curr.len()];

        let mesh = None;

        // Pick the corners as audio emitters
        let ix1 = 0;
        let ix2 = nx - 1;
        let iy1 = 0;
        let iy2 = ny - 1;
        let iz1 = 0;
        let iz2 = nx - 1;

        let audio_emitter_indices = vec![
            iz1 * nx * ny + iy1 * nx + ix1,
            iz1 * nx * ny + iy1 * nx + ix2,
            iz1 * nx * ny + iy2 * nx + ix1,
            iz1 * nx * ny + iy2 * nx + ix2,
            iz2 * nx * ny + iy1 * nx + ix1,
            iz2 * nx * ny + iy1 * nx + ix2,
            iz2 * nx * ny + iy2 * nx + ix1,
            iz2 * nx * ny + iy2 * nx + ix2,
        ];

        // Pick the sides as audio emitters
        // let mut audio_emitter_indices = Vec::new();
        // for iz in 0..NZ {
        //     for iy in 0..NY {
        //         for ix in 0..NX {
        //             if iz == 0 || iy == 0 || ix == 0 || ix == nx - 1 || iy == ny - 1 || iz == nz - 1
        //             {
        //                 audio_emitter_indices.push(iz * nx * ny + iy * nx + ix);
        //             }
        //         }
        //     }
        // }

        let simulation_time_epoch = simulation_time;
        XpbdState {
            points_curr,
            velocities,
            shape_constraints,
            audio_emitter_indices,
            mesh,
            simulation_time_hare: simulation_time_epoch,
            simulation_time_hound: simulation_time_epoch,
            simulation_time_epoch,
        }
    }
}
