use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Error as IoError, ErrorKind as IoErrorKind, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{PhenoStoreError, core::PhenoStore};
use serde_json::error::Category;
use uuid::Uuid;
// use phenopackets::schema::v2::Cohort;

//
// ERRORS
//
/// public error type to report issues to the user of this crate
#[derive(Debug)]
#[non_exhaustive]
pub enum PhenoCohortError {
    AlreadyExists,
    NotFound,
    InvalidData,
    InvalidInput,
    PermissionDenied,

    Io(std::io::Error),
    Store(PhenoStoreError),
}

impl std::fmt::Display for PhenoCohortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            err => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for PhenoCohortError {}

impl From<std::io::Error> for PhenoCohortError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::NotFound => Self::NotFound,
            _ => Self::Io(value),
        }
    }
}

impl From<serde_json::Error> for PhenoCohortError {
    fn from(value: serde_json::Error) -> Self {
        match value.classify() {
            Category::Data => Self::InvalidData,
            Category::Syntax => Self::InvalidInput,
            Category::Io => Self::Io(value.into()),
            Category::Eof => Self::Io(value.into()),
        }
    }
}

//
// COHORT MANAGER
//
pub trait CohortManager {
    fn new_cohort(&self) -> Result<CohortId, PhenoCohortError>;
    fn get(&self, id: &CohortId) -> Result<Option<Cohort>, PhenoCohortError>;
    fn update(&self, id: &CohortId, cohort: &Cohort) -> Result<bool, PhenoCohortError>;
    fn remove(&self, id: &CohortId) -> Result<bool, PhenoCohortError>;
    fn iter_cohorts(&self) -> impl Iterator<Item = (CohortId, Cohort)>;
}

//
// COHORT
//
type CohortId = Uuid;
type PhenoId = Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Cohort {
    pub members: Vec<PhenoId>,
}

impl Cohort {
    fn new() -> Cohort {
        Cohort {
            members: Vec::new(),
        }
    }

    // add a Phenopacket Id to the cohort at `path`
    // TODO move this logic to the manager
    // pub fn add(path: &PathBuf, id: &PhenoId) -> Result<(), PhenoCohortError> {
    //     let mut cohort = Cohort::read(path)?;
    //     cohort.members.push(*id);
    //     cohort.write(path)?;
    //     Ok(())
    // }
}

pub trait CohortCodec {
    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, String>;
    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), String>;
}

pub struct JsonCohortCodec;

impl CohortCodec for JsonCohortCodec {
    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, String> {
        let c: Cohort = serde_json::from_reader(r).expect("Error is handled");
        Ok(c)
    }

    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), String> {
        serde_json::to_writer(w, c).expect("todo");
        Ok(())
    }
}

pub struct FileCohortManager<P, C> {
    cohorts_dir: P,
    codec: C,
}

impl<P: AsRef<Path>, C: CohortCodec> CohortManager for FileCohortManager<P, C> {
    fn new_cohort(&self) -> Result<CohortId, PhenoCohortError> {
        let id = CohortId::new_v4();
        let path = self.get_cohort_path(&id);
        let _ = fs::File::create(&path)
            .expect("expect to have permission to write in cohort dir, since we created it.");
        Ok(id)
    }

    fn get(&self, id: &CohortId) -> Result<Option<Cohort>, PhenoCohortError> {
        let path = self.get_cohort_path(id);
        if let Ok(file) = File::open(path) {
            let mut r = BufReader::new(file);
            let c = self
                .codec
                .read(&mut r)
                .map_err(|e| PhenoCohortError::InvalidData)?;
            Ok(Some(c))
        } else {
            todo!()
        }
    }

    fn update(&self, id: &CohortId, cohort: &Cohort) -> Result<bool, PhenoCohortError> {
        todo!()
    }

    fn remove(&self, id: &CohortId) -> Result<bool, PhenoCohortError> {
        let path = self.get_cohort_path(&id);
        match fs::remove_file(&path) {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                IoErrorKind::NotFound => Ok(false),
                IoErrorKind::IsADirectory => Err(PhenoCohortError::InvalidData), // fs might be corrupted by external instance
                IoErrorKind::PermissionDenied => Err(PhenoCohortError::PermissionDenied), //TODO do we really want to propagate or do we want to assume we can write here and panic if not, since we created the parent dir?
                _ => Err(PhenoCohortError::Io(e)),
            },
        }
    }

    /// iterate over Cohorts as tuples of ([`Uuid`], [`Cohort`])
    fn iter_cohorts(&self) -> impl Iterator<Item = (CohortId, Cohort)> {
        self.cohorts_dir.as_ref().read_dir().unwrap().map(|entry| {
            // crop id from path /some/path/ID.ext
            let filename = entry.unwrap().file_name();
            let id_str = Path::new(&filename)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap();

            let id = CohortId::from_str(&id_str).unwrap();
            let cohort = self.get(&id).unwrap().unwrap();
            (id, cohort)
        })
    }
}

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
    pub fn new(cohorts_dir: P, codec: C) -> Result<Self, PhenoCohortError> {
        if !cohorts_dir.as_ref().exists() {
            if !cohorts_dir.as_ref().is_dir() {
                //TODO return configuration error
            }
        } else {
            fs::create_dir_all(&cohorts_dir).map_err(PhenoCohortError::Io)?;
        }
        Ok(FileCohortManager { cohorts_dir, codec })
    }

    /// returns the path for a cohort file based on its id
    fn get_cohort_path(&self, id: &CohortId) -> PathBuf {
        self.cohorts_dir.as_ref().join(format!("{id}.json"))
    }
}

//
// TESTS
//
#[cfg(test)]
mod test_cohort {
    use super::*;
    // use crate::fs::testutils;

    #[test]
    // fn test_new() {
    //     // setup
    //     let fps = testutils::example_store_empty().unwrap(); //FilePhenoStore::new("data").expect("example dir `data` should exist and be readable");
    //     let tmp = tempfile::tempdir().expect("should be possible to create temporary folder");
    //     let fcm = FileCohortManager::new(tmp.into(), fps).expect("initiation with existing empty dirs should work");

    //     assert!(fcm.dir.exists(), "directory should exist after initiation");
    // }

    fn test_cohort_path() {
        // setup
        let fcm = FileCohortManager::new(std::path::PathBuf::from("data"), JsonCohortCodec)
            .expect("initiation with existing empty dirs should work");

        let id: Uuid = "67e55044-10b1-426f-9247-bb680e5fe0c8"
            .parse()
            .expect("valid UUID");
        let cohort_path = fcm.get_cohort_path(&id);

        assert_eq!(
            cohort_path.to_str().expect("valid string"),
            "data/67e55044-10b1-426f-9247-bb680e5fe0c8.json"
        )
    }
}
