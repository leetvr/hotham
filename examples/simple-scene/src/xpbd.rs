use hotham::glam::Vec3;
use nalgebra::{self, Matrix3, Vector3};

use crate::utils::grid;

#[derive(Clone)]
pub struct ShapeConstraint(Vec<usize>, Vec<Vector3<f32>>, Matrix3<f32>);

#[derive(Clone)]
pub enum ContactState {
    New,
    Sticking,
    Sliding,
}

#[derive(Clone)]
pub struct Contact {
    pub contact_in_local: Vec3,
    pub state: ContactState,
}

pub fn create_points(center: Vec3, size: Vec3, nx: usize, ny: usize, nz: usize) -> Vec<Vec3> {
    let half_size = size * 0.5;
    grid(center - half_size, center + half_size, nx, ny, nz).collect::<Vec<_>>()
}

pub fn create_shape_constraints(
    points: &[Vec3],
    nx: usize,
    ny: usize,
    nz: usize,
) -> Vec<ShapeConstraint> {
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

pub fn resolve_collisions(
    points_next: &mut Vec<Vec3>,
    active_collisions: &mut Vec<Option<Contact>>,
) {
    let inner_r = 1.0;
    let inner_r2 = inner_r * inner_r;
    let outer_r = 5.0;
    let outer_r2 = outer_r * outer_r;
    let stiction_factor = 0.25; // Maximum tangential correction per correction along normal.

    for (p, c) in points_next.iter_mut().zip(active_collisions) {
        let d2 = p.length_squared();
        if d2 < inner_r2 {
            let length = p.length();
            *p *= inner_r / length;
            let stiction_d = (inner_r - length) * stiction_factor;
            let stiction_d2 = stiction_d * stiction_d;
            if let Some(Contact {
                contact_in_local: contact_point,
                state: contact_state,
            }) = c
            {
                if p.distance_squared(*contact_point) > stiction_d2 {
                    let delta = *p - *contact_point;
                    *p -= delta * (stiction_d * delta.length_recip());
                    *p *= inner_r / p.length();
                    *contact_point = *p;
                    *contact_state = ContactState::Sliding;
                } else {
                    *p = *contact_point;
                    *contact_state = ContactState::Sticking;
                }
            } else {
                *c = Some(Contact {
                    contact_in_local: *p,
                    state: ContactState::New,
                });
            }
        } else if d2 > outer_r2 {
            let length = p.length();
            *p *= outer_r / length;
            let stiction_d = (length - outer_r) * stiction_factor;
            let stiction_d2 = stiction_d * stiction_d;
            if let Some(Contact {
                contact_in_local: contact_point,
                state: contact_state,
            }) = c
            {
                if p.distance_squared(*contact_point) > stiction_d2 {
                    let delta = *p - *contact_point;
                    *p -= delta * (stiction_d * delta.length_recip());
                    *p *= outer_r / p.length();
                    *contact_point = *p;
                    *contact_state = ContactState::Sliding;
                } else {
                    *p = *contact_point;
                    *contact_state = ContactState::Sticking;
                }
            } else {
                *c = Some(Contact {
                    contact_in_local: *p,
                    state: ContactState::New,
                });
            }
        } else {
            *c = None;
        }
        // if p.z < -r {
        //     p.z = -r;
        // }
    }
}

pub fn resolve_shape_matching_constraints(
    points_next: &mut Vec<Vec3>,
    shape_constraints: &[ShapeConstraint],
    shape_compliance: f32,
    dt: f32,
) {
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
            let correction = delta * (-1.0 / (1.0 + shape_compliance_per_dt2));
            points_next[*ip] += correction;
        }
    }
}
