use hotham::{
    components::{Collider, GlobalTransform},
    glam::{vec3, Vec3},
    hecs::World,
    na,
};

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

pub struct XpbdCollisions {
    pub active_collisions: Vec<Option<Contact>>,
}

pub fn resolve_ecs_collisions(world: &mut World, points_next: &mut [Vec3], stiction_factor: f32) {
    puffin::profile_function!();
    for (_, (transform, collider, collisions)) in world
        .query_mut::<(Option<&GlobalTransform>, &Collider, &mut XpbdCollisions)>()
        .into_iter()
    {
        let m = match transform {
            Some(transform) => transform.to_isometry(),
            None => Default::default(),
        };

        for (p_global, c) in points_next
            .iter_mut()
            .zip(&mut collisions.active_collisions)
        {
            let pt_local =
                m.inverse_transform_point(&na::Point3::new(p_global.x, p_global.y, p_global.z));
            let proj_local = collider.shape.project_local_point(&pt_local, false);
            if proj_local.is_inside {
                let mut p_local = vec3(pt_local.x, pt_local.y, pt_local.z);
                let point_on_surface_in_local =
                    vec3(proj_local.point.x, proj_local.point.y, proj_local.point.z);
                let d = p_local.distance(point_on_surface_in_local);
                p_local = point_on_surface_in_local;
                if let Some(Contact {
                    contact_in_local,
                    state: contact_state,
                }) = c
                {
                    let stiction_d = d * stiction_factor;
                    let stiction_d2 = stiction_d * stiction_d;
                    if p_local.distance_squared(*contact_in_local) > stiction_d2 {
                        let delta = p_local - *contact_in_local;
                        p_local -= delta * (stiction_d * delta.length_recip());
                        let pt = na::Point3::new(p_local.x, p_local.y, p_local.z);
                        let proj = collider.shape.project_local_point(&pt, false);
                        if proj.is_inside {
                            p_local = vec3(proj.point.x, proj.point.y, proj.point.z);
                        }
                        *contact_in_local = p_local;
                        *contact_state = ContactState::Sliding;
                    } else {
                        p_local = *contact_in_local;
                        *contact_state = ContactState::Sticking;
                    }
                } else {
                    *c = Some(Contact {
                        contact_in_local: p_local,
                        state: ContactState::New,
                    });
                }
                let pt = m.transform_point(&na::Point3::new(p_local.x, p_local.y, p_local.z));
                *p_global = vec3(pt.x, pt.y, pt.z);
            }
        }
    }
}
