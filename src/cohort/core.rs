use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// CohortManager implements a management backend for a collection of [`Cohort`] entries.
///
/// A new cohort can be added to the manager using [`CohortManager::add`], returning the [`Uuid`] of the newly created cohort.
/// A cohort can be retireved ([`CohortManager::get`]), updated or removed. We can also iterate over all the cohorts a CohortManager is managing using [`CohortManager::iter_cohorts`].
///
/// In case of non-normal execution, all methods return a [`CohortManagerError`].
pub trait CohortManager {
    /// add a [`Cohort`] to the [`CohortManager`] and return its [`Uuid`].
    fn add(&self, cohort: &Cohort) -> Result<Uuid, CohortManagerError>;

    /// Gets a [`Cohort`] based on its `id`.
    ///
    /// Returns `Ok(None)` if no Cohort was found with `id`.
    ///
    /// # Errors
    ///  - InvalidData: if cohort could not be read
    ///  - Io(io::ErrorKind): if some other io error occured, for example network issues
    fn get(&self, id: &Uuid) -> Result<Option<Cohort>, CohortManagerError>;

    /// write the `cohort` to be found at `id`
    ///
    /// # Returns
    ///  - Ok(true): successfully updated cohort data
    ///  - Ok(false): did not find `id`
    fn update(&self, id: &Uuid, cohort: &Cohort) -> Result<bool, CohortManagerError>;

    /// remove a [`Cohort`] from the [`CohortManager`], based on its `id`.
    ///
    /// # Returns
    ///  - Ok(true): successfully removed cohort
    ///  - Ok(false): cohort was not found and thus nothing removed
    fn remove(&self, id: &Uuid) -> Result<bool, CohortManagerError>;

    /// iterate over Cohorts as tuples of ([`Uuid`], [`Cohort`])
    ///
    /// # Errors
    ///  - fails if no iterator could be created
    ///  - errors arising from a single cohort fail *silently* i.e. erronous entries are ignored.
    fn iter_cohorts(&self) -> Result<impl Iterator<Item = (Uuid, Cohort)>, CohortManagerError>;
}

/// The reasons why the [`CohortManager`] can fail.
#[derive(Debug)]
#[non_exhaustive]
pub enum CohortManagerError {
    /// The data that should have been valid was found invalid.
    /// For instance, a data structure was truncated, or contained a number where a string was expected.
    InvalidData,

    /// An IO-related issue that is unrelated to the main functionality of the manager.
    /// This can be a connection error, reaching a storage quota, lack of priviledges for the action, etc.
    Io(std::io::Error),
}

impl std::fmt::Display for CohortManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => write!(f, "Encountered invalid data, verify data integrity."),
            Self::Io(err) => write!(f, "Ran into an issue with the CohortManager backend: {err}"),
        }
    }
}

impl std::error::Error for CohortManagerError {}

impl From<std::io::Error> for CohortManagerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Cohort {
    pub description: String,
    pub members: Vec<Uuid>,
}

impl Cohort {
    pub fn new() -> Cohort {
        Cohort {
            description: String::new(),
            members: Vec::new(),
        }
    }
}
