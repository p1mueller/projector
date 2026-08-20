use std::{fmt::Display, io::Error, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    AlreadyExists(&'static str),
    InvalidField(&'static str),
    InvalidProject,
    Unavailable { path: PathBuf, reason: String },
    IOError(Error),
    ExecutionError(Error),
}

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
