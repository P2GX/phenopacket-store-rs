use std::{error::Error};
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;

/// The error returned when a non-normal execution happens, e.g. network connectivity issue,
/// but not when a phenopacket is searched and not found.
#[derive(Debug)]
#[non_exhaustive]
pub enum PhenoStoreError {
    NotFound,
    AlreadyExists,
    InvalidData,

    Io(IoErrorKind),

    Other(Box<dyn Error + Send + Sync>),
}

#[derive(Debug)]
pub enum IoErrorKind {
    PermissionDenied,
    ReadOnly,
    CapacityExceeded,
    Timeout,
    Initialization,
}

impl From<std::io::Error> for PhenoStoreError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::InvalidData => Self::InvalidData,

            std::io::ErrorKind::PermissionDenied => Self::Io(IoErrorKind::PermissionDenied),
            std::io::ErrorKind::ReadOnlyFilesystem => Self::Io(IoErrorKind::ReadOnly),

            _ => Self::Other(Box::new(error)),
        }
    }
}

pub trait PhenoStore {
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError>;
    fn remove(&self, id: &Uuid) -> Result<(), PhenoStoreError>;
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<(), PhenoStoreError>;
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError>;
}
