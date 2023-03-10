use hotham::glam::Vec3;
use nalgebra::{self, Matrix3, Vector3};

use crate::utils::grid;

#[derive(Clone)]
pub struct ShapeConstraint(Vec<usize>, Vec<Vector3<f32>>, Matrix3<f32>);

pub fn create_points(center: Vec3, size: Vec3, nx: usize, ny: usize, nz: usize) -> Vec<Vec3> {
    puffin::profile_function!();
    let half_size = size * 0.5;
    grid(center - half_size, center + half_size, nx, ny, nz).collect::<Vec<_>>()
}

pub fn create_shape_constraints(
    points: &[Vec3],
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
                let mean: Vector3<f32> = ips
                    .iter()
                    .map(|&ip| Vector3::from(points[ip]))
                    .fold(Vector3::zeros(), |acc, p| acc + p)
                    / ips.len() as f32;
                let shape: Vec<Vector3<f32>> = ips
                    .iter()
                    .map(|&ip| Vector3::from(points[ip]) - mean)
                    .collect();
                let a_qq_inv = shape
                    .iter()
                    .fold(Matrix3::zeros(), |acc, q| acc + q * q.transpose())
                    .try_inverse()
                    .unwrap();
                constraints.push(ShapeConstraint(ips.to_vec(), shape, a_qq_inv));
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
    points_next: &mut [Vec3],
    shape_constraints: &[ShapeConstraint],
    shape_compliance: f32,
    inv_particle_mass: f32,
    dt: f32,
) {
    puffin::profile_function!();
    let shape_compliance_per_dt2 = shape_compliance / (dt * dt);
    for ShapeConstraint(ips, template_shape, a_qq_inv) in shape_constraints {
        let mean: Vector3<f32> = ips
            .iter()
            .map(|&ip| Vector3::from(points_next[ip]))
            .fold(Vector3::zeros(), |acc, p| acc + p)
            / ips.len() as f32;
        let a_pq = ips
            .iter()
            .map(|&ip| Vector3::from(points_next[ip]) - mean)
            .zip(template_shape)
            .fold(Matrix3::zeros(), |acc, (p, q)| acc + p * q.transpose());
        let mut svd = (a_pq * a_qq_inv).svd(true, true);
        svd.singular_values[0] = 1.0;
        svd.singular_values[1] = 1.0;
        svd.singular_values[2] =
            (svd.u.unwrap().determinant() * svd.v_t.unwrap().determinant()).signum();
        let rot = svd.recompose().unwrap();
        for (i, ip) in ips.iter().enumerate() {
            let goal = Vec3::from(mean + rot * template_shape[i]);
            let delta = points_next[*ip] - goal;
            let correction =
                delta * (-inv_particle_mass / (inv_particle_mass + shape_compliance_per_dt2));
            points_next[*ip] += correction;
        }
    }
}
