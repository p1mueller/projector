pub mod project;

pub use project::ProjectForm;

use crate::widgets::form::FormState;

pub trait GetForm {
    fn state(&self) -> &FormState;
    fn state_mut(&mut self) -> &mut FormState;
}
