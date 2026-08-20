pub mod config;
pub mod error;
pub mod model;

pub use config::{Config, ProjectConfig};
use directories::UserDirs;
pub use error::ProjectError;
pub use model::{Project, ProjectRequest};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
pub struct ProjectHandler {
    project_folder: PathBuf,
    config_path: PathBuf,
    projects: Vec<Project>,
}

impl Default for ProjectHandler {
    fn default() -> Self {
        Self::new(get_projects_path(None), "settings.json")
    }
}

impl ProjectHandler {
    pub fn new(folder: PathBuf, config_name: &str) -> Self {
        std::fs::create_dir_all(&folder).expect("Failed to create the project folder");
        let config_path = folder.join(config_name);
        Self::from_config_path(&config_path)
    }

    pub fn from_config_path(path: &Path) -> Self {
        if !path.exists() {
            std::fs::write(path, "{}").expect("Failed to create the configuration file");
        }
        let config_path = std::fs::canonicalize(path).expect("Path couldn't be resolved");
        let project_folder = path.parent().expect("There should be a parent");
        Self {
            project_folder: project_folder.to_owned(),
            config_path,
            projects: Vec::new(),
        }
    }

    pub fn read_config(&mut self) -> std::io::Result<()> {
        let content = std::fs::read_to_string(&self.config_path)?;
        let config: Config = serde_json::from_str(&content)?;

        self.projects = config
            .into_iter()
            .map(|(file_name, project)| {
                let valid = self.project_folder.join(&file_name).exists();
                Project::new(project.name, file_name, project.parent, project.icon, valid)
            })
            .collect();
        self.sort_projects();
        Ok(())
    }

    pub fn write_config(&self) -> std::io::Result<()> {
        let mut config = BTreeMap::new();
        for project in &self.projects {
            let key = project.script_name.clone();
            config.insert(
                key,
                ProjectConfig {
                    name: project.name.clone(),
                    parent: project.parent.clone(),
                    icon: project.icon.clone(),
                },
            );
        }
        let serialized = serde_json::to_string_pretty(&config)?;
        std::fs::write(&self.config_path, serialized)
    }

    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    pub fn filter_projects(&self, text: &str) -> impl Iterator<Item = &Project> {
        let text = text.to_lowercase();
        self.projects.iter().filter(move |p| {
            p.name.to_lowercase().contains(&text)
                || p.parent
                    .as_deref()
                    .is_some_and(|parent| parent.to_lowercase().contains(&text))
        })
    }

    pub fn get_project(&self, index: usize) -> &Project {
        &self.projects[index]
    }

    pub fn get_project_mut(&mut self, index: usize) -> &mut Project {
        &mut self.projects[index]
    }

    pub fn add_project(&mut self, request: ProjectRequest) -> Result<(), ProjectError> {
        if request.name.is_empty() {
            return Err(ProjectError::InvalidField("name"));
        }
        if request.script.is_empty() {
            return Err(ProjectError::InvalidField("script"));
        }

        if self.projects.iter().any(|p| p.name == request.name) {
            return Err(ProjectError::AlreadyExists("name"));
        }

        if self
            .projects
            .iter()
            .any(|p| p.script_name == request.script)
        {
            return Err(ProjectError::AlreadyExists("script"));
        }
        let script = self.project_folder.join(request.script);
        if !(script.exists() && script.is_file()) {
            create_template(&script)?
        }

        self.projects.push(request.into());
        self.sort_projects();
        Ok(())
    }

    pub fn edit_project(
        &mut self,
        project: &Project,
        request: ProjectRequest,
    ) -> Result<(), ProjectError> {
        self.remove_project(project)?;
        self.add_project(request)
    }

    pub fn remove_project(&mut self, project: &Project) -> Result<(), ProjectError> {
        let index = self.index_of(project)?;
        self.projects.remove(index);
        Ok(())
    }

    pub fn launch_project(&self, project: &Project) -> Result<(), ProjectError> {
        let path = self.script_path(project);
        Command::new(path)
            .spawn()
            .map_err(ProjectError::ExecutionError)?;
        Ok(())
    }

    pub fn edit_settings(&self) -> Result<(), ProjectError> {
        edit::edit_file(&self.config_path).map_err(ProjectError::IOError)
    }

    pub fn edit_project_script(&self, project: &Project) -> Result<(), ProjectError> {
        let path = self.script_path(project);
        Self::edit_file(path)
    }

    pub fn index_of(&self, project: &Project) -> Result<usize, ProjectError> {
        self.projects
            .iter()
            .position(|p| p == project)
            .ok_or(ProjectError::InvalidProject)
    }

    fn edit_file<P: AsRef<Path>>(path: P) -> Result<(), ProjectError> {
        edit::edit_file(path).map_err(ProjectError::IOError)
    }

    fn sort_projects(&mut self) {
        self.projects.sort_by(|a, b| a.name.cmp(&b.name));
    }

    pub fn script_path(&self, project: &Project) -> PathBuf {
        self.project_folder.join(&project.script_name)
    }
}

pub fn get_projects_path(project_folder: Option<&str>) -> PathBuf {
    let project_folder = project_folder.unwrap_or(".projects");
    UserDirs::new()
        .expect("No valid user directories")
        .home_dir()
        .join(project_folder)
}

pub fn get_file_name(path: &Path) -> &str {
    path.file_name().unwrap().to_str().unwrap()
}

pub fn create_template(path: &PathBuf) -> std::io::Result<()> {
    std::fs::write(
        path,
        r#"#!/bin/bash
script_folder="$(dirname "$(readlink -f "$0")")"
"#,
    )?;
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(path) {
        let mut permissions = meta.permissions();
        permissions.set_mode(0o750);
        std::fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp_handler() -> (TempDir, ProjectHandler) {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path().to_path_buf();
        (tmp, ProjectHandler::new(folder, "settings.json"))
    }

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
    fn add_project_appends_and_sorts_by_name() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Zebra", "zebra.sh", "", "")).unwrap();
        h.add_project(req("Apple", "apple.sh", "", "")).unwrap();
        h.add_project(req("Mango", "mango.sh", "", "")).unwrap();
        let names: Vec<_> = h.projects().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Apple", "Mango", "Zebra"]);
    }

    #[test]
    fn add_project_rejects_empty_name_and_script() {
        let (_tmp, mut h) = tmp_handler();
        let err = h.add_project(req("", "x.sh", "", "")).unwrap_err();
        assert!(
            matches!(err, ProjectError::InvalidField("name")),
            "got {err}"
        );
        let err = h.add_project(req("X", "", "", "")).unwrap_err();
        assert!(
            matches!(err, ProjectError::InvalidField("script")),
            "got {err}"
        );
    }

    #[test]
    fn add_project_rejects_duplicate_name() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Duplicated", "one.sh", "", "")).unwrap();
        let err = h
            .add_project(req("Duplicated", "two.sh", "", ""))
            .unwrap_err();
        assert!(
            matches!(err, ProjectError::AlreadyExists("name")),
            "got {err}"
        );
    }

    #[test]
    fn add_project_rejects_duplicate_script() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("First", "shared.sh", "", "")).unwrap();
        let err = h
            .add_project(req("Second", "shared.sh", "", ""))
            .unwrap_err();
        assert!(
            matches!(err, ProjectError::AlreadyExists("script")),
            "got {err}"
        );
    }

    #[test]
    fn add_project_creates_template_when_script_missing() {
        let (tmp, mut h) = tmp_handler();
        h.add_project(req("Auto", "new-script.sh", "", "")).unwrap();
        let path = tmp.path().join("new-script.sh");
        assert!(path.is_file());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("#!/bin/bash"));
    }

    #[test]
    fn filter_projects_matches_name_case_insensitive() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Alpha Service", "a.sh", "", "")).unwrap();
        h.add_project(req("Beta Service", "b.sh", "", "")).unwrap();
        h.add_project(req("Unrelated", "c.sh", "", "")).unwrap();

        let lower: Vec<_> = h
            .filter_projects("service")
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(lower, vec!["Alpha Service", "Beta Service"]);
        let upper: Vec<_> = h
            .filter_projects("SERVICE")
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(upper, vec!["Alpha Service", "Beta Service"]);
    }

    #[test]
    fn filter_projects_matches_parent() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("App", "a.sh", "backend", "")).unwrap();
        h.add_project(req("Job", "j.sh", "backend", "")).unwrap();
        h.add_project(req("Web", "w.sh", "frontend", "")).unwrap();

        let hits: Vec<_> = h
            .filter_projects("backend")
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(hits, vec!["App", "Job"]);
    }

    #[test]
    fn filter_projects_no_match_yields_empty() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Alpha", "a.sh", "", "")).unwrap();
        assert!(h.filter_projects("zzz-unknown").next().is_none());
    }

    #[test]
    fn remove_project_removes_entry() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Target", "t.sh", "", "")).unwrap();
        h.add_project(req("Keep", "k.sh", "", "")).unwrap();
        assert_eq!(h.projects().len(), 2);

        let target = h
            .projects()
            .iter()
            .find(|p| p.name == "Target")
            .unwrap()
            .clone();
        h.remove_project(&target).unwrap();
        assert_eq!(h.projects().len(), 1);
        assert_eq!(h.projects()[0].name, "Keep");
    }

    #[test]
    fn remove_project_unknown_fails_with_invalid_project() {
        let (_tmp, mut h) = tmp_handler();
        let phantom = Project::new(
            "ghost".to_string(),
            "ghost.sh".to_string(),
            None,
            None,
            true,
        );
        let err = h.remove_project(&phantom).unwrap_err();
        assert!(matches!(err, ProjectError::InvalidProject), "got {err}");
    }

    #[test]
    fn edit_project_replaces_name_and_script() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Original", "orig.sh", "", "")).unwrap();
        let original = h.projects().first().unwrap().clone();
        h.edit_project(&original, req("Renamed", "renamed.sh", "", ""))
            .unwrap();
        assert_eq!(h.projects().len(), 1);
        assert_eq!(h.projects()[0].name, "Renamed");
        assert_eq!(h.projects()[0].script_name, "renamed.sh");
    }

    #[test]
    fn read_then_write_round_trip_preserves_fields() {
        let (tmp, mut h) = tmp_handler();
        h.add_project(req("Alpha", "a.sh", "backend", "\u{1F170}"))
            .unwrap();
        h.add_project(req("Beta", "b.sh", "", "")).unwrap();
        h.write_config().unwrap();

        let mut fresh = ProjectHandler::new(tmp.path().to_path_buf(), "settings.json");
        fresh.read_config().unwrap();

        let names: Vec<_> = fresh.projects().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Beta"]);

        let alpha = fresh.projects().iter().find(|p| p.name == "Alpha").unwrap();
        assert_eq!(alpha.parent.as_deref(), Some("backend"));
        assert_eq!(alpha.icon.as_deref(), Some("\u{1F170}"));
        assert!(alpha.valid);
    }

    #[test]
    fn read_config_flags_missing_scripts_as_invalid() {
        let (tmp, mut h) = tmp_handler();
        h.add_project(req("Phantom", "no-such-script.sh", "", ""))
            .unwrap();
        h.write_config().unwrap();
        fs::remove_file(tmp.path().join("no-such-script.sh")).unwrap();

        let mut fresh = ProjectHandler::new(tmp.path().to_path_buf(), "settings.json");
        fresh.read_config().unwrap();
        assert_eq!(fresh.projects().len(), 1);
        assert!(
            !fresh.projects()[0].valid,
            "expected a missing script to be flagged invalid"
        );
    }

    #[test]
    fn create_template_starts_with_shebang() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("tpl.sh");
        create_template(&path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("#!/bin/bash"), "got {content:?}");
        assert!(content.contains("script_folder"));
    }

    #[test]
    #[cfg(unix)]
    fn create_template_sets_executable_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("tpl.sh");
        create_template(&path).unwrap();
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o750);
    }
}
