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

// pub fn edit_script(&self) -> std::io::Result<bool> {
//     let before = get_modified_time(&self.script)?;
//     Command::new("$EDITOR").arg(&self.script).output()?;
//     let after = get_modified_time(&self.script)?;
// }

// fn get_modified_time(path: &PathBuf) -> std::io::Result<SystemTime> {
//     std::fs::metadata(path)?.modified()
// }
