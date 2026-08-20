//! Typed form widgets for the TUI.
//!
//! Each form wraps a [`crate::widgets::form::FormState`] and gives it a
//! meaningful shape (for example, the fields of a project). Forms are rendered
//! by the generic [`crate::widgets::form::Form`] widget: it only needs access
//! to the underlying state, which is what the [`GetForm`] trait provides.

pub mod project;

pub use project::ProjectForm;

use crate::widgets::form::FormState;

/// Access to a form's underlying [`FormState`].
///
/// [`crate::widgets::form::Form`] is a generic, stateless widget that renders
/// whatever [`FormState`] it is given. Implementing this trait lets a typed
/// form (e.g. [`ProjectForm`]) be handed to that widget through
/// [`GetForm::state_mut`] without leaking the concrete type.
pub trait GetForm {
    /// An immutable view of the form's state.
    fn state(&self) -> &FormState;

    /// A mutable view of the form's state, used when rendering an input.
    fn state_mut(&mut self) -> &mut FormState;
}
