use std::ops::{Deref, DerefMut};

use super::GetForm;
use crate::{
    project::{Project, model::ProjectRequest},
    widgets::{
        form::{FormState, InputField},
        text_input::TextInputState,
    },
};

/// The "add" / "edit project" form.
///
/// Wraps a [`FormState`] with the four fields every project entry has
/// (name, script, parent, icon). The field order is positional, so the
/// `name`/`script`/`parent`/`icon` accessors below index into
/// [`FormState::input_fields`].
#[derive(Debug)]
pub struct ProjectForm {
    pub(super) state: FormState,
}

impl GetForm for ProjectForm {
    fn state(&self) -> &FormState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut FormState {
        &mut self.state
    }
}

impl Default for ProjectForm {
    fn default() -> Self {
        let state = FormState::new(vec![
            InputField::new("Name"),
            InputField::new("Script"),
            InputField::new("Parent"),
            InputField::new("Icon"),
        ]);

        Self { state }
    }
}

impl ProjectForm {
    /// The text currently typed into the *Name* field.
    pub fn name(&self) -> &str {
        self.state.input_fields[0].text()
    }

    /// The text currently typed into the *Script* field (the script file name).
    pub fn script(&self) -> &str {
        self.state.input_fields[1].text()
    }

    /// The text currently typed into the *Parent* field (may be empty).
    pub fn parent(&self) -> &str {
        self.state.input_fields[2].text()
    }

    /// The text currently typed into the *Icon* field (may be empty).
    pub fn icon(&self) -> &str {
        self.state.input_fields[3].text()
    }

    /// Set the *Name* field.
    pub fn set_name(&mut self, name: &str) {
        self.state.input_fields[0].set_text(name);
    }

    /// Set the *Script* field.
    pub fn set_script(&mut self, script: &str) {
        self.state.input_fields[1].set_text(script);
    }

    /// Set the *Parent* field, or clear it when `parent` is `None`.
    pub fn set_parent(&mut self, parent: Option<&str>) {
        let text = parent.unwrap_or("");
        self.state.input_fields[2].set_text(text);
    }

    /// Set the *Icon* field, or clear it when `icon` is `None`.
    pub fn set_icon(&mut self, icon: Option<&str>) {
        let text = icon.unwrap_or("");
        self.state.input_fields[3].set_text(text);
    }

    /// Populate every field from `project`. Typically used when opening the
    /// form in *Edit* mode.
    pub fn set_project(&mut self, project: Project) {
        self.set_name(&project.name);
        self.set_script(&project.script_name);
        self.set_parent(project.parent.as_deref());
        self.set_icon(project.icon.as_deref());
    }

    /// Build a [`ProjectRequest`] from the current field values. Empty
    /// `parent`/`icon` strings are treated as absent.
    pub fn get_project(&self) -> ProjectRequest<'_> {
        ProjectRequest {
            name: self.name(),
            script: self.script(),
            parent: self.parent(),
            icon: self.icon(),
        }
    }
}

// `ProjectForm` derefs to a [`TextInputState`] so that keyboard input can be
// forwarded straight into whichever input field is currently focused. (This
// is a two-step coercion: `self.state` is a `FormState`, which itself derefs to
// the selected field's text input.)
impl Deref for ProjectForm {
    type Target = TextInputState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for ProjectForm {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
