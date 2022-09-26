use crate::{
    contexts::XrContext,
    util::{is_space_valid, posef_to_isometry},
    xr,
};
use nalgebra::{Isometry3, Vector2, Vector3};

#[derive(Debug, Default)]
pub struct LeftInputContext {
    // boolean input
    x_button: bool,
    x_button_prev: bool,
    y_button: bool,
    y_button_prev: bool,
    menu_button: bool,
    menu_button_prev: bool,
    grip_button: bool,
    grip_button_prev: bool,
    trigger_button: bool,
    trigger_button_prev: bool,
    thumbstick_click: bool,
    thumbstick_click_prev: bool,
    // touch boolean input
    x_touch: bool,
    x_touch_prev: bool,
    y_touch: bool,
    y_touch_prev: bool,
    trigger_touch: bool,
    trigger_touch_prev: bool,
    thumbstick_touch: bool,
    thumbstick_touch_prev: bool,
    thumbrest_touch: bool,
    thumbrest_touch_prev: bool,
    // float input
    grip_analog: f32,
    grip_analog_prev: f32,
    trigger_analog: f32,
    trigger_analog_prev: f32,
    // vec2 input
    thumbstick_xy: Vector2<f32>,
    // vec3 input
    linear_velocity: Vector3<f32>,
    angular_velocity: Vector3<f32>,
    // pose input
    stage_from_grip: Isometry3<f32>,
    stage_from_aim: Isometry3<f32>,
}

impl LeftInputContext {
    pub fn x_button(&self) -> bool {
        self.x_button
    }
    pub fn x_button_just_pressed(&self) -> bool {
        self.x_button && !self.x_button_prev
    }
    pub fn x_button_just_released(&self) -> bool {
        !self.x_button && self.x_button_prev
    }
    pub fn y_button(&self) -> bool {
        self.y_button
    }
    pub fn y_button_just_pressed(&self) -> bool {
        self.y_button && !self.y_button_prev
    }
    pub fn y_button_just_released(&self) -> bool {
        !self.y_button && self.y_button_prev
    }
    pub fn menu_button(&self) -> bool {
        self.menu_button
    }
    pub fn menu_button_just_pressed(&self) -> bool {
        self.menu_button && !self.menu_button_prev
    }
    pub fn menu_button_just_released(&self) -> bool {
        !self.menu_button && self.menu_button_prev
    }
    pub fn grip_button(&self) -> bool {
        self.grip_button
    }
    pub fn grip_button_just_pressed(&self) -> bool {
        self.grip_button && !self.grip_button_prev
    }
    pub fn grip_button_just_released(&self) -> bool {
        !self.grip_button && self.grip_button_prev
    }
    pub fn trigger_button(&self) -> bool {
        self.trigger_button
    }
    pub fn trigger_button_just_pressed(&self) -> bool {
        self.trigger_button && !self.trigger_button_prev
    }
    pub fn trigger_button_just_released(&self) -> bool {
        !self.trigger_button && self.trigger_button_prev
    }
    pub fn thumbstick_click(&self) -> bool {
        self.thumbstick_click
    }
    pub fn thumbstick_click_just_pressed(&self) -> bool {
        self.thumbstick_click && !self.thumbstick_click_prev
    }
    pub fn thumbstick_click_just_released(&self) -> bool {
        !self.thumbstick_click && self.thumbstick_click_prev
    }
    pub fn x_touch(&self) -> bool {
        self.x_touch
    }
    pub fn x_touch_just_pressed(&self) -> bool {
        self.x_touch && !self.x_touch_prev
    }
    pub fn x_touch_just_released(&self) -> bool {
        !self.x_touch && self.x_touch_prev
    }
    pub fn y_touch(&self) -> bool {
        self.y_touch
    }
    pub fn y_touch_just_pressed(&self) -> bool {
        self.y_touch && !self.y_touch_prev
    }
    pub fn y_touch_just_released(&self) -> bool {
        !self.y_touch && self.y_touch_prev
    }
    pub fn trigger_touch(&self) -> bool {
        self.trigger_touch
    }
    pub fn trigger_touch_just_pressed(&self) -> bool {
        self.trigger_touch && !self.trigger_touch_prev
    }
    pub fn trigger_touch_just_released(&self) -> bool {
        !self.trigger_touch && self.trigger_touch_prev
    }
    pub fn thumbstick_touch(&self) -> bool {
        self.thumbstick_touch
    }
    pub fn thumbstick_touch_just_pressed(&self) -> bool {
        self.thumbstick_touch && !self.thumbstick_touch_prev
    }
    pub fn thumbstick_touch_just_released(&self) -> bool {
        !self.thumbstick_touch && self.thumbstick_touch_prev
    }
    pub fn thumbrest_touch(&self) -> bool {
        self.thumbrest_touch
    }
    pub fn thumbrest_touch_just_pressed(&self) -> bool {
        self.thumbrest_touch && !self.thumbrest_touch_prev
    }
    pub fn thumbrest_touch_just_released(&self) -> bool {
        !self.thumbrest_touch && self.thumbrest_touch_prev
    }
    pub fn grip_analog(&self) -> f32 {
        self.grip_analog
    }
    pub fn trigger_analog(&self) -> f32 {
        self.trigger_analog
    }
    pub fn thumbstick_xy(&self) -> Vector2<f32> {
        self.thumbstick_xy
    }
    pub fn linear_velocity(&self) -> Vector3<f32> {
        self.linear_velocity
    }
    pub fn angular_velocity(&self) -> Vector3<f32> {
        self.angular_velocity
    }
    pub fn stage_from_grip(&self) -> Isometry3<f32> {
        self.stage_from_grip
    }
    pub fn stage_from_aim(&self) -> Isometry3<f32> {
        self.stage_from_aim
    }
}

#[derive(Debug, Default)]
pub struct RightInputContext {
    // boolean input
    a_button: bool,
    a_button_prev: bool,
    b_button: bool,
    b_button_prev: bool,
    grip_button: bool,
    grip_button_prev: bool,
    trigger_button: bool,
    trigger_button_prev: bool,
    thumbstick_click: bool,
    thumbstick_click_prev: bool,
    // touch boolean input
    a_touch: bool,
    a_touch_prev: bool,
    b_touch: bool,
    b_touch_prev: bool,
    trigger_touch: bool,
    trigger_touch_prev: bool,
    thumbstick_touch: bool,
    thumbstick_touch_prev: bool,
    thumbrest_touch: bool,
    thumbrest_touch_prev: bool,
    // float input
    grip_analog: f32,
    grip_analog_prev: f32,
    trigger_analog: f32,
    trigger_analog_prev: f32,
    // vec2 input
    thumbstick_xy: Vector2<f32>,
    // vec3 input
    linear_velocity: Vector3<f32>,
    angular_velocity: Vector3<f32>,
    // pose input
    stage_from_grip: Isometry3<f32>,
    stage_from_aim: Isometry3<f32>,
}

impl RightInputContext {
    pub fn a_button(&self) -> bool {
        self.a_button
    }
    pub fn a_button_just_pressed(&self) -> bool {
        self.a_button && !self.a_button_prev
    }
    pub fn a_button_just_released(&self) -> bool {
        !self.a_button && self.a_button_prev
    }
    pub fn b_button(&self) -> bool {
        self.b_button
    }
    pub fn b_button_just_pressed(&self) -> bool {
        self.b_button && !self.b_button_prev
    }
    pub fn b_button_just_released(&self) -> bool {
        !self.b_button && self.b_button_prev
    }
    pub fn grip_button(&self) -> bool {
        self.grip_button
    }
    pub fn grip_button_just_pressed(&self) -> bool {
        self.grip_button && !self.grip_button_prev
    }
    pub fn grip_button_just_released(&self) -> bool {
        !self.grip_button && self.grip_button_prev
    }
    pub fn trigger_button(&self) -> bool {
        self.trigger_button
    }
    pub fn trigger_button_just_pressed(&self) -> bool {
        self.trigger_button && !self.trigger_button_prev
    }
    pub fn trigger_button_just_released(&self) -> bool {
        !self.trigger_button && self.trigger_button_prev
    }
    pub fn thumbstick_click(&self) -> bool {
        self.thumbstick_click
    }
    pub fn thumbstick_click_just_pressed(&self) -> bool {
        self.thumbstick_click && !self.thumbstick_click_prev
    }
    pub fn thumbstick_click_just_released(&self) -> bool {
        !self.thumbstick_click && self.thumbstick_click_prev
    }
    pub fn a_touch(&self) -> bool {
        self.a_touch
    }
    pub fn a_touch_just_pressed(&self) -> bool {
        self.a_touch && !self.a_touch_prev
    }
    pub fn a_touch_just_released(&self) -> bool {
        !self.a_touch && self.a_touch_prev
    }
    pub fn b_touch(&self) -> bool {
        self.b_touch
    }
    pub fn b_touch_just_pressed(&self) -> bool {
        self.b_touch && !self.b_touch_prev
    }
    pub fn b_touch_just_released(&self) -> bool {
        !self.b_touch && self.b_touch_prev
    }
    pub fn trigger_touch(&self) -> bool {
        self.trigger_touch
    }
    pub fn trigger_touch_just_pressed(&self) -> bool {
        self.trigger_touch && !self.trigger_touch_prev
    }
    pub fn trigger_touch_just_released(&self) -> bool {
        !self.trigger_touch && self.trigger_touch_prev
    }
    pub fn thumbstick_touch(&self) -> bool {
        self.thumbstick_touch
    }
    pub fn thumbstick_touch_just_pressed(&self) -> bool {
        self.thumbstick_touch && !self.thumbstick_touch_prev
    }
    pub fn thumbstick_touch_just_released(&self) -> bool {
        !self.thumbstick_touch && self.thumbstick_touch_prev
    }
    pub fn thumbrest_touch(&self) -> bool {
        self.thumbrest_touch
    }
    pub fn thumbrest_touch_just_pressed(&self) -> bool {
        self.thumbrest_touch && !self.thumbrest_touch_prev
    }
    pub fn thumbrest_touch_just_released(&self) -> bool {
        !self.thumbrest_touch && self.thumbrest_touch_prev
    }
    pub fn grip_analog(&self) -> f32 {
        self.grip_analog
    }
    pub fn trigger_analog(&self) -> f32 {
        self.trigger_analog
    }
    pub fn thumbstick_xy(&self) -> Vector2<f32> {
        self.thumbstick_xy
    }
    pub fn linear_velocity(&self) -> Vector3<f32> {
        self.linear_velocity
    }
    pub fn angular_velocity(&self) -> Vector3<f32> {
        self.angular_velocity
    }
    pub fn stage_from_grip(&self) -> Isometry3<f32> {
        self.stage_from_grip
    }
    pub fn stage_from_aim(&self) -> Isometry3<f32> {
        self.stage_from_aim
    }
}

#[derive(Debug, Default)]
/// Context that holds input state. Allows users to query for input events without having to
/// worry about OpenXR internals.
///
/// Currently only supports buttons on the Oculus Touch controllers, but will be expanded to
/// support triggers and more.
pub struct InputContext {
    pub left: LeftInputContext,
    pub right: RightInputContext,
}

impl InputContext {
    /// Synchronize the context state with OpenXR. Automatically called by `Engine`
    /// each tick.
    pub fn update(&mut self, xr_context: &XrContext) {
        let input = &xr_context.input;
        let session = &xr_context.session;
        let left_subaction_path = input.left_hand_subaction_path;
        let right_subaction_path = input.right_hand_subaction_path;
        let time = xr_context.frame_state.predicted_display_time;

        self.left.x_button_prev = self.left.x_button;
        self.left.y_button_prev = self.left.y_button;
        self.left.menu_button_prev = self.left.menu_button;
        self.left.grip_button_prev = self.left.grip_button;
        self.left.trigger_button_prev = self.left.trigger_button;
        self.left.thumbstick_click_prev = self.left.thumbstick_click;
        self.left.x_touch_prev = self.left.x_touch;
        self.left.y_touch_prev = self.left.y_touch;
        self.left.trigger_touch_prev = self.left.trigger_touch;
        self.left.thumbstick_touch_prev = self.left.thumbstick_touch;
        self.left.thumbrest_touch_prev = self.left.thumbrest_touch;
        self.left.grip_analog_prev = self.left.grip_analog;
        self.left.trigger_analog_prev = self.left.trigger_analog;

        self.right.a_button_prev = self.right.a_button;
        self.right.b_button_prev = self.right.b_button;
        self.right.grip_button_prev = self.right.grip_button;
        self.right.trigger_button_prev = self.right.trigger_button;
        self.right.thumbstick_click_prev = self.right.thumbstick_click;
        self.right.a_touch_prev = self.right.a_touch;
        self.right.b_touch_prev = self.right.b_touch;
        self.right.trigger_touch_prev = self.right.trigger_touch;
        self.right.thumbstick_touch_prev = self.right.thumbstick_touch;
        self.right.thumbrest_touch_prev = self.right.thumbrest_touch;
        self.right.grip_analog_prev = self.right.grip_analog;
        self.right.trigger_analog_prev = self.right.trigger_analog;

        self.left.x_button =
            xr::ActionInput::get(&input.x_button_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.y_button =
            xr::ActionInput::get(&input.y_button_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.menu_button =
            xr::ActionInput::get(&input.menu_button_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.thumbstick_click =
            xr::ActionInput::get(&input.thumbstick_click_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.x_touch =
            xr::ActionInput::get(&input.x_touch_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.y_touch =
            xr::ActionInput::get(&input.y_touch_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.trigger_touch =
            xr::ActionInput::get(&input.trigger_touch_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.thumbstick_touch =
            xr::ActionInput::get(&input.thumbstick_touch_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.thumbrest_touch =
            xr::ActionInput::get(&input.thumbrest_touch_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.grip_analog =
            xr::ActionInput::get(&input.squeeze_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.grip_button = self.left.grip_analog > 0.1;
        self.left.trigger_analog =
            xr::ActionInput::get(&input.trigger_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.trigger_button = self.left.trigger_analog > 0.1;
        self.left.thumbstick_xy.x =
            xr::ActionInput::get(&input.thumbstick_x_action, session, left_subaction_path)
                .unwrap()
                .current_state;
        self.left.thumbstick_xy.y =
            xr::ActionInput::get(&input.thumbstick_y_action, session, left_subaction_path)
                .unwrap()
                .current_state;

        let (location, velocity) = &input
            .left_hand_grip_space
            .relate(&xr_context.stage_space, time)
            .unwrap();
        if is_space_valid(location) {
            self.left.stage_from_grip = posef_to_isometry(location.pose);
            self.left.linear_velocity = mint::Vector3::from(velocity.linear_velocity).into();
            self.left.angular_velocity = mint::Vector3::from(velocity.angular_velocity).into();
        }
        let location = &input
            .left_hand_aim_space
            .locate(&xr_context.stage_space, time)
            .unwrap();
        if is_space_valid(location) {
            self.left.stage_from_aim = posef_to_isometry(location.pose);
        }

        self.right.a_button =
            xr::ActionInput::get(&input.a_button_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.b_button =
            xr::ActionInput::get(&input.b_button_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.thumbstick_click = xr::ActionInput::get(
            &input.thumbstick_click_action,
            session,
            right_subaction_path,
        )
        .unwrap()
        .current_state;
        self.right.a_touch =
            xr::ActionInput::get(&input.a_touch_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.b_touch =
            xr::ActionInput::get(&input.b_touch_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.trigger_touch =
            xr::ActionInput::get(&input.trigger_touch_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.thumbstick_touch = xr::ActionInput::get(
            &input.thumbstick_touch_action,
            session,
            right_subaction_path,
        )
        .unwrap()
        .current_state;
        self.right.thumbrest_touch =
            xr::ActionInput::get(&input.thumbrest_touch_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.grip_analog =
            xr::ActionInput::get(&input.squeeze_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.grip_button = self.right.grip_analog > 0.1;
        self.right.trigger_analog =
            xr::ActionInput::get(&input.trigger_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.trigger_button = self.right.trigger_analog > 0.1;
        self.right.thumbstick_xy.x =
            xr::ActionInput::get(&input.thumbstick_x_action, session, right_subaction_path)
                .unwrap()
                .current_state;
        self.right.thumbstick_xy.y =
            xr::ActionInput::get(&input.thumbstick_y_action, session, right_subaction_path)
                .unwrap()
                .current_state;

        let (location, velocity) = &input
            .right_hand_grip_space
            .relate(&xr_context.stage_space, time)
            .unwrap();
        if is_space_valid(location) {
            self.right.stage_from_grip = posef_to_isometry(location.pose);
            self.right.linear_velocity = mint::Vector3::from(velocity.linear_velocity).into();
            self.right.angular_velocity = mint::Vector3::from(velocity.angular_velocity).into();
        }
        let location = &input
            .right_hand_aim_space
            .locate(&xr_context.stage_space, time)
            .unwrap();
        if is_space_valid(location) {
            self.right.stage_from_aim = posef_to_isometry(location.pose);
        }
    }
}
