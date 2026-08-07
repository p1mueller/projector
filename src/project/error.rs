use std::{fmt::Display, io::Error};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    AlreadyExists(&'static str),
    InvalidField(&'static str),
    ScriptDoesNotExist,
    InvalidIndex(usize),
    IOError(Error),
    ExecutionError(Error),
}

impl Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(f, "Field `{field}` must not be empty"),
            Self::ScriptDoesNotExist => write!(f, "Script path does not exist"),
            Self::AlreadyExists(field) => write!(f, "Entry with same `{field}` value exists"),
            Self::InvalidIndex(index) => write!(f, "Cannot find a project with index {index}"),
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
