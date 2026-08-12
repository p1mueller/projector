use std::ops::{Deref, DerefMut};

use super::GetForm;
use crate::{
    project::{Project, model::ProjectRequest},
    widgets::{
        form::{FormState, InputField},
        text_input::TextInputState,
    },
};

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
    pub fn name(&self) -> &str {
        self.state.input_fields[0].text()
    }

    pub fn script(&self) -> &str {
        self.state.input_fields[1].text()
    }

    pub fn parent(&self) -> &str {
        self.state.input_fields[2].text()
    }

    pub fn icon(&self) -> &str {
        self.state.input_fields[3].text()
    }

    pub fn set_name(&mut self, name: &str) {
        self.state.input_fields[0].set_text(name);
    }

    pub fn set_script(&mut self, script: &str) {
        self.state.input_fields[1].set_text(script);
    }

    pub fn set_parent(&mut self, parent: Option<&str>) {
        let text = parent.unwrap_or("");
        self.state.input_fields[2].set_text(text);
    }

    pub fn set_icon(&mut self, icon: Option<&str>) {
        let text = icon.unwrap_or("");
        self.state.input_fields[3].set_text(text);
    }

    pub fn set_project(&mut self, project: Project) {
        self.set_name(&project.name);
        self.set_script(&project.script_name);
        self.set_parent(project.parent.as_deref());
        self.set_icon(project.icon.as_deref());
    }

    pub fn get_project(&self) -> ProjectRequest<'_> {
        ProjectRequest {
            name: self.name(),
            script: self.script(),
            parent: self.parent(),
            icon: self.icon(),
        }
    }
}

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
