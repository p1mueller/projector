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
            create_template(request.script, &script)?
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
            .output()
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

pub fn create_template(name: &str, path: &PathBuf) -> std::io::Result<()> {
    std::fs::write(
        path,
        format!(
            r#"#!/bin/bash
project_folder="/mnt/data/projects/{}"
script_folder="$(dirname "$(readlink -f "$0")")"
"#,
            name
        ),
    )?;
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(path) {
        let mut permissions = meta.permissions();
        permissions.set_mode(0o750);
        std::fs::set_permissions(path, permissions)?;
    }

    Ok(())
}
