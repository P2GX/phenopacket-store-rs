use std::error::Error;
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;

pub trait PhenoStore {
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError>;
    fn remove(&self, id: &Uuid) -> Result<(), PhenoStoreError>;
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<(), PhenoStoreError>;
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError>;
}

impl<T> PhenoStore for &T
where
    T: PhenoStore + ?Sized,
{
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError> {
        (*self).add(phenopacket)
    }

    fn remove(&self, id: &Uuid) -> Result<(), PhenoStoreError> {
        (*self).remove(id)
    }

    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<(), PhenoStoreError> {
        (*self).update(id, phenopacket)
    }

    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError> {
        (*self).get(id)
    }
}

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

#[derive(Debug, Clone)]
pub enum IoErrorKind {
    PermissionDenied,
    ReadOnly,
    CapacityExceeded,
    Timeout,
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

impl std::fmt::Display for PhenoStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PhenoStoreError {}

#[cfg(test)]
mod test_pheno_store_error {
    use parameterized::parameterized;

    use super::{IoErrorKind, PhenoStoreError};

    #[parameterized(
        val={
            (PhenoStoreError::AlreadyExists, "AlreadyExists"),
            (PhenoStoreError::Io(IoErrorKind::ReadOnly), "Io(ReadOnly)"),
            (PhenoStoreError::Other(Box::new(PhenoStoreError::NotFound)), "Other(NotFound)"),
        },
    )]
    fn test_display(val: (PhenoStoreError, &'static str)) {
        let (error, expected) = val;
        let msg = error.to_string();
        assert_eq!(&msg, expected);
    }
}
