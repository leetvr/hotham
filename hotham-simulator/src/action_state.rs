use std::collections::HashMap;

use openxr_sys::{Action, Path, FALSE};

#[derive(Debug, Clone, Default)]
/// Stores Action state, allowing the simulator to simulate input for an application.
// A bit yuck to use u64 instead of Action, but it doesn't support Hash.. but whatever.
pub struct ActionState {
    boolean_actions: HashMap<u64, bool>,
    bindings: HashMap<Path, u64>,
}
impl ActionState {
    pub(crate) fn get_boolean(&self, action: Action) -> openxr_sys::Bool32 {
        self.boolean_actions
            .get(&action.into_raw())
            .map(|p| (*p).into())
            .unwrap_or(FALSE)
    }

    pub(crate) fn add_binding(&mut self, path: Path, action: Action) {
        self.bindings.insert(path, action.into_raw());
    }

    /// Resets all action state.
    pub(crate) fn clear(&mut self) {
        // Set all the booleans to false.
        self.boolean_actions.values_mut().for_each(|v| *v = false);
    }

    pub(crate) fn set_boolean(&mut self, path: &Path, value: bool) {
        let action = self.bindings.get(path).unwrap();
        self.boolean_actions.insert(*action, value);
    }
}
