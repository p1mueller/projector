//! In-memory data model for a project and the request used to create one.

/// A single runnable project known to the handler.
///
/// A project is a shell script (identified by [`Project::script_name`]) plus
/// display metadata. `valid` reports whether the script file actually exists
/// on disk at the time the project was loaded.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Display name shown in the UI.
    pub name: String,
    /// File name of the script relative to the project folder.
    pub script_name: String,
    /// Optional parent/group the project belongs to.
    pub parent: Option<String>,
    /// Optional icon (e.g. an emoji) shown next to the project.
    pub icon: Option<String>,
    /// Whether the script file exists on disk.
    pub valid: bool,
}

/// User-provided fields for adding or editing a project.
///
/// Borrowed strings to avoid copies at the call site. `parent` and `icon` are
/// empty strings for "unset" and are converted to `None` when constructing a
/// [`Project`].
pub struct ProjectRequest<'a> {
    /// Display name (must be non-empty).
    pub name: &'a str,
    /// Script file name (must be non-empty).
    pub script: &'a str,
    /// Parent/group, or empty for none.
    pub parent: &'a str,
    /// Icon, or empty for none.
    pub icon: &'a str,
}

impl Project {
    /// Create a project from its individual fields.
    pub fn new(
        name: String,
        script_name: String,
        parent: Option<String>,
        icon: Option<String>,
        valid: bool,
    ) -> Self {
        Self {
            name,
            script_name,
            parent,
            icon,
            valid,
        }
    }
}

impl From<ProjectRequest<'_>> for Project {
    // Builds a valid [`Project`] from a request: blank `parent`/`icon`
    // become `None` and `valid` is set to `true`.
    fn from(value: ProjectRequest) -> Self {
        let parent = str_to_option(value.parent);
        let icon = str_to_option(value.icon);
        Project::new(
            value.name.to_owned(),
            value.script.to_owned(),
            parent,
            icon,
            true,
        )
    }
}

/// Convert a string to `Option<String>`: empty maps to `None`, anything else to `Some`.
pub fn str_to_option(value: &str) -> Option<String> {
    (!value.is_empty()).then_some(value).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req<'a>(
        name: &'a str,
        script: &'a str,
        parent: &'a str,
        icon: &'a str,
    ) -> ProjectRequest<'a> {
        ProjectRequest {
            name,
            script,
            parent,
            icon,
        }
    }

    #[test]
    fn str_to_option_maps_empty_to_none() {
        assert_eq!(str_to_option(""), None);
    }

    #[test]
    fn str_to_option_maps_non_empty_to_some() {
        assert_eq!(str_to_option("abc"), Some("abc".to_string()));
        assert_eq!(str_to_option("🐘"), Some("🐘".to_string()));
    }

    #[test]
    fn request_with_all_fields_produces_full_project() {
        let project = Project::from(req("Name", "script.sh", "backend", "🐘"));
        assert_eq!(project.name, "Name");
        assert_eq!(project.script_name, "script.sh");
        assert_eq!(project.parent.as_deref(), Some("backend"));
        assert_eq!(project.icon.as_deref(), Some("\u{1F418}"));
        assert!(
            project.valid,
            "freshly added projects are always marked valid"
        );
    }

    #[test]
    fn request_with_optional_fields_blank_maps_to_none() {
        let project = Project::from(req("Bare", "bare.sh", "", ""));
        assert_eq!(project.parent, None);
        assert_eq!(project.icon, None);
    }

    #[test]
    fn from_request_matches_direct_construction() {
        let via_request = Project::from(req("X", "x.sh", "same", "same"));
        let direct = Project::new(
            "X".to_string(),
            "x.sh".to_string(),
            Some("same".to_string()),
            Some("same".to_string()),
            true,
        );
        assert_eq!(via_request, direct);
    }
}

// pub fn edit_script(&self) -> std::io::Result<bool> {
//     let before = get_modified_time(&self.script)?;
//     Command::new("$EDITOR").arg(&self.script).output()?;
//     let after = get_modified_time(&self.script)?;
// }

// fn get_modified_time(path: &PathBuf) -> std::io::Result<SystemTime> {
//     std::fs::metadata(path)?.modified()
// }
