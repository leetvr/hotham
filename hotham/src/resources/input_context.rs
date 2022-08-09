use crate::{resources::XrContext, xr};

#[derive(Debug, Default)]
/// Context that holds input state. Allows users to query for input events without having to
/// worry about OpenXR internals.
///
/// Currently only supports buttons on the Oculus Touch controllers, but will be expanded to
/// support triggers and more.
pub struct InputContext {
    a_button: bool,
    a_button_prev: bool,
    b_button: bool,
    b_button_prev: bool,
    x_button: bool,
    x_button_prev: bool,
    y_button: bool,
    y_button_prev: bool,
}

impl InputContext {
    /// Get the current state of the A button
    pub fn a_button(&self) -> bool {
        self.a_button
    }

    /// Was the A button just pressed this frame?
    pub fn a_button_just_pressed(&self) -> bool {
        !self.a_button_prev & self.a_button
    }

    /// Was the A button just pressed this frame?
    pub fn a_button_just_released(&self) -> bool {
        self.a_button_prev & !self.a_button
    }

    /// Get the current state of the B button
    pub fn b_button(&self) -> bool {
        self.b_button
    }

    /// Was the B button just pressed this frame?
    pub fn b_button_just_pressed(&self) -> bool {
        !self.b_button_prev & self.b_button
    }

    /// Was the B button just released this frame?
    pub fn b_button_released(&self) -> bool {
        self.b_button_prev & !self.b_button
    }

    /// Get the current state of the X button
    pub fn x_button(&self) -> bool {
        self.x_button
    }

    /// Was the X button just pressed this frame?
    pub fn x_button_just_pressed(&self) -> bool {
        !self.x_button_prev & self.x_button
    }

    /// Was the X button just released this frame?
    pub fn x_button_just_released(&self) -> bool {
        self.x_button_prev & !self.x_button
    }

    /// Get the current state of the Y button
    pub fn y_button(&self) -> bool {
        self.y_button
    }

    /// Was the Y button just pressed this frame?
    pub fn y_button_just_pressed(&self) -> bool {
        !self.y_button_prev & self.y_button
    }

    /// Was the Y button just released this frame?
    pub fn y_button_just_released(&self) -> bool {
        self.y_button_prev & !self.y_button
    }

    /// Synchronize the context state with OpenXR. Automatically called by `Engine`
    /// each tick.
    pub(crate) fn update(&mut self, xr_context: &XrContext) {
        self.a_button_prev = self.a_button;
        self.b_button_prev = self.b_button;
        self.x_button_prev = self.x_button;
        self.y_button_prev = self.y_button;

        let input = &xr_context.input;
        self.a_button = xr::ActionInput::get(
            &input.a_button_action,
            &xr_context.session,
            input.right_hand_subaction_path,
        )
        .unwrap()
        .current_state;
        self.b_button = xr::ActionInput::get(
            &input.b_button_action,
            &xr_context.session,
            input.right_hand_subaction_path,
        )
        .unwrap()
        .current_state;
        self.x_button = xr::ActionInput::get(
            &input.x_button_action,
            &xr_context.session,
            input.left_hand_subaction_path,
        )
        .unwrap()
        .current_state;
        self.y_button = xr::ActionInput::get(
            &input.y_button_action,
            &xr_context.session,
            input.left_hand_subaction_path,
        )
        .unwrap()
        .current_state;
    }
}
