use lazy_static::*;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::{atomic::AtomicIsize, Mutex};

use hotham::glam::{vec2, Vec2};

use super::{
    load_snapshot, load_snapshot_subset, send_poses_to_rerun, solve_ik, IkNodeID, IkState,
};

struct PuffinServerManager {
    server: Option<puffin_http::Server>,
}

impl PuffinServerManager {
    fn new() -> Self {
        eprintln!("Starting puffin server");
        puffin::set_scopes_on(true); // Tell puffin to collect data
        puffin::GlobalProfiler::lock().new_frame();

        Self {
            server: match puffin_http::Server::new("0.0.0.0:8585") {
                Ok(server) => {
                    eprintln!(
                        "Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585"
                    );
                    Some(server)
                }
                Err(err) => {
                    eprintln!("Failed to start puffin server: {}", err);
                    None
                }
            },
        }
    }
}

impl Drop for PuffinServerManager {
    fn drop(&mut self) {
        eprintln!("Stopping puffin server");
        if let Some(ref server) = self.server {
            // Wait for up to 2 seconds for the puffin viewer to connect
            for _ in 0..20 {
                if server.num_clients() != 0 {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        puffin::GlobalProfiler::lock().new_frame();
    }
}

struct PuffinServerUser {}

impl PuffinServerUser {
    fn new() -> Self {
        if PUFFIN_USERS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            let mut puffin_server = PUFFIN_SERVER.lock().unwrap();
            if puffin_server.is_none() {
                *puffin_server = Some(PuffinServerManager::new());
            }
        }
        Self {}
    }
}

impl Drop for PuffinServerUser {
    fn drop(&mut self) {
        if PUFFIN_USERS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1 {
            *PUFFIN_SERVER.lock().unwrap() = None;
        }
    }
}

lazy_static! {
    static ref PUFFIN_SERVER: Mutex<Option<PuffinServerManager>> = Default::default();
    static ref PUFFIN_USERS: AtomicIsize = Default::default();
}

#[must_use]
fn start_puffin_server() -> PuffinServerUser {
    PuffinServerUser::new()
}

trait Slerp {
    fn slerp(self, other: Self, t: f32) -> Self;
}

impl Slerp for Vec2 {
    fn slerp(self, b: Vec2, t: f32) -> Vec2 {
        let dot = self.dot(b);
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let inv_sin_theta = 1.0 / sin_theta;
        let a_factor = (theta * (1.0 - t)).sin() * inv_sin_theta;
        let b_factor = (theta * t).sin() * inv_sin_theta;
        self * a_factor + b * b_factor
    }
}

fn test_ik_solver(
    data: &str,
    thumbsticks: Option<(Vec2, Vec2)>,
) -> Result<(), hotham::anyhow::Error> {
    puffin::profile_function!();
    let session = rerun::SessionBuilder::new("XPBD").connect(rerun::default_server_addr());
    rerun::MsgSender::new("stage")
        .with_timeless(true)
        .with_splat(rerun::components::ViewCoordinates::from_up_and_handedness(
            rerun::coordinates::SignedAxis3::POSITIVE_Y,
            rerun::coordinates::Handedness::Right,
        ))?
        .send(&session)?;
    session.sink().drop_msgs_if_disconnected();

    let mut state = IkState::default();
    load_snapshot(&mut state, data);
    let (left_thumbstick, right_thumbstick) = thumbsticks.unwrap_or((Vec2::ZERO, Vec2::ZERO));
    for _ in 0..100 {
        let (shoulder_width, hip_width, sternum_height_in_torso, hip_height_in_pelvis) = solve_ik(
            state.get_affine(IkNodeID::Hmd),
            state.get_affine(IkNodeID::LeftGrip),
            state.get_affine(IkNodeID::LeftAim),
            state.get_affine(IkNodeID::RightGrip),
            state.get_affine(IkNodeID::RightAim),
            left_thumbstick,
            right_thumbstick,
            &mut state,
        );

        send_poses_to_rerun(
            &session,
            &state,
            shoulder_width,
            sternum_height_in_torso,
            hip_width,
            hip_height_in_pelvis,
        );
    }
    Ok(())
}

fn test_ik_solver_transition(
    data1: &str,
    data2: &str,
    thumbsticks1: Option<(Vec2, Vec2)>,
    thumbsticks2: Option<(Vec2, Vec2)>,
) -> Result<(), hotham::anyhow::Error> {
    puffin::profile_function!();
    let session = rerun::SessionBuilder::new("XPBD").connect(rerun::default_server_addr());
    rerun::MsgSender::new("stage")
        .with_timeless(true)
        .with_splat(rerun::components::ViewCoordinates::from_up_and_handedness(
            rerun::coordinates::SignedAxis3::POSITIVE_Y,
            rerun::coordinates::Handedness::Right,
        ))?
        .send(&session)?;
    session.sink().drop_msgs_if_disconnected();

    let mut state = IkState::default();
    load_snapshot(&mut state, data1);

    let (left_thumbstick1, right_thumbstick1) = thumbsticks1.unwrap_or((Vec2::ZERO, Vec2::ZERO));

    for _ in 0..100 {
        let (shoulder_width, hip_width, sternum_height_in_torso, hip_height_in_pelvis) = solve_ik(
            state.get_affine(IkNodeID::Hmd),
            state.get_affine(IkNodeID::LeftGrip),
            state.get_affine(IkNodeID::LeftAim),
            state.get_affine(IkNodeID::RightGrip),
            state.get_affine(IkNodeID::RightAim),
            left_thumbstick1,
            right_thumbstick1,
            &mut state,
        );

        send_poses_to_rerun(
            &session,
            &state,
            shoulder_width,
            sternum_height_in_torso,
            hip_width,
            hip_height_in_pelvis,
        );
    }

    load_snapshot_subset(
        &mut state,
        data2,
        &[
            IkNodeID::Hmd,
            IkNodeID::LeftGrip,
            IkNodeID::LeftAim,
            IkNodeID::RightGrip,
            IkNodeID::RightAim,
        ],
    );

    let (left_thumbstick2, right_thumbstick2) = thumbsticks2.unwrap_or((Vec2::ZERO, Vec2::ZERO));

    for i in 0..100 {
        let t = (i as f32 / 50.0).min(1.0);
        let (shoulder_width, hip_width, sternum_height_in_torso, hip_height_in_pelvis) = solve_ik(
            state.get_affine(IkNodeID::Hmd),
            state.get_affine(IkNodeID::LeftGrip),
            state.get_affine(IkNodeID::LeftAim),
            state.get_affine(IkNodeID::RightGrip),
            state.get_affine(IkNodeID::RightAim),
            left_thumbstick1.slerp(left_thumbstick2, t),
            right_thumbstick1.slerp(right_thumbstick2, t),
            &mut state,
        );

        send_poses_to_rerun(
            &session,
            &state,
            shoulder_width,
            sternum_height_in_torso,
            hip_width,
            hip_height_in_pelvis,
        );
    }

    Ok(())
}

#[test]
fn test_ik_solver_neutral() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-12_22.23.47.json"),
        None,
    )
}

#[test]
fn test_ik_solver_facing_x_dir() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-13_21.06.56.json"),
        None,
    )
}

#[test]
fn test_ik_solver_arms_up1() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-13_21.40.18.json"),
        None,
    )
}

#[test]
fn test_ik_solver_arms_up2() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-13_21.40.20.json"),
        None,
    )
}

#[test]
fn test_ik_solver_arms_up_transition() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-13_21.40.18.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-13_21.40.20.json"),
        None,
        None,
    )
}

#[test]
fn test_ik_solver_hands_bent_up() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-13_22.04.18.json"),
        None,
    )
}

#[test]
fn test_ik_solver_right_hand_punch() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_00.14.45.json"),
        Some((vec2(0.0, 0.0), vec2(0.0, 1.0))),
    )
}

#[test]
fn test_ik_solver_kick_transition1() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_21.56.59.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_21.57.01.json"),
        Some((vec2(0.0, 0.0), vec2(1.0, 0.0))),
        Some((vec2(0.0, 0.0), vec2(0.0, 0.0))),
    )
}

#[test]
fn test_ik_solver_kick_transition2() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        Some((vec2(0.0, 0.0), vec2(0.0, 0.0))),
        Some((vec2(-1.0, 0.0), vec2(0.0, 0.0))),
    )
}

#[test]
fn test_ik_solver_kick_transition3() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        Some((vec2(0.0, -1.0), vec2(0.0, 0.0))),
        Some((vec2(-1.0, 0.0), vec2(0.0, 0.0))),
    )
}

#[test]
fn test_ik_solver_kick_transition4() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-17_21.49.35.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-17_21.49.35.json"),
        Some((vec2(0.0, 0.0), vec2(0.0, -1.0))),
        Some((vec2(0.0, 0.0), vec2(1.0, 0.0))),
    )
}

#[test]
fn test_ik_solver_punch_kick_mix() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-17_21.49.35.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-17_21.49.35.json"),
        Some((vec2(0.0, 0.0), vec2(0.0, 0.0))),
        Some((vec2(0.0, 0.0), vec2(FRAC_1_SQRT_2, FRAC_1_SQRT_2))),
    )
}

#[test]
fn test_ik_solver_elbow_knee_transition1() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver_transition(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        Some((vec2(0.0, 0.0), vec2(0.0, 0.0))),
        Some((vec2(FRAC_1_SQRT_2, -FRAC_1_SQRT_2), vec2(0.0, 0.0))),
    )
}

#[test]
fn test_ik_solver_elbow_knee_soft1() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-16_22.13.45.json"),
        Some((
            vec2(FRAC_1_SQRT_2 * 0.5, -FRAC_1_SQRT_2 * 0.5),
            vec2(0.0, 0.0),
        )),
    )
}

#[test]
fn test_ik_solver_elbow1() -> hotham::anyhow::Result<()> {
    let _ = start_puffin_server();
    puffin::profile_function!();
    test_ik_solver(
        include_str!("../../../../inverse_kinematics_snapshot_2023-04-19_00.13.58.json"),
        Some((vec2(0.0, 0.0), vec2(-1.0, 0.0))),
    )
}
