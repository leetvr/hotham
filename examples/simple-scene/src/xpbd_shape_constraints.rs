use hotham::glam::{dmat3, dvec3, DVec3};
use nalgebra::{self, Matrix3, Unit, UnitQuaternion, Vector3};

use crate::utils::grid;

#[derive(Clone)]
pub struct ShapeConstraint {
    point_indices: [usize; 8],
    template_shape: [Vector3<f64>; 8],
    a_qq_inv: Matrix3<f64>,
    pub cached_rot: UnitQuaternion<f64>,
}

pub fn create_points(center: DVec3, size: DVec3, nx: usize, ny: usize, nz: usize) -> Vec<DVec3> {
    puffin::profile_function!();
    let half_size = size * 0.5;
    grid(center - half_size, center + half_size, nx, ny, nz).collect::<Vec<_>>()
}

pub fn create_shape_constraints(
    points: &[DVec3],
    nx: usize,
    ny: usize,
    nz: usize,
) -> Vec<ShapeConstraint> {
    puffin::profile_function!();
    let mut constraints = Vec::<ShapeConstraint>::with_capacity(
        nx * ny * (nz - 1) + nx * (ny - 1) * nz + (nx - 1) * ny * nz,
    );
    // Loop over blocks of vertices
    for iz2 in 1..nz {
        let iz1 = iz2 - 1;
        for iy2 in 1..ny {
            let iy1 = iy2 - 1;
            for ix2 in 1..nx {
                let ix1 = ix2 - 1;
                let ips = [
                    iz1 * nx * ny + iy1 * nx + ix1,
                    iz1 * nx * ny + iy1 * nx + ix2,
                    iz1 * nx * ny + iy2 * nx + ix1,
                    iz1 * nx * ny + iy2 * nx + ix2,
                    iz2 * nx * ny + iy1 * nx + ix1,
                    iz2 * nx * ny + iy1 * nx + ix2,
                    iz2 * nx * ny + iy2 * nx + ix1,
                    iz2 * nx * ny + iy2 * nx + ix2,
                ];
                let mean: Vector3<f64> = ips
                    .iter()
                    .map(|&ip| Vector3::from(points[ip]))
                    .fold(Vector3::zeros(), |acc, p| acc + p)
                    / ips.len() as f64;
                let shape = ips.map(|ip| Vector3::from(points[ip]) - mean);
                let a_qq_inv = shape
                    .iter()
                    .fold(Matrix3::zeros(), |acc, q| acc + q * q.transpose())
                    .try_inverse()
                    .unwrap();
                constraints.push(ShapeConstraint {
                    point_indices: ips,
                    template_shape: shape,
                    a_qq_inv,
                    cached_rot: Default::default(),
                });
            }
        }
    }
    constraints
}

// 𝛼 = compliance = inverse physical stiffness
// C = constraint error (scalar)
// ∇𝐶ᵢ = constraint gradient wrt particle i (vector) = How to move 𝐱ᵢ for a maximal increase of C
// 𝐱ᵢ = position of particle i
// ∆𝐱ᵢ = correction of particle i
// 𝑤ᵢ = inverse mass of particle i
//                      -C 𝑤ᵢ∇𝐶ᵢ
// ∆𝐱ᵢ = ―――――――――――――――――――――――――――――――――――――――――――――
//        𝑤₁|∇𝐶₁|² + 𝑤₂|∇𝐶₂|² + ⋯ + 𝑤ₙ|∇𝐶ₙ|² + 𝛼/∆𝑡²
//                        -C
// λ = ―――――――――――――――――――――――――――――――――――――――――――――
//      𝑤₁|∇𝐶₁|² + 𝑤₂|∇𝐶₂|² + ⋯ + 𝑤ₙ|∇𝐶ₙ|² + 𝛼/∆𝑡²
//
// ∆𝐱ᵢ = λ 𝑤ᵢ∇𝐶ᵢ
pub fn resolve_shape_matching_constraints(
    points_next: &mut [DVec3],
    shape_constraints: &mut [ShapeConstraint],
    shape_compliance: f64,
    inv_particle_mass: f64,
    dt: f64,
) {
    puffin::profile_function!();
    const MAX_ITER: usize = 4;
    const EPS: f64 = 1.0e-8;
    let shape_compliance_per_dt2 = shape_compliance / (dt * dt);
    for ShapeConstraint {
        point_indices: ips,
        template_shape,
        a_qq_inv,
        cached_rot,
    } in shape_constraints
    {
        let mean: Vector3<f64> = ips
            .iter()
            .map(|&ip| Vector3::from(points_next[ip]))
            .fold(Vector3::zeros(), |acc, p| acc + p)
            / ips.len() as f64;
        let a_pq = ips
            .iter()
            .map(|&ip| Vector3::from(points_next[ip]) - mean)
            .zip(template_shape.iter())
            .fold(Matrix3::zeros(), |acc, (p, q)| acc + p * q.transpose());
        extract_rotation(&(a_pq * *a_qq_inv), cached_rot, MAX_ITER, EPS);
        let rot = cached_rot.to_rotation_matrix();
        for (i, ip) in ips.iter().enumerate() {
            let goal = DVec3::from(mean + rot * template_shape[i]);
            let delta = points_next[*ip] - goal;
            let correction =
                delta * (-inv_particle_mass / (inv_particle_mass + shape_compliance_per_dt2));
            points_next[*ip] += correction;
        }
    }
}

// nalgebra has a similar implementation in UnitQuaternion::<f64>::from_matrix_eps but this is simpler and faster!
fn extract_rotation(a: &Matrix3<f64>, q: &mut UnitQuaternion<f64>, max_iter: usize, eps: f64) {
    // puffin::profile_function!();
    for _iter in 0..max_iter {
        let r = q.to_rotation_matrix();
        let r = r.matrix();
        let omega = (r.column(0).cross(&a.column(0))
            + r.column(1).cross(&a.column(1))
            + r.column(2).cross(&a.column(2)))
            * (1.0
                / (r.column(0).dot(&a.column(0))
                    + r.column(1).dot(&a.column(1))
                    + r.column(2).dot(&a.column(2)))
                .abs()
                + 1.0e-9);
        let (omega, w) = Unit::new_and_get(omega);
        if w < eps {
            break;
        }
        *q = UnitQuaternion::<f64>::from_axis_angle(&omega, w) * *q;
    }
    q.renormalize();
}

pub fn damping_of_shape_matching_constraints(
    points: &[DVec3],
    velocities: &mut [DVec3],
    shape_constraints: &[ShapeConstraint],
    shape_damping: f64,
    dt: f64,
) {
    puffin::profile_function!();
    let shape_damping_times_dt = (shape_damping * dt).min(1.0);
    for ShapeConstraint {
        point_indices: ips, ..
    } in shape_constraints
    {
        let mean_pos: DVec3 = ips
            .iter()
            .map(|&ip| points[ip])
            .fold(DVec3::ZERO, |acc, p| acc + p)
            / ips.len() as f64;
        let mean_vel: DVec3 = ips
            .iter()
            .map(|&ip| velocities[ip])
            .fold(DVec3::ZERO, |acc, v| acc + v)
            / ips.len() as f64;
        let mut angular_momentum = DVec3::ZERO;
        let mut acc_rx2 = 0.0;
        let mut acc_ry2 = 0.0;
        let mut acc_rz2 = 0.0;
        let mut acc_rxy = 0.0;
        let mut acc_rxz = 0.0;
        let mut acc_ryz = 0.0;
        for &ip in ips {
            let r = points[ip] - mean_pos;
            let v = velocities[ip] - mean_vel;
            angular_momentum += r.cross(v);
            //        | r.y²+r.z²   -r.x·r.y    -r.x·r.z  |
            //  RRᵀ = | -r.x·r.y    r.z²+r.x²   -r.yr.z   |
            //        | -r.x·r.z    -r.yr.z     r.x²+r.y² |
            acc_rx2 += r.x * r.x;
            acc_ry2 += r.y * r.y;
            acc_rz2 += r.z * r.z;
            acc_rxy += r.x * r.y;
            acc_rxz += r.x * r.z;
            acc_ryz += r.y * r.z;
        }
        let angular_mass = dmat3(
            dvec3(acc_ry2 + acc_rz2, -acc_rxy, -acc_rxz),
            dvec3(-acc_rxy, acc_rz2 + acc_rx2, -acc_ryz),
            dvec3(-acc_rxz, -acc_ryz, acc_rx2 + acc_ry2),
        );
        let angular_velocity = angular_mass.inverse() * angular_momentum;
        for &ip in ips {
            let r = points[ip] - mean_pos;
            let v_bar = mean_vel + angular_velocity.cross(r);
            let v = &mut velocities[ip];
            *v += shape_damping_times_dt * (v_bar - *v);
        }
    }
}
