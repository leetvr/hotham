use std::time::Instant;

use hotham::{anyhow, components::Collider, hecs::World};

use crate::{xpbd_state::XpbdState, InterpolatedTransform};

pub fn init_rerun_session() -> anyhow::Result<rerun::Session> {
    let mut session = rerun::SessionBuilder::new("XPBD").connect(rerun::default_server_addr());
    rerun::MsgSender::new("world")
        .with_timeless(true)
        .with_splat(rerun::components::ViewCoordinates::from_up_and_handedness(
            rerun::coordinates::SignedAxis3::POSITIVE_Y,
            rerun::coordinates::Handedness::Right,
        ))?
        .send(&mut session)?;
    Ok(session)
}

pub fn send_xpbd_state_to_rerun(
    xpbd_state: &XpbdState,
    session: &mut rerun::Session,
    simulation_time: Instant,
    simulation_time_epoch: Instant,
) -> anyhow::Result<()> {
    let simulation_timeline =
        rerun::time::Timeline::new("simulation_time", rerun::time::TimeType::Time);
    let time_since_epoch = simulation_time - simulation_time_epoch;
    let radius = rerun::components::Radius(0.025);
    let color = rerun::components::ColorRGBA::from_rgb(64, 64, 64);
    let points = xpbd_state
        .points_curr
        .iter()
        .map(|&p| rerun::components::Point3D::new(p.x as _, p.y as _, p.z as _))
        .collect::<Vec<rerun::components::Point3D>>();
    rerun::MsgSender::new("world/points")
        .with_time(
            simulation_timeline,
            rerun::time::Time::from_seconds_since_epoch(time_since_epoch.as_secs_f64()),
        )
        .with_component(&points)?
        .with_splat(color)?
        .with_splat(radius)?
        .send(session)?;
    Ok(())
}

pub fn send_colliders_to_rerun(
    world: &World,
    session: &mut rerun::Session,
    simulation_time: Instant,
    simulation_time_epoch: Instant,
) -> anyhow::Result<()> {
    let simulation_timeline =
        rerun::time::Timeline::new("simulation_time", rerun::time::TimeType::Time);
    let time_since_epoch = simulation_time - simulation_time_epoch;
    let mut radii = Vec::new();
    let color = rerun::components::ColorRGBA::from_rgb(128, 128, 64);
    let mut points = Vec::new();
    for (_, (collider, transform)) in world.query::<(&Collider, &InterpolatedTransform)>().iter() {
        let p = transform.0.transform_point3(collider.offset_from_parent);
        if let Some(ball) = collider.shape.as_ball() {
            points.push(rerun::components::Point3D::new(
                p.x as _, p.y as _, p.z as _,
            ));
            radii.push(rerun::components::Radius(ball.radius));
        }
    }
    if points.is_empty() {
        return Ok(());
    }
    assert_eq!(radii.len(), points.len());
    rerun::MsgSender::new("world/colliders")
        .with_time(
            simulation_timeline,
            rerun::time::Time::from_seconds_since_epoch(time_since_epoch.as_secs_f64()),
        )
        .with_component(&points)?
        .with_splat(color)?
        .with_component(&radii)?
        .send(session)?;
    Ok(())
}
