use std::error::Error;
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;

use crate::query::Query;

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

    /// Get the `phenopacket` stored under the `id`.
    ///
    /// Returns `Ok(None)` if no such phenopacket exists.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError>;

    /// Update the `phenopacket` stored under `id`. Returns `Ok(true)` if the phenopacket was updated
    /// or `Ok(false)` if no phenopacket was found for the `id`.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<bool, PhenoStoreError>;

    /// Remove the `phenopacket` from the store.
    ///
    /// Returns `Ok(true)` if the phenopacket was removed or `Ok(false)` if it was not found.
    ///
    /// # Errors
    ///
    /// Fails for reasons unrelated to normal backend functionality.
    fn remove(&self, id: &Uuid) -> Result<bool, PhenoStoreError>;

    /// Iterate over all phenopackets stored in this [`PhenoStore`].
    ///
    /// # Errors
    ///
    ///  - fails if no iterator could be created
    ///  - errors arising from a single phenopacket fail *silently* i.e. erronous entries are ignored.
    fn iter_phenopackets(&self) -> Result<impl Iterator<Item = Uuid>, PhenoStoreError>;

    /// Query the stored phenopackets.
    ///
    /// Returns the [`Uuid`]s of phenopackets, where the `query` ([`Query::test`]) evaulated to `true`
    ///
    /// The provided implementation fetches and checks *all* stored phenopackets. Storage backends that can filter natively should override this (for example an RDBMS).
    fn query(
        &self,
        query: &impl Query<Phenopacket>,
    ) -> Result<impl Iterator<Item = Uuid>, PhenoStoreError> {
        Ok(self.iter_phenopackets()?.filter(|id| {
            if let Ok(Some(pp)) = self.get(id) {
                query.test(&pp)
            } else {
                false
            }
        }))
    }
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

    fn iter_phenopackets(&self) -> Result<impl Iterator<Item = Uuid>, PhenoStoreError> {
        (*self).iter_phenopackets()
    }

    fn query(
        &self,
        query: &impl Query<Phenopacket>,
    ) -> Result<impl Iterator<Item = Uuid>, PhenoStoreError> {
        (*self).query(query)
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
