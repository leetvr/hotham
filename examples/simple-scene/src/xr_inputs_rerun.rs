use hotham::{anyhow, contexts::XrContext, util::is_space_valid, xr};

mod rr {
    pub use rerun::{
        components::{Box3D, ColorRGBA, Quaternion, Radius, Rigid3, Transform, Vec3D},
        time::{Time, TimeType, Timeline},
        MsgSender, Session,
    };
}

fn transform_from_pose(pose: xr::Posef) -> rr::Transform {
    rr::Transform::Rigid3(rr::Rigid3 {
        rotation: rr::Quaternion {
            x: pose.orientation.x,
            y: pose.orientation.y,
            z: pose.orientation.z,
            w: pose.orientation.w,
        },
        translation: rr::Vec3D([pose.position.x, pose.position.y, pose.position.z]),
    })
}

pub fn send_xr_inputs_state_to_rerun(
    xr_context: &XrContext,
    session: &rr::Session,
    time: xr::Time,
    name: &str,
) -> anyhow::Result<()> {
    let input = &xr_context.input;
    let left_hand_grip = &input
        .left_hand_grip_space
        .locate(&xr_context.stage_space, time)
        .unwrap();
    let left_hand_aim = &input
        .left_hand_aim_space
        .locate(&xr_context.stage_space, time)
        .unwrap();
    let right_hand_grip = &input
        .right_hand_grip_space
        .locate(&xr_context.stage_space, time)
        .unwrap();
    let right_hand_aim = &input
        .right_hand_aim_space
        .locate(&xr_context.stage_space, time)
        .unwrap();

    let xr_timeline = rr::Timeline::new("xr_time", rr::TimeType::Time);
    let rerun_time = rr::Time::from_seconds_since_epoch(time.as_nanos() as f64 / 1e9);
    let box3d = rr::Box3D::new(0.05, 0.05, 0.05);
    let radius = rr::Radius(0.005);

    if is_space_valid(left_hand_grip) {
        rr::MsgSender::new(format!("stage/left_grip_{name}"))
            .with_time(xr_timeline, rerun_time)
            .with_component(&[transform_from_pose(left_hand_grip.pose)])?
            .with_splat(box3d)?
            .with_splat(radius)?
            .with_component(&[rr::ColorRGBA::from_rgb(255, 0, 0)])?
            .send(session)?;
    }

    if is_space_valid(left_hand_aim) {
        rr::MsgSender::new(format!("stage/left_aim_{name}"))
            .with_time(xr_timeline, rerun_time)
            .with_component(&[transform_from_pose(left_hand_aim.pose)])?
            .with_splat(box3d)?
            .with_splat(radius)?
            .with_component(&[rr::ColorRGBA::from_rgb(255, 255, 0)])?
            .send(session)?;
    }

    if is_space_valid(right_hand_grip) {
        rr::MsgSender::new(format!("stage/right_grip_{name}"))
            .with_time(xr_timeline, rerun_time)
            .with_component(&[transform_from_pose(right_hand_grip.pose)])?
            .with_splat(box3d)?
            .with_splat(radius)?
            .with_component(&[rr::ColorRGBA::from_rgb(0, 0, 255)])?
            .send(session)?;
    }

    if is_space_valid(right_hand_aim) {
        rr::MsgSender::new(format!("stage/right_aim_{name}"))
            .with_time(xr_timeline, rerun_time)
            .with_component(&[transform_from_pose(right_hand_aim.pose)])?
            .with_splat(box3d)?
            .with_splat(radius)?
            .with_component(&[rr::ColorRGBA::from_rgb(0, 255, 255)])?
            .send(session)?;
    }

    Ok(())
}
