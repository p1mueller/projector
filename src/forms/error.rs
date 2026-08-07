use std::fmt::Display;

use super::GetForm;
use crate::widgets::form::FormState;

#[derive(Debug)]
pub struct ErrorForm {
    pub(super) state: FormState,
}

impl GetForm for ErrorForm {
    fn state(&self) -> &FormState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut FormState {
        &mut self.state
    }
}

impl Default for ErrorForm {
    fn default() -> Self {
        let state = FormState::new(Vec::new());

        Self { state }
    }
}

impl ErrorForm {
    pub fn clear(&mut self) {
        self.state.clear();
    }

    pub fn set_error(&mut self, error: impl Display) {
        self.state.set_error(error);
    }
}
