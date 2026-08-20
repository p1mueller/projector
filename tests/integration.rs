//! End-to-end integration tests against the public `project` API.
//!
//! These drive the same code paths a user would hit through the TUI
//! (add → launch → edit → read) but without instantiating a terminal, so
//! they can run in any environment.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::Path};
use tempfile::TempDir;

use projector::project::{LaunchResult, ProjectHandler, ProjectRequest};

fn handler_in(folder: &Path) -> ProjectHandler {
    ProjectHandler::new(folder.to_path_buf(), "settings.json")
}

fn req<'a>(name: &'a str, script: &'a str) -> ProjectRequest<'a> {
    ProjectRequest {
        name,
        script,
        parent: "",
        icon: "",
    }
}

fn set_mode(path: &std::path::Path, mode: u32) {
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(mode);
        fs::set_permissions(path, permissions).unwrap();
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
}

fn collect<F>(rx: std::sync::mpsc::Receiver<F>) -> F {
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .expect("timed out waiting for launch result")
}

/// Writes an executable shell script into `dir` and returns its path.
fn write_exec_script(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("could not write script");
    set_mode(&path, 0o755);
    path
}

#[test]
fn add_persists_to_config_and_round_trips() {
    let tmp = TempDir::new().unwrap();
    let mut h = handler_in(tmp.path());
    h.add_project(req("Alpha", "alpha.sh")).unwrap();
    h.add_project(req("Beta", "beta.sh")).unwrap();
    h.write_config().unwrap();

    // A fresh handler (as if the app was restarted) must see the same projects.
    let mut fresh = handler_in(tmp.path());
    fresh.read_config().unwrap();
    let names: Vec<_> = fresh.projects().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["Alpha", "Beta"]);
    for p in fresh.projects() {
        assert!(p.valid, "scripts should exist and be marked valid");
    }
}

#[test]
fn launch_reports_success_via_background_callback() {
    let tmp = TempDir::new().unwrap();
    let mut h = handler_in(tmp.path());
    h.add_project(req("Hi", "hi.sh")).unwrap();
    // Overwrite the auto-created template with a real script.
    write_exec_script(tmp.path(), "hi.sh", "#!/bin/sh\necho hello projector\n");

    let project = h.projects().first().cloned().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    h.launch_project(&project, move |r| {
        let _ = tx.send(r);
    })
    .unwrap();

    let launch = collect(rx);
    let LaunchResult {
        success,
        code,
        stdout,
        stderr: _,
    } = launch.expect("spawn should succeed");
    assert!(success);
    assert_eq!(code, Some(0));
    assert!(stdout.trim().ends_with("hello projector"), "got {stdout:?}");
}

#[test]
fn launch_reports_failure_with_stderr_and_exit_code() {
    let tmp = TempDir::new().unwrap();
    let mut h = handler_in(tmp.path());
    h.add_project(req("Boom", "boom.sh")).unwrap();
    write_exec_script(tmp.path(), "boom.sh", "#!/bin/sh\necho oops >&2\nexit 7\n");

    let project = h.projects().first().cloned().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    h.launch_project(&project, move |r| {
        let _ = tx.send(r);
    })
    .unwrap();

    let launch = collect(rx);
    let LaunchResult {
        success,
        code,
        stdout,
        stderr,
    } = launch.expect("spawn should succeed");
    assert!(!success);
    assert_eq!(code, Some(7));
    assert_eq!(stderr.trim_end(), "oops");
    assert!(stdout.trim().is_empty());
}

#[test]
fn launch_rejects_non_executable_script() {
    let tmp = TempDir::new().unwrap();
    let mut h = handler_in(tmp.path());
    h.add_project(req("RO", "ro.sh")).unwrap();
    set_mode(&tmp.path().join("ro.sh"), 0o644);

    let project = h.projects().first().cloned().unwrap();
    let result = h.launch_project(&project, |_| {});
    assert!(
        matches!(
            result,
            Err(projector::project::ProjectError::Unavailable { .. })
        ),
        "expected Unavailable error, got {result:?}"
    );
}

#[test]
fn edit_replaces_an_existing_project() {
    let tmp = TempDir::new().unwrap();
    let mut h = handler_in(tmp.path());
    h.add_project(req("Original", "orig.sh")).unwrap();
    let original = h.projects().first().cloned().unwrap();

    h.edit_project(&original, req("Renamed", "renamed.sh"))
        .unwrap();

    assert_eq!(h.projects().len(), 1);
    assert_eq!(h.projects()[0].name, "Renamed");
    assert_eq!(h.projects()[0].script_name, "renamed.sh");
}

#[test]
fn remove_then_persist_drops_the_entry() {
    let tmp = TempDir::new().unwrap();
    let mut h = handler_in(tmp.path());
    h.add_project(req("Keep", "keep.sh")).unwrap();
    h.add_project(req("Drop", "drop.sh")).unwrap();
    h.write_config().unwrap();

    let drop = h
        .projects()
        .iter()
        .find(|p| p.name == "Drop")
        .unwrap()
        .clone();
    h.remove_project(&drop).unwrap();
    h.write_config().unwrap();

    let mut fresh = handler_in(tmp.path());
    fresh.read_config().unwrap();
    assert_eq!(
        fresh.projects().len(),
        1,
        "only the kept project should remain"
    );
    assert_eq!(fresh.projects()[0].name, "Keep");
}
