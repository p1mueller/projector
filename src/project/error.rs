//! Errors returned by the project handler.
//!
//! The single [`ProjectError`] enum covers validation, script-availability,
//! filesystem, and execution failures, with user-facing `Display` text.

use std::{fmt::Display, io::Error, path::PathBuf};
use thiserror::Error;

/// Errors produced by the project handler.
///
/// Covers validation failures while adding/editing projects, problems locating
/// or checking a project's script, filesystem errors, and execution failures.
/// Each variant's `Display` text (see the impl below) is safe to show users.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// A project with the same `name` or `script` value already exists.
    /// The payload is the duplicated field name (`"name"` or `"script"`).
    AlreadyExists(&'static str),
    /// A required input field was empty. The payload names the field
    /// (`"name"` or `"script"`).
    InvalidField(&'static str),
    /// The referenced project is not part of this handler's list.
    InvalidProject,
    /// The script backing a project cannot be launched.
    Unavailable {
        /// Path of the offending script.
        path: PathBuf,
        /// Human-readable reason (e.g. `"does not exist"`).
        reason: String,
    },
    /// A filesystem operation failed.
    IOError(Error),
    /// A project script failed to run.
    ExecutionError(Error),
}

// User-facing messages for each variant.
impl Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(f, "Field `{field}` must not be empty"),
            Self::Unavailable { path, reason } => write!(f, "Script at {path:?} {reason}"),
            Self::AlreadyExists(field) => write!(f, "Entry with same `{field}` value exists"),
            Self::InvalidProject => write!(f, "Project was created outside of Project Handler"),
            Self::IOError(error) => error.fmt(f),
            Self::ExecutionError(error) => {
                write!(f, "There was an error during execution:\n{error}")
            }
        }
    }
}

impl From<Error> for ProjectError {
    fn from(value: Error) -> Self {
        ProjectError::IOError(value)
    }
}
