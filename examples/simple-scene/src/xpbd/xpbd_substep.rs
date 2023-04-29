use itertools::izip;

use hotham::{glam::DVec3, hecs::World};

use super::{
    xpbd_collisions::resolve_ecs_collisions,
    xpbd_shape_constraints::{
        damping_of_shape_matching_constraints, resolve_shape_matching_constraints, ShapeConstraint,
    },
};

pub struct SimulationParams {
    pub dt: f64,
    pub acc: DVec3,
    pub particle_mass: f64,
    pub shape_compliance: f64, // Inverse of physical stiffness
    pub shape_damping: f64, // Linear damping towards rigid body motion, fraction of speed per second
    pub stiction_factor: f64, // Maximum tangential correction per correction along normal.
}

pub fn xpbd_substep(
    world: &mut World,
    velocities: &mut [DVec3],
    points: &mut [DVec3],
    shape_constraints: &mut [ShapeConstraint],
    &SimulationParams {
        dt,
        acc,
        particle_mass,
        shape_compliance,
        shape_damping,
        stiction_factor,
    }: &SimulationParams,
) {
    puffin::profile_function!();
    // Apply external forces
    {
        puffin::profile_scope!("Apply external forces");
        for vel in velocities.iter_mut() {
            *vel += acc * dt;
        }
    }

    // Predict new positions
    let points_prev = points.to_vec();
    {
        puffin::profile_scope!("Predict new positions");
        for (point, vel) in points.iter_mut().zip(velocities.iter()) {
            *point += *vel * dt;
        }
    };

    // TODO: Resolve distance constraints

    // Resolve shape matching constraints
    resolve_shape_matching_constraints(
        points,
        shape_constraints,
        shape_compliance,
        particle_mass.recip(),
        dt,
    );

    // Resolve collisions
    resolve_ecs_collisions(world, points, stiction_factor);

    // Update velocities
    {
        puffin::profile_scope!("update_velocities");
        for (vel, prev, point) in izip!(velocities.iter_mut(), points_prev.iter(), points.iter()) {
            *vel = (*point - *prev) / dt;
        }
    }

    damping_of_shape_matching_constraints(points, velocities, shape_constraints, shape_damping, dt);
}
