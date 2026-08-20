#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub script_name: String,
    pub parent: Option<String>,
    pub icon: Option<String>,
    pub valid: bool,
}

pub struct ProjectRequest<'a> {
    pub name: &'a str,
    pub script: &'a str,
    pub parent: &'a str,
    pub icon: &'a str,
}

impl Project {
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
