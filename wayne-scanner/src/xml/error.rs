use std::fmt::Debug;

use thiserror::Error;

pub type XmlResult<T> = Result<T, XmlError>;

#[derive(Debug, Error)]
#[error("Invalid XML [{location}]: {error}")]
pub struct XmlError {
    error: anyhow::Error,
    location: u64,
}

impl XmlError {
    pub fn new(error: anyhow::Error, location: u64) -> Self {
        Self { error, location }
    }
}
