use hotham::{
    components::{Collider, GlobalTransform},
    glam::{dvec3, DVec3},
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
    pub contact_in_local: DVec3,
    pub state: ContactState,
}

pub struct XpbdCollisions {
    pub active_collisions: Vec<Option<Contact>>,
}

pub fn resolve_ecs_collisions(world: &mut World, points_next: &mut [DVec3], stiction_factor: f64) {
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
            let pt_local = m.inverse_transform_point(&na::Point3::new(
                p_global.x as f32,
                p_global.y as f32,
                p_global.z as f32,
            ));
            let proj_local = collider.shape.project_local_point(&pt_local, false);
            if proj_local.is_inside {
                let mut p_local = dvec3(pt_local.x as _, pt_local.y as _, pt_local.z as _);
                let point_on_surface_in_local = dvec3(
                    proj_local.point.x as f64,
                    proj_local.point.y as f64,
                    proj_local.point.z as f64,
                );
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
                        let pt =
                            na::Point3::new(p_local.x as f32, p_local.y as f32, p_local.z as f32);
                        let proj = collider.shape.project_local_point(&pt, false);
                        if proj.is_inside {
                            p_local =
                                dvec3(proj.point.x as _, proj.point.y as _, proj.point.z as _);
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
                let pt = m.transform_point(&na::Point3::new(
                    p_local.x as f32,
                    p_local.y as f32,
                    p_local.z as f32,
                ));
                *p_global = dvec3(pt.x as _, pt.y as _, pt.z as _);
            }
        }
    }
}
