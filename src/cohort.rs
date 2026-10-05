use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, ErrorKind as IoErrorKind, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use serde_json::error::Category;
use uuid::Uuid;

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

/// CohortManager implements a management backend for a collection of [`Cohort`] entries.
///
/// A new cohort can be added to the manager using [`CohortManager::add`], returning the [`Uuid`] of the newly created cohort.
/// A cohort can be retireved ([`CohortManager::get`]), updated or removed. We can also iterate over all the cohorts a CohortManager is managing using [`CohortManager::iter_cohorts`].
///
/// In case of non-normal execution, all methods return a [`CohortManagerError`].
pub trait CohortManager {
    /// add a [`Cohort`] to the [`CohortManager`] and return its [`Uuid`].
    fn add(&self, cohort: &Cohort) -> Result<CohortId, CohortManagerError>;

    /// Gets a [`Cohort`] based on its `id`.
    ///
    /// Returns `Ok(None)` if no Cohort was found with `id`.
    ///
    /// # Errors
    ///  - InvalidData: if cohort could not be read
    ///  - Io(io::ErrorKind): if some other io error occured, for example network issues
    fn get(&self, id: &CohortId) -> Result<Option<Cohort>, CohortManagerError>;

    /// write the `cohort` to be found at `id`
    ///
    /// # Returns
    ///  - Ok(true): successfully updated cohort data
    ///  - Ok(false): did not find `id`
    fn update(&self, id: &CohortId, cohort: &Cohort) -> Result<bool, CohortManagerError>;

    /// remove a [`Cohort`] from the [`CohortManager`], based on its `id`.
    ///
    /// # Returns
    ///  - Ok(true): successfully removed cohort
    ///  - Ok(false): cohort was not found and thus nothing removed
    fn remove(&self, id: &CohortId) -> Result<bool, CohortManagerError>;

    /// iterate over Cohorts as tuples of ([`Uuid`], [`Cohort`])
    fn iter_cohorts(&self) -> Result<impl Iterator<Item = (CohortId, Cohort)>, CohortManagerError>;
}

type CohortId = Uuid;
type PhenoId = Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Cohort {
    pub description: String,
    pub members: Vec<PhenoId>,
}

impl Cohort {
    fn new() -> Cohort {
        Cohort {
            description: String::new(),
            members: Vec::new(),
        }
    }
}

/// provides abilities to read/write a [`Cohort`] from/to the filesystem.
pub trait CohortCodec {
    type Error: std::error::Error + Send + Sync + 'static;

    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, Self::Error>;
    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), Self::Error>;

    /// returns the file extention for the cohort files, the codec uses
    fn ext(&self) -> &str;
}

/// A filesystem-based implementation of the [`CohortManager`].
pub struct FileCohortManager<P, C> {
    cohorts_dir: P,
    codec: C,
}

impl<P, C> CohortManager for FileCohortManager<P, C>
where
    P: AsRef<Path>,
    C: CohortCodec,
    C::Error: Into<CohortManagerError>,
{
    fn add(&self, cohort: &Cohort) -> Result<CohortId, CohortManagerError> {
        let id = CohortId::new_v4();
        let path = self.get_cohort_path(&id);
        let mut writer = BufWriter::new(fs::File::create(&path)?);

        // write the new (empty) cohort state to file
        let _ = self.codec.write(cohort, &mut writer).map_err(|e| {
            fs::remove_file(&path).unwrap();
            e.into()
        });
        writer.flush()?;
        Ok(id)
    }

    fn get(&self, id: &CohortId) -> Result<Option<Cohort>, CohortManagerError> {
        let path = self.get_cohort_path(id);
        match File::open(path) {
            Ok(file) => {
                let mut r = BufReader::new(file);
                let c = self
                    .codec
                    .read(&mut r)
                    .map_err(|_| CohortManagerError::InvalidData)?;
                Ok(Some(c))
            }
            Err(e) => match e.kind() {
                IoErrorKind::NotFound => Ok(None),
                _ => Err(CohortManagerError::Io(e)),
            },
        }
    }

    fn update(&self, id: &CohortId, cohort: &Cohort) -> Result<bool, CohortManagerError> {
        let cohort_file = self.get_cohort_path(id);
        if !cohort_file.exists() {
            return Ok(false);
        }
        let tmp_path = cohort_file.with_added_extension("tmp");
        let try_update = {
            let mut writer = BufWriter::new(File::create(&tmp_path)?);
            self.codec
                .write(cohort, &mut writer)
                .map_err(Into::into)
                .and_then(|()| writer.flush().map_err(Into::into))
        };

        match try_update {
            Ok(_) => {
                fs::rename(&tmp_path, &cohort_file)?;
                Ok(true)
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp_path);
                Err(e)
            }
        }
    }

    fn remove(&self, id: &CohortId) -> Result<bool, CohortManagerError> {
        let path = self.get_cohort_path(id);
        match fs::remove_file(&path) {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                IoErrorKind::NotFound => Ok(false),
                IoErrorKind::IsADirectory => Err(CohortManagerError::InvalidData), // fs might be corrupted by external instance
                _ => Err(CohortManagerError::Io(e)),
            },
        }
    }

    /// iterate over tuples of ([`CohortId`], [`Cohort`])
    ///
    /// skips invalid entries
    ///
    /// # Errors
    ///  - if the directory cannot be read for some reason
    fn iter_cohorts(&self) -> Result<impl Iterator<Item = (CohortId, Cohort)>, CohortManagerError> {
        Ok(self
            .cohorts_dir
            .as_ref()
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                // crop id from path
                let filename = entry.file_name();
                let id_str = Path::new(&filename).file_stem()?.to_str()?;
                let id = CohortId::from_str(id_str).ok()?;
                let cohort = self.get(&id).ok()??;
                Some((id, cohort))
            }))
    }
}

/// Represents the errors that can happen during [`FileCohortManager`] configuration.
#[derive(Debug)]
pub enum FileCohortManagerError {
    /// The provided path did not point to a directory.
    NotADirectory,
    /// Another IO-related error.
    Io(std::io::Error),
}

impl From<std::io::Error> for FileCohortManagerError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            IoErrorKind::NotADirectory => Self::NotADirectory,
            _ => Self::Io(value),
        }
    }
}

impl std::fmt::Display for FileCohortManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileCohortManagerError::NotADirectory => write!(f, "Not a directory"),
            FileCohortManagerError::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for FileCohortManagerError {}

impl<P: AsRef<Path>, C: CohortCodec> FileCohortManager<P, C> {
    /// Initiate a new file-based CohortManager.
    ///
    /// Will either open the given `dir` as [`FileCohortManager`], or attempt to create a directory at `dir` if it does not exist.
    ///
    /// # Errors
    ///
    /// - the given path exists, but is not a directory
    /// - the path does not exist and could not be created
    ///
    pub fn new(cohorts_dir: P, codec: C) -> Result<Self, FileCohortManagerError> {
        if !cohorts_dir.as_ref().exists() {
            if !cohorts_dir.as_ref().is_dir() {
                // path exists but is not a dir
                return Err(FileCohortManagerError::NotADirectory);
            }
        } else {
            fs::create_dir_all(&cohorts_dir)?;
        }
        Ok(FileCohortManager { cohorts_dir, codec })
    }

    /// returns the path for a cohort file based on its id
    fn get_cohort_path(&self, id: &CohortId) -> PathBuf {
        self.cohorts_dir
            .as_ref()
            .join(id.to_string())
            .with_added_extension(self.codec.ext())
    }
}

/// implements [`CohortCodec`] using [`serde_json`]. To be used in [`FileCohortManager`].
pub struct JsonCohortCodec;

impl CohortCodec for JsonCohortCodec {
    type Error = serde_json::Error;

    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, Self::Error> {
        let c: Cohort = serde_json::from_reader(r)?;
        Ok(c)
    }

    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), Self::Error> {
        serde_json::to_writer(w, c)?;
        Ok(())
    }

    fn ext(&self) -> &str {
        "json"
    }
}

impl From<serde_json::Error> for CohortManagerError {
    fn from(value: serde_json::Error) -> Self {
        match value.classify() {
            Category::Data | Category::Syntax | Category::Eof => Self::InvalidData,
            Category::Io => Self::Io(value.into()),
        }
    }
}

//
// TESTS
//
#[cfg(test)]
mod test_cohort_manager {
    use super::*;
    use std::assert_matches;
    use tempfile;

    #[test]
    fn test_new() {
        // setup
        let tmpdir = tempfile::TempDir::new().expect("creation of tempdir should work.");
        let _ = FileCohortManager::new(&tmpdir, JsonCohortCodec)
            .expect("initiation on temp dir and json codec to succeed.");
    }

    #[test]
    fn test_cohort_path() {
        // setup
        let dir = Path::new("data");
        let fcm = FileCohortManager::new(&dir, JsonCohortCodec)
            .expect("initiation on temp dir and json codec to succeed.");

        let id: Uuid = "67e55044-10b1-426f-9247-bb680e5fe0c8"
            .parse()
            .expect("valid UUID");
        let cohort_path = fcm.get_cohort_path(&id);

        assert_eq!(
            cohort_path.to_str().expect("valid string"),
            "data/67e55044-10b1-426f-9247-bb680e5fe0c8.json"
        )
    }

    #[test]
    fn test_add_get() -> Result<(), CohortManagerError> {
        // setup
        let tmpdir = tempfile::TempDir::new()?;
        let fcm = FileCohortManager::new(&tmpdir, JsonCohortCodec)
            .expect("initiation on temp dir and json codec to succeed.");

        //test add
        let mut c1 = Cohort::new();
        c1.description = String::from("cohort #1");
        let id1 = fcm.add(&c1).expect("adding a first cohort should work");
        let mut c2 = Cohort::new();
        c2.description = String::from("cohort #2");
        let id2 = fcm.add(&c2).expect("adding second cohort should work");

        // test get success
        let c1_get = fcm.get(&id1);
        assert_matches!(c1_get, Ok(Some(_)));
        let c2_get = fcm.get(&id2);
        assert_matches!(c2_get, Ok(Some(_)));
        let c1_get_again = fcm.get(&id1);
        assert_matches!(
            c1_get_again,
            Ok(Some(_)),
            "should be able to get the same cohort multiple times."
        );

        // test get fail
        let non_existent_id = Uuid::new_v4();
        let non_existent_get = fcm.get(&non_existent_id);
        assert_matches!(
            non_existent_get,
            Ok(None),
            "lookup of non existing id should return Ok(None)"
        );

        Ok(())
    }

    #[test]
    fn test_remove() -> Result<(), CohortManagerError> {
        // setup
        let tmpdir = tempfile::TempDir::new()?;
        let fcm = FileCohortManager::new(&tmpdir, JsonCohortCodec)
            .expect("initiation on temp dir and json codec to succeed.");
        let c1 = fcm.add(&Cohort::new())?;
        let c2 = fcm.add(&Cohort::new())?;
        let c3 = fcm.add(&Cohort::new())?;
        assert_matches!(fcm.get(&c1), Ok(Some(_)));
        assert_matches!(fcm.get(&c2), Ok(Some(_)));
        assert_matches!(fcm.get(&c3), Ok(Some(_)));

        let t_remove_existing = fcm.remove(&c2);
        assert_matches!(
            t_remove_existing,
            Ok(true),
            "failed to remove existing cohort"
        );
        assert_matches!(
            fcm.get(&c2),
            Ok(None),
            "removed cohort should not exist in CohortManager anymore"
        );

        let t_remove_nonexisting = fcm.remove(&c2);
        assert_matches!(t_remove_nonexisting, Ok(false));

        Ok(())
    }

    #[test]
    fn test_update() -> Result<(), CohortManagerError> {
        //setup
        let tmpdir = tempfile::TempDir::new()?;
        let fcm = FileCohortManager::new(&tmpdir, JsonCohortCodec)
            .expect("initiation on temp dir and json codec to succeed.");

        // test
        let id1 = fcm.add(&Cohort::new())?;
        let mut c2 = Cohort::new();
        c2.members.push(Uuid::new_v4());

        let t_sucess = fcm.update(&id1, &c2);
        assert_matches!(t_sucess, Ok(true), "update failed");
        let c2_fetched = fcm.get(&id1);
        assert_matches!(c2_fetched, Ok(Some(_)), "unable to get cohort after update");
        let c2_fetched = c2_fetched?.unwrap();
        assert!(
            c2_fetched.members.len() == 1,
            "update failed to change data"
        );
        let t_id_not_found = fcm.update(&Uuid::new_v4(), &c2);
        assert_matches!(
            t_id_not_found,
            Ok(false),
            "update with nonexistent id failed different than expected"
        );

        Ok(())
    }

    #[test]
    fn test_iter_cohorts() -> Result<(), CohortManagerError> {
        // setup
        let tmpdir = tempfile::TempDir::new()?;
        let fcm = FileCohortManager::new(&tmpdir, JsonCohortCodec)
            .expect("initiation on temp dir and json codec to succeed.");
        let c1 = fcm.add(&Cohort::new())?;
        let c2 = fcm.add(&Cohort::new())?;
        let c3 = fcm.add(&Cohort::new())?;
        let mut cs = vec![c1, c2, c3];

        let mut cs_test = Vec::new();

        for (id, _) in fcm
            .iter_cohorts()
            .expect("creation of first test iterator failed")
        {
            cs_test.push(id);
        }
        cs.sort();
        cs_test.sort();
        assert_eq!(
            cs, cs_test,
            "cohorts added and cohorts iterated over do not match."
        );

        // TEST corrupted file is skipped and does not panic the iterator
        let corrupted_id = Uuid::new_v4();
        let corruped_file = File::create_new(tmpdir.path().join(format!("{corrupted_id}.json")))?;
        let mut writer = BufWriter::new(corruped_file);
        let _ = write!(writer, "this file is not a valid cohort.json");
        let mut cs_test = Vec::new();

        for (id, _) in fcm
            .iter_cohorts()
            .expect("creation of second test iterator failed")
        {
            cs_test.push(id);
        }
        cs_test.sort();
        assert_eq!(
            cs, cs_test,
            "cohorts added and cohorts iterated over do not match."
        );
        assert!(
            !cs_test.contains(&corrupted_id),
            "iterator should skip corrupted files"
        );

        Ok(())
    }
}
