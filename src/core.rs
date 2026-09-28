use std::error::Error;
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;

/// PhenoStore implements a storage backend for phenopackets.
///
/// Phenopackets can be stored with [`PhenoStore::add`] method and a UUID is provided upon successful storage
/// to allow retrieving the phenopacket with the [`PhenoStore::get`] method.
/// A phenopacket can be updated and removed.
///
/// All methods return an error in case of non-normal execution, a reason unrelated to normal function of the backend
/// (e.g. storage capacity exceeded, network connectivity issues).
pub trait PhenoStore {
    /// Store the `phenopacket` and return a [`Uuid`] for the later access.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError>;

    /// Remove the `phenopacket` from the store.
    ///
    /// Returns `Ok(true)` if the phenopacket was removed or `Ok(false)` if it was not found.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn remove(&self, id: &Uuid) -> Result<bool, PhenoStoreError>;

    /// Update the `phenopacket` stored under `id`. Returns `Ok(true)` if the phenopacket was updated and `Ok(false)` otherwise.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<bool, PhenoStoreError>;

    /// Get the `phenopacket` stored under the `id`.
    ///
    /// Returns `Ok(None)` if no such phenopacket exists.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError>;
}

impl<T> PhenoStore for &T
where
    T: PhenoStore + ?Sized,
{
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError> {
        (*self).add(phenopacket)
    }

    fn remove(&self, id: &Uuid) -> Result<bool, PhenoStoreError> {
        (*self).remove(id)
    }

    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<bool, PhenoStoreError> {
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
    // NotFound,
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
            (PhenoStoreError::Other(Box::new(PhenoStoreError::AlreadyExists)), "Other(AlreadyExists)"),
        },
    )]
    fn test_display(val: (PhenoStoreError, &'static str)) {
        let (error, expected) = val;
        let msg = error.to_string();
        assert_eq!(&msg, expected);
    }
}
