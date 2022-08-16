use anyhow::Result;
use openxr::{self as xr, Action, ActionSet, Haptic, Path, Posef, Space};

pub struct Input {
    pub action_set: ActionSet,
    pub grip_pose_action: Action<Posef>,
    pub aim_pose_action: Action<Posef>,
    pub squeeze_action: Action<f32>,
    pub trigger_action: Action<f32>,
    pub trigger_touch_action: Action<bool>,
    pub haptic_feedback_action: Action<Haptic>,
    pub x_button_action: Action<bool>,
    pub x_touch_action: Action<bool>,
    pub y_button_action: Action<bool>,
    pub y_touch_action: Action<bool>,
    pub menu_button_action: Action<bool>,
    pub a_button_action: Action<bool>,
    pub a_touch_action: Action<bool>,
    pub b_button_action: Action<bool>,
    pub b_touch_action: Action<bool>,
    pub thumbstick_x_action: Action<f32>,
    pub thumbstick_y_action: Action<f32>,
    pub thumbstick_click_action: Action<bool>,
    pub thumbstick_touch_action: Action<bool>,
    pub thumbrest_touch_action: Action<bool>,
    pub left_hand_grip_space: Space,
    pub left_hand_aim_space: Space,
    pub left_hand_subaction_path: Path,
    pub right_hand_grip_space: Space,
    pub right_hand_aim_space: Space,
    pub right_hand_subaction_path: Path,
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
        let left_hand_grip_pose_path = instance
            .string_to_path("/user/hand/left/input/grip/pose")
            .unwrap();
        let left_hand_aim_pose_path = instance
            .string_to_path("/user/hand/left/input/aim/pose")
            .unwrap();
        let right_hand_grip_pose_path = instance
            .string_to_path("/user/hand/right/input/grip/pose")
            .unwrap();
        let right_hand_aim_pose_path = instance
            .string_to_path("/user/hand/right/input/aim/pose")
            .unwrap();

        let left_hand_squeeze_path = instance
            .string_to_path("/user/hand/left/input/squeeze/value")
            .unwrap();
        let left_hand_trigger_path = instance
            .string_to_path("/user/hand/left/input/trigger/value")
            .unwrap();
        let left_hand_trigger_touch_path = instance
            .string_to_path("/user/hand/left/input/trigger/touch")
            .unwrap();
        let left_hand_haptic_feedback_path = instance
            .string_to_path("/user/hand/left/output/haptic")
            .unwrap();

        let right_hand_squeeze_path = instance
            .string_to_path("/user/hand/right/input/squeeze/value")
            .unwrap();
        let right_hand_trigger_path = instance
            .string_to_path("/user/hand/right/input/trigger/value")
            .unwrap();
        let right_hand_trigger_touch_path = instance
            .string_to_path("/user/hand/right/input/trigger/touch")
            .unwrap();
        let right_hand_haptic_feedback_path = instance
            .string_to_path("/user/hand/right/output/haptic")
            .unwrap();

        let x_button_path = instance
            .string_to_path("/user/hand/left/input/x/click")
            .unwrap();
        let x_button_touch_path = instance
            .string_to_path("/user/hand/left/input/x/touch")
            .unwrap();
        let y_button_path = instance
            .string_to_path("/user/hand/left/input/y/click")
            .unwrap();
        let y_button_touch_path = instance
            .string_to_path("/user/hand/left/input/y/touch")
            .unwrap();
        let menu_button_path = instance
            .string_to_path("/user/hand/left/input/menu/click")
            .unwrap();

        let a_button_path = instance
            .string_to_path("/user/hand/right/input/a/click")
            .unwrap();
        let a_button_touch_path = instance
            .string_to_path("/user/hand/right/input/a/touch")
            .unwrap();
        let b_button_path = instance
            .string_to_path("/user/hand/right/input/b/click")
            .unwrap();
        let b_button_touch_path = instance
            .string_to_path("/user/hand/right/input/b/touch")
            .unwrap();

        let left_hand_thumbstick_x_path = instance
            .string_to_path("/user/hand/left/input/thumbstick/x")
            .unwrap();
        let left_hand_thumbstick_y_path = instance
            .string_to_path("/user/hand/left/input/thumbstick/y")
            .unwrap();
        let left_hand_thumbstick_click_path = instance
            .string_to_path("/user/hand/left/input/thumbstick/click")
            .unwrap();
        let left_hand_thumbstick_touch_path = instance
            .string_to_path("/user/hand/left/input/thumbstick/touch")
            .unwrap();
        let left_hand_thumbrest_touch_path = instance
            .string_to_path("/user/hand/left/input/thumbrest/touch")
            .unwrap();

        let right_hand_thumbstick_x_path = instance
            .string_to_path("/user/hand/right/input/thumbstick/x")
            .unwrap();
        let right_hand_thumbstick_y_path = instance
            .string_to_path("/user/hand/right/input/thumbstick/y")
            .unwrap();
        let right_hand_thumbstick_click_path = instance
            .string_to_path("/user/hand/right/input/thumbstick/click")
            .unwrap();
        let right_hand_thumbstick_touch_path = instance
            .string_to_path("/user/hand/right/input/thumbstick/touch")
            .unwrap();
        let right_hand_thumbrest_touch_path = instance
            .string_to_path("/user/hand/right/input/thumbrest/touch")
            .unwrap();

        let grip_pose_action = action_set.create_action::<xr::Posef>(
            "hand_pose",
            "Hand Pose",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let aim_pose_action = action_set.create_action::<xr::Posef>(
            "pointer_pose",
            "Pointer Pose",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let trigger_action = action_set.create_action::<f32>(
            "trigger",
            "Trigger Pull",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let trigger_touch_action = action_set.create_action::<bool>(
            "trigger_touched",
            "Trigger Touch",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let squeeze_action = action_set.create_action::<f32>(
            "squeeze",
            "Grip Pull",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let haptic_feedback_action = action_set.create_action::<Haptic>(
            "haptic_feedback",
            "Haptic Feedback",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let x_button_action = action_set.create_action::<bool>("x_button", "X Button", &[])?;
        let x_touch_action =
            action_set.create_action::<bool>("x_button_touch", "X Button Touch", &[])?;
        let y_button_action = action_set.create_action::<bool>("y_button", "Y Button", &[])?;
        let y_touch_action =
            action_set.create_action::<bool>("y_button_touch", "Y Button Touch", &[])?;
        let menu_button_action =
            action_set.create_action::<bool>("menu_button", "Menu Button", &[])?;

        let a_button_action = action_set.create_action::<bool>("a_button", "A Button", &[])?;
        let a_touch_action =
            action_set.create_action::<bool>("a_button_touch", "A Button Touch", &[])?;
        let b_button_action = action_set.create_action::<bool>("b_button", "B Button", &[])?;
        let b_touch_action =
            action_set.create_action::<bool>("b_button_touch", "B Button Touch", &[])?;

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
        let thumbstick_click_action = action_set.create_action::<bool>(
            "thumbstick_click",
            "Thumbstick Click",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;
        let thumbstick_touch_action = action_set.create_action::<bool>(
            "thumbstick_touch",
            "Thumbstick Touch",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;
        let thumbrest_touch_action = action_set.create_action::<bool>(
            "thumbrest_touch",
            "Thumbrest Touch",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        // Bind our actions to input devices using the given profile
        instance.suggest_interaction_profile_bindings(
            instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .unwrap(),
            &[
                xr::Binding::new(&grip_pose_action, left_hand_grip_pose_path),
                xr::Binding::new(&grip_pose_action, right_hand_grip_pose_path),
                xr::Binding::new(&aim_pose_action, left_hand_aim_pose_path),
                xr::Binding::new(&aim_pose_action, right_hand_aim_pose_path),
                xr::Binding::new(&squeeze_action, left_hand_squeeze_path),
                xr::Binding::new(&squeeze_action, right_hand_squeeze_path),
                xr::Binding::new(&trigger_action, left_hand_trigger_path),
                xr::Binding::new(&trigger_action, right_hand_trigger_path),
                xr::Binding::new(&trigger_touch_action, left_hand_trigger_touch_path),
                xr::Binding::new(&trigger_touch_action, right_hand_trigger_touch_path),
                xr::Binding::new(&haptic_feedback_action, left_hand_haptic_feedback_path),
                xr::Binding::new(&haptic_feedback_action, right_hand_haptic_feedback_path),
                xr::Binding::new(&x_button_action, x_button_path),
                xr::Binding::new(&x_touch_action, x_button_touch_path),
                xr::Binding::new(&y_button_action, y_button_path),
                xr::Binding::new(&y_touch_action, y_button_touch_path),
                xr::Binding::new(&menu_button_action, menu_button_path),
                xr::Binding::new(&a_button_action, a_button_path),
                xr::Binding::new(&a_touch_action, a_button_touch_path),
                xr::Binding::new(&b_button_action, b_button_path),
                xr::Binding::new(&b_touch_action, b_button_touch_path),
                xr::Binding::new(&thumbstick_x_action, left_hand_thumbstick_x_path),
                xr::Binding::new(&thumbstick_x_action, right_hand_thumbstick_x_path),
                xr::Binding::new(&thumbstick_y_action, left_hand_thumbstick_y_path),
                xr::Binding::new(&thumbstick_y_action, right_hand_thumbstick_y_path),
                xr::Binding::new(&thumbstick_click_action, left_hand_thumbstick_click_path),
                xr::Binding::new(&thumbstick_click_action, right_hand_thumbstick_click_path),
                xr::Binding::new(&thumbstick_touch_action, left_hand_thumbstick_touch_path),
                xr::Binding::new(&thumbstick_touch_action, right_hand_thumbstick_touch_path),
                xr::Binding::new(&thumbrest_touch_action, left_hand_thumbrest_touch_path),
                xr::Binding::new(&thumbrest_touch_action, right_hand_thumbrest_touch_path),
            ],
        )?;

        let left_hand_grip_space = grip_pose_action.create_space(
            session.clone(),
            left_hand_subaction_path,
            Posef::IDENTITY,
        )?;
        let left_hand_aim_space = aim_pose_action.create_space(
            session.clone(),
            left_hand_subaction_path,
            Posef::IDENTITY,
        )?;

        let right_hand_grip_space = grip_pose_action.create_space(
            session.clone(),
            right_hand_subaction_path,
            Posef::IDENTITY,
        )?;
        let right_hand_aim_space = aim_pose_action.create_space(
            session.clone(),
            right_hand_subaction_path,
            Posef::IDENTITY,
        )?;

        Ok(Input {
            action_set,
            grip_pose_action,
            aim_pose_action,
            squeeze_action,
            trigger_action,
            trigger_touch_action,
            haptic_feedback_action,
            x_button_action,
            x_touch_action,
            y_button_action,
            y_touch_action,
            menu_button_action,
            a_button_action,
            a_touch_action,
            b_button_action,
            b_touch_action,
            thumbstick_x_action,
            thumbstick_y_action,
            thumbstick_click_action,
            thumbstick_touch_action,
            thumbrest_touch_action,
            left_hand_grip_space,
            left_hand_aim_space,
            left_hand_subaction_path,
            right_hand_grip_space,
            right_hand_aim_space,
            right_hand_subaction_path,
        })
    }
}
