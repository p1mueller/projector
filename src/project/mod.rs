//! Manage a user's projects: load, save, add, edit, filter, sort, and launch
//! shell-script projects configured in a per-project-folder JSON file.
//!
//! The entry point is [`ProjectHandler`]; supporting types live in
//! [`config`](self::config::Config), [`error`](self::error::ProjectError),
//! [`model`](self::model::Project), and [`sort`](self::sort::SortMode).

pub mod config;
pub mod error;
pub mod model;
pub mod sort;

pub use config::{Config, ProjectConfig};
use directories::UserDirs;
pub use error::ProjectError;
pub use model::{Project, ProjectRequest};
pub use sort::SortMode;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

/// Outcome of a launched project script.
///
/// Produced by [`ProjectHandler::launch_project`] and handed to `on_done`
/// once the script finishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResult {
    /// Whether the script exited with a success status.
    pub success: bool,
    /// The script's exit code, if it exited normally.
    pub code: Option<i32>,
    /// The script's standard output.
    pub stdout: String,
    /// The script's standard error.
    pub stderr: String,
}

/// Manages a collection of projects on disk and in memory.
///
/// Owns the folder holding the project scripts, the path to the JSON config
/// file, and the in-memory project list that can be inspected, edited, and
/// persisted.
#[derive(Debug)]
pub struct ProjectHandler {
    /// Folder containing the project scripts.
    project_folder: PathBuf,
    /// Path to the JSON config file.
    config_path: PathBuf,
    /// Projects currently loaded, in order.
    projects: Vec<Project>,
}

// Default handler rooted at `~/.projects` with a `settings.json` config file.
impl Default for ProjectHandler {
    fn default() -> Self {
        Self::new(get_projects_path(None), "settings.json")
    }
}

impl ProjectHandler {
    /// Create a handler for `folder` (created if missing) with a config file
    /// named `config_name` inside it.
    pub fn new(folder: PathBuf, config_name: &str) -> Self {
        std::fs::create_dir_all(&folder).expect("Failed to create the project folder");
        let config_path = folder.join(config_name);
        Self::from_config_path(&config_path)
    }

    /// Create a handler from the config file at `path`, writing an empty
    /// config (`{}`) first if the file does not exist.
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

    /// Load projects from the config file into memory.
    ///
    /// A project whose script file is missing from the project folder is
    /// loaded with `valid` set to `false`.
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
        Ok(())
    }

    /// Persist the in-memory project list to the config file as pretty-printed JSON.
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

    /// All projects in current order.
    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    /// Projects whose name or parent contains `text` (case-insensitive).
    pub fn filter_projects(&self, text: &str) -> impl Iterator<Item = &Project> {
        let text = text.to_lowercase();
        self.projects.iter().filter(move |p| {
            p.name.to_lowercase().contains(&text)
                || p.parent
                    .as_deref()
                    .is_some_and(|parent| parent.to_lowercase().contains(&text))
        })
    }

    /// The project at `index` (panics if out of range).
    pub fn get_project(&self, index: usize) -> &Project {
        &self.projects[index]
    }

    /// Mutable access to the project at `index` (panics if out of range).
    pub fn get_project_mut(&mut self, index: usize) -> &mut Project {
        &mut self.projects[index]
    }

    /// Add a project, creating a template script first if none exists at
    /// `request.script`.
    ///
    /// # Errors
    /// - `InvalidField` if `name` or `script` is empty.
    /// - `AlreadyExists` if the name or script is already in use.
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
        Ok(())
    }

    /// Replace `project` with the values in `request` (remove then re-add).
    pub fn edit_project(
        &mut self,
        project: &Project,
        request: ProjectRequest,
    ) -> Result<(), ProjectError> {
        // Replaces `project` with `request` by removing then re-adding it.
        self.remove_project(project)?;
        self.add_project(request)
    }

    /// Remove `project` from the list.
    ///
    /// # Errors
    /// - `InvalidProject` if `project` is not part of this handler.
    pub fn remove_project(&mut self, project: &Project) -> Result<(), ProjectError> {
        let index = self.index_of(project)?;
        self.projects.remove(index);
        Ok(())
    }

    /// Launches a project script in the background.
    ///
    /// Fails immediately if the script is missing or not executable. Otherwise the script is
    /// run on a worker thread and `on_done` is invoked with its output once it finishes.
    pub fn launch_project(
        &self,
        project: &Project,
        on_done: impl FnOnce(Result<LaunchResult, String>) + Send + 'static,
    ) -> Result<(), ProjectError> {
        let path = self.script_path(project);
        check_executable(&path)?;

        std::thread::spawn(move || {
            let result = match Command::new(path).output() {
                Ok(output) => Ok(LaunchResult {
                    success: output.status.success(),
                    code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                }),
                Err(error) => Err(error.to_string()),
            };
            on_done(result);
        });
        Ok(())
    }

    /// Open the config file in the user's editor.
    pub fn edit_settings(&self) -> Result<(), ProjectError> {
        edit::edit_file(&self.config_path).map_err(ProjectError::IOError)
    }

    /// Open `project`'s script file in the user's editor.
    pub fn edit_project_script(&self, project: &Project) -> Result<(), ProjectError> {
        let path = self.script_path(project);
        Self::edit_file(path)
    }

    /// Position of `project` in the list.
    ///
    /// # Errors
    /// - `InvalidProject` if `project` is not part of this handler.
    pub fn index_of(&self, project: &Project) -> Result<usize, ProjectError> {
        self.projects
            .iter()
            .position(|p| p == project)
            .ok_or(ProjectError::InvalidProject)
    }

    fn edit_file<P: AsRef<Path>>(path: P) -> Result<(), ProjectError> {
        edit::edit_file(path).map_err(ProjectError::IOError)
    }

    /// Absolute path to `project`'s script file.
    pub fn script_path(&self, project: &Project) -> PathBuf {
        self.project_folder.join(&project.script_name)
    }

    /// Reorders the in-memory project list according to `mode`.
    pub fn sort_projects(&mut self, mode: SortMode) {
        mode.apply(&mut self.projects)
    }
}

/// Returns `Ok` if `path` exists and is an executable regular file.
pub fn check_executable(path: &Path) -> Result<(), ProjectError> {
    let metadata = std::fs::metadata(path).map_err(|_| ProjectError::Unavailable {
        path: path.to_path_buf(),
        reason: "does not exist".into(),
    })?;
    if !(metadata.is_file() && is_executable(&metadata)) {
        return Err(ProjectError::Unavailable {
            path: path.to_path_buf(),
            reason: "is not an executable regular file".into(),
        });
    }
    Ok(())
}

// Any execute bit set (owner, group, or other) counts as executable.
#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

// Non-Unix targets have no meaningful mode bits; assume executable.
#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}

/// Default projects location: `~/.projects` (`project_folder` if given).
pub fn get_projects_path(project_folder: Option<&str>) -> PathBuf {
    let project_folder = project_folder.unwrap_or(".projects");
    UserDirs::new()
        .expect("No valid user directories")
        .home_dir()
        .join(project_folder)
}

/// Write a new shell-script template to `path` and, on Unix, make it
/// executable (mode `0o750`).
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
    fn sort_projects_by_name() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Zebra", "zebra.sh", "", "")).unwrap();
        h.add_project(req("Apple", "apple.sh", "", "")).unwrap();
        h.add_project(req("Mango", "mango.sh", "", "")).unwrap();
        h.sort_projects(SortMode::Name);
        let names: Vec<_> = h.projects().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Apple", "Mango", "Zebra"]);
    }

    #[test]
    fn sort_projects_by_parent_with_missing_last() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("Zeta", "z.sh", "frontend", "")).unwrap();
        h.add_project(req("Alpha", "a.sh", "backend", "")).unwrap();
        h.add_project(req("Bare", "b.sh", "", "")).unwrap();
        h.add_project(req("Mid", "m.sh", "Frontend", "")).unwrap();
        h.sort_projects(SortMode::Parent);
        // "backend" < "frontend" (case-insensitive); no-parent entries go last.
        let parents: Vec<Option<String>> = h.projects().iter().map(|p| p.parent.clone()).collect();
        assert_eq!(
            parents,
            vec![
                Some("backend".into()),
                Some("Frontend".into()),
                Some("frontend".into()),
                None,
            ]
        );
    }

    #[test]
    fn sort_projects_by_script() {
        let (_tmp, mut h) = tmp_handler();
        h.add_project(req("First", "zeta.sh", "", "")).unwrap();
        h.add_project(req("Last", "alpha.sh", "", "")).unwrap();
        h.add_project(req("Middle", "mid.sh", "", "")).unwrap();
        h.sort_projects(SortMode::Script);
        let scripts: Vec<_> = h
            .projects()
            .iter()
            .map(|p| p.script_name.as_str())
            .collect();
        assert_eq!(scripts, vec!["alpha.sh", "mid.sh", "zeta.sh"]);
    }

    #[test]
    fn sort_mode_cycles_name_parent_script() {
        assert_eq!(SortMode::Name.next(), SortMode::Parent);
        assert_eq!(SortMode::Parent.next(), SortMode::Script);
        assert_eq!(SortMode::Script.next(), SortMode::Name);
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

    fn collect<F>(
        value: std::sync::mpsc::Receiver<F>,
    ) -> std::result::Result<F, std::sync::mpsc::RecvTimeoutError> {
        value
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| std::sync::mpsc::RecvTimeoutError::Timeout)
    }

    fn project_in(script: &str) -> Project {
        Project::new("test".to_string(), script.to_string(), None, None, true)
    }

    #[test]
    fn launch_project_rejects_missing_script() {
        let (_tmp, h) = tmp_handler();
        let project = project_in("not-there.sh");
        let result = h.launch_project(&project, |result: Result<LaunchResult, String>| {
            let _ = result;
        });
        let err = result.unwrap_err();
        assert!(matches!(err, ProjectError::Unavailable { .. }), "got {err}");
    }

    #[test]
    #[cfg(unix)]
    fn launch_project_rejects_non_executable_script() {
        use std::os::unix::fs::PermissionsExt;
        let (tmp, mut h) = tmp_handler();
        let script = tmp.path().join("read-only.sh");
        fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&script, permissions).unwrap();

        h.add_project(req("x", "read-only.sh", "", "")).unwrap();
        let project = h.projects().first().unwrap().clone();
        let result = h.launch_project(&project, |_| {});
        let err = result.unwrap_err();
        assert!(matches!(err, ProjectError::Unavailable { .. }), "got {err}");
    }

    #[test]
    #[cfg(unix)]
    fn launch_project_reports_success_and_output() {
        let (tmp, h) = tmp_handler();
        let script = tmp.path().join("ok.sh");
        fs::write(&script, "#!/bin/sh\necho launched\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&script).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script, permissions).unwrap();
        }

        let project = project_in("ok.sh");
        let (tx, rx) = std::sync::mpsc::channel();
        h.launch_project(&project, move |result| {
            let _ = tx.send(result);
        })
        .unwrap();

        let result = collect(rx).unwrap();
        let result = result.expect("expected Ok(LaunchResult)");
        assert!(result.success);
        assert!(result.stderr.is_empty());
        assert!(result.stdout.trim_end().ends_with("launched"));
    }

    #[test]
    #[cfg(unix)]
    fn launch_project_reports_non_zero_exit_code_and_stderr() {
        let (tmp, h) = tmp_handler();
        let script = tmp.path().join("fail.sh");
        fs::write(&script, "#!/bin/sh\necho boom >&2\nexit 3\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&script).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script, permissions).unwrap();
        }

        let project = project_in("fail.sh");
        let (tx, rx) = std::sync::mpsc::channel();
        h.launch_project(&project, move |result| {
            let _ = tx.send(result);
        })
        .unwrap();

        let result = collect(rx).unwrap();
        let result = result.expect("expected Ok(LaunchResult)");
        assert!(!result.success);
        assert_eq!(result.code, Some(3));
        assert_eq!(result.stderr.trim_end(), "boom");
    }

    #[test]
    #[cfg(unix)]
    fn check_executable_returns_not_executable_for_read_only() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ro.sh");
        fs::write(&path, "#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&path, permissions).unwrap();

        let err = check_executable(&path).unwrap_err();
        assert!(matches!(err, ProjectError::Unavailable { .. }), "got {err}");
    }
}
