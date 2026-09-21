use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;

/// The error returned when a non-normal execution happens, e.g. network connectivity issue,
/// but not when a phenopacket is searched and not found.
#[derive(Debug)]
#[non_exhaustive]
pub enum PhenoStoreError {
    NotFound,
    AlreadyExists,
    InvalidInput,
    InvalidData,

    Storage(StorageError),

    Other,
}

#[derive(Debug)]
pub enum StorageError {
    PermissionDenied,
    ReadOnly,
    CapacityExceeded,
    Unavailable,
    Timeout,
    Initialization,
    Corrupted,
    Other,
}

impl From<IoError> for PhenoStoreError {
    fn from(error: IoError) -> Self {
        match error.kind() {
            IoErrorKind::AlreadyExists => Self::AlreadyExists,
            IoErrorKind::NotFound => Self::NotFound,
            IoErrorKind::InvalidData => Self::InvalidData,
            IoErrorKind::InvalidInput => Self::InvalidInput,

            IoErrorKind::PermissionDenied => Self::Storage(StorageError::PermissionDenied),
            IoErrorKind::ReadOnlyFilesystem => Self::Storage(StorageError::ReadOnly),
            //TODO: add other storage errors
            _ => Self::Other,
        }
    }
}

pub trait PhenoStore {
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError>;
    fn remove(&self, id: &Uuid) -> Result<(), PhenoStoreError>;
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<(), PhenoStoreError>;
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError>;
}
