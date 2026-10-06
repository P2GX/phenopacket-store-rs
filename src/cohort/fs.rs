use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, ErrorKind as IoErrorKind, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use uuid::Uuid;

use super::codec::CohortCodec;
use super::{Cohort, CohortManager, CohortManagerError};

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
    fn add(&self, cohort: &Cohort) -> Result<Uuid, CohortManagerError> {
        let id = Uuid::new_v4();
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

    fn get(&self, id: &Uuid) -> Result<Option<Cohort>, CohortManagerError> {
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

    fn update(&self, id: &Uuid, cohort: &Cohort) -> Result<bool, CohortManagerError> {
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

    fn remove(&self, id: &Uuid) -> Result<bool, CohortManagerError> {
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

    /// iterate over tuples of ([`Uuid`], [`Cohort`])
    ///
    /// skips invalid entries
    ///
    /// # Errors
    ///  - if the directory cannot be read for some reason
    fn iter_cohorts(&self) -> Result<impl Iterator<Item = (Uuid, Cohort)>, CohortManagerError> {
        Ok(self
            .cohorts_dir
            .as_ref()
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let id = Self::get_cohort_id(&entry.path()).ok()?;
                let cohort = self.get(&id).ok()??;
                Some((id, cohort))
            }))
    }
}

impl<P, C> FileCohortManager<P, C>
where
    P: AsRef<Path>,
{
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

    /// extracts the [`Uuid`] from a [`Cohort`] path.
    fn get_cohort_id(path: &Path) -> Result<Uuid, uuid::Error> {
        let id_str = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        Uuid::from_str(id_str)
    }
}

impl<P, C> FileCohortManager<P, C>
where
    P: AsRef<Path>,
    C: CohortCodec,
{
    /// returns the path for a cohort file based on its id
    fn get_cohort_path(&self, id: &Uuid) -> PathBuf {
        self.cohorts_dir
            .as_ref()
            .join(id.to_string())
            .with_added_extension(self.codec.ext())
    }
}

//
// ERRORS
//
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

//
// TESTS
//
#[cfg(test)]
mod test_cohort_manager {

    use super::super::codec::JsonCohortCodec;
    use super::super::{Cohort, CohortManager, CohortManagerError};
    use super::FileCohortManager;
    use std::assert_matches;
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::path::Path;
    use tempfile;
    use uuid::Uuid;

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
