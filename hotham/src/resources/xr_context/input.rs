use anyhow::Result;
use openxr::{self as xr, Action, ActionSet, Haptic, Path, Posef, Space};

pub struct Input {
    pub action_set: ActionSet,
    pub pose_action: Action<Posef>,
    pub grab_action: Action<f32>,
    pub trigger_action: Action<f32>,
    pub haptic_feedback_action: Action<Haptic>,
    pub a_button_action: Action<bool>,
    pub b_button_action: Action<bool>,
    pub x_button_action: Action<bool>,
    pub y_button_action: Action<bool>,
    pub thumbstick_x_action: Action<f32>,
    pub thumbstick_y_action: Action<f32>,
    pub left_hand_space: Space,
    pub left_hand_subaction_path: Path,
    pub left_pointer_space: Space,
    pub right_hand_space: Space,
    pub right_hand_subaction_path: Path,
    pub right_pointer_space: Space,
}

impl Input {
    pub fn oculus_touch_controller(
        instance: &xr::Instance,
        session: &xr::Session<xr::Vulkan>,
    ) -> Result<Self> {
        // Create an action set to encapsulate our actions
        let action_set = instance.create_action_set("input", "input pose information", 0)?;

        let left_hand_subaction_path = instance.string_to_path("/user/hand/left").unwrap();
        let right_hand_subaction_path = instance.string_to_path("/user/hand/right").unwrap();
        let left_hand_pose_path = instance
            .string_to_path("/user/hand/left/input/grip/pose")
            .unwrap();
        let left_pointer_path = instance
            .string_to_path("/user/hand/left/input/aim/pose")
            .unwrap();
        let right_hand_pose_path = instance
            .string_to_path("/user/hand/right/input/grip/pose")
            .unwrap();
        let right_pointer_path = instance
            .string_to_path("/user/hand/right/input/aim/pose")
            .unwrap();

        let left_hand_grip_squeeze_path = instance
            .string_to_path("/user/hand/left/input/squeeze/value")
            .unwrap();
        let left_hand_grip_trigger_path = instance
            .string_to_path("/user/hand/left/input/trigger/value")
            .unwrap();
        let left_hand_haptic_feedback_path = instance
            .string_to_path("/user/hand/left/output/haptic")
            .unwrap();

        let right_hand_grip_squeeze_path = instance
            .string_to_path("/user/hand/right/input/squeeze/value")
            .unwrap();
        let right_hand_grip_trigger_path = instance
            .string_to_path("/user/hand/right/input/trigger/value")
            .unwrap();
        let right_hand_haptic_feedback_path = instance
            .string_to_path("/user/hand/right/output/haptic")
            .unwrap();

        let x_button_path = instance
            .string_to_path("/user/hand/left/input/x/click")
            .unwrap();
        let y_button_path = instance
            .string_to_path("/user/hand/left/input/y/click")
            .unwrap();

        let a_button_path = instance
            .string_to_path("/user/hand/right/input/a/click")
            .unwrap();
        let b_button_path = instance
            .string_to_path("/user/hand/right/input/b/click")
            .unwrap();

        let left_hand_thumbstick_x_path = instance
            .string_to_path("/user/hand/left/input/thumbstick/x")
            .unwrap();
        let left_hand_thumbstick_y_path = instance
            .string_to_path("/user/hand/left/input/thumbstick/y")
            .unwrap();

        let right_hand_thumbstick_x_path = instance
            .string_to_path("/user/hand/right/input/thumbstick/x")
            .unwrap();
        let right_hand_thumbstick_y_path = instance
            .string_to_path("/user/hand/right/input/thumbstick/y")
            .unwrap();

        let pose_action = action_set.create_action::<xr::Posef>(
            "hand_pose",
            "Hand Pose",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let aim_action = action_set.create_action::<xr::Posef>(
            "pointer_pose",
            "Pointer Pose",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let trigger_action = action_set.create_action::<f32>(
            "trigger_pulled",
            "Pull Trigger",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let grab_action = action_set.create_action::<f32>(
            "grab_object",
            "Grab Object",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let haptic_feedback_action = action_set.create_action::<Haptic>(
            "haptic_feedback",
            "Haptic Feedback",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let x_button_action = action_set.create_action::<bool>("x_button", "X Button", &[])?;
        let y_button_action = action_set.create_action::<bool>("y_button", "Y Button", &[])?;

        let a_button_action = action_set.create_action::<bool>("a_button", "A Button", &[])?;
        let b_button_action = action_set.create_action::<bool>("b_button", "B Button", &[])?;

        let thumbstick_x_action = action_set.create_action::<f32>(
            "thumbstick_x",
            "Thumbstick X",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;
        let thumbstick_y_action = action_set.create_action::<f32>(
            "thumbstick_y",
            "Thumbstick Y",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        // Bind our actions to input devices using the given profile
        instance.suggest_interaction_profile_bindings(
            instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .unwrap(),
            &[
                xr::Binding::new(&pose_action, left_hand_pose_path),
                xr::Binding::new(&pose_action, right_hand_pose_path),
                xr::Binding::new(&aim_action, left_pointer_path),
                xr::Binding::new(&aim_action, right_pointer_path),
                xr::Binding::new(&grab_action, left_hand_grip_squeeze_path),
                xr::Binding::new(&grab_action, right_hand_grip_squeeze_path),
                xr::Binding::new(&trigger_action, left_hand_grip_trigger_path),
                xr::Binding::new(&trigger_action, right_hand_grip_trigger_path),
                xr::Binding::new(&grab_action, right_hand_grip_squeeze_path),
                xr::Binding::new(&haptic_feedback_action, left_hand_haptic_feedback_path),
                xr::Binding::new(&haptic_feedback_action, right_hand_haptic_feedback_path),
                xr::Binding::new(&x_button_action, x_button_path),
                xr::Binding::new(&y_button_action, y_button_path),
                xr::Binding::new(&a_button_action, a_button_path),
                xr::Binding::new(&b_button_action, b_button_path),
                xr::Binding::new(&thumbstick_x_action, left_hand_thumbstick_x_path),
                xr::Binding::new(&thumbstick_x_action, right_hand_thumbstick_x_path),
                xr::Binding::new(&thumbstick_y_action, left_hand_thumbstick_y_path),
                xr::Binding::new(&thumbstick_y_action, right_hand_thumbstick_y_path),
            ],
        )?;

        let left_hand_space =
            pose_action.create_space(session.clone(), left_hand_subaction_path, Posef::IDENTITY)?;
        let left_pointer_space =
            aim_action.create_space(session.clone(), left_hand_subaction_path, Posef::IDENTITY)?;

        let right_hand_space = pose_action.create_space(
            session.clone(),
            right_hand_subaction_path,
            Posef::IDENTITY,
        )?;
        let right_pointer_space =
            aim_action.create_space(session.clone(), right_hand_subaction_path, Posef::IDENTITY)?;

        Ok(Input {
            action_set,
            pose_action,
            grab_action,
            trigger_action,
            haptic_feedback_action,
            a_button_action,
            b_button_action,
            x_button_action,
            y_button_action,
            thumbstick_x_action,
            thumbstick_y_action,
            left_hand_space,
            left_hand_subaction_path,
            left_pointer_space,
            right_hand_space,
            right_hand_subaction_path,
            right_pointer_space,
        })
    }
}
