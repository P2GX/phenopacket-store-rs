use std::{fs, io::{BufRead, BufWriter, Error as IoError, ErrorKind as IoErrorKind}, path::{Path, PathBuf}};
use serde::{Serialize, Deserialize};

use serde_json::error::Category;
use uuid::Uuid;
use crate::{PhenoStoreError, core::PhenoStore};
// use phenopackets::schema::v2::Cohort;

//
// ERRORS
//
/// public error type to report issues to the user of this crate
#[derive(Debug)]
#[non_exhaustive]
pub enum PhenoCohortError { //TODO will we acutally need this or does PhenoStoreError cover everything we need here?
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
            err => write!(f, "{err}")
        }
    }
}

impl std::error::Error for PhenoCohortError {}

impl From<std::io::Error> for PhenoCohortError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::NotFound => Self::NotFound,
            _ => Self::Io(value)
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
    // fn add_phenopacket_to_cohort(&self, pp_id: &PhenoId, cohort_id: &CohortId) -> Result<bool, PhenoCohortError>;
    fn update(&self, id: &CohortId, cohort: &Cohort) -> Result<bool, PhenoCohortError>;
    fn remove(&self, id: &CohortId) -> Result<bool, PhenoCohortError>;
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
        Cohort { members: Vec::new() }
    }
    
    // fn read<R: BufRead>(r: R) -> Result<Self, PhenoCohortError> {
    //     // let file = fs::File::open(path).map_err(|_e| PhenoCohortError::NotFound)?;  //TODO could also fail with permissionDenied?
    //     // let rdr = std::io::BufReader::new(file);
    //     let cohort = serde_json::from_reader(r).map_err(|_e| PhenoCohortError::InvalidData)?;
    //     Ok(cohort)
    // }

    // fn write(&self, path: &PathBuf) -> Result<(), PhenoCohortError> {
    //     let file  = fs::File::open(path)?;
    //     let wrt = BufWriter::new(file);
    //     let _ = serde_json::to_writer(wrt, self).map_err(|error| {
    //         // cleanup in case of failure
    //         let _ = fs::remove_file(path);
    //         PhenoCohortError::from(error)
    //     });
    //     Ok(())
    // }

    /// add a Phenopacket Id to the cohort at `path`
    /// TODO move this logic to the manager
    pub fn add(path: &PathBuf, id: &PhenoId) -> Result<(), PhenoCohortError> {
        let mut cohort = Cohort::read(path)?;
        cohort.members.push(*id);
        cohort.write(path)?;
        Ok(())
    }

}


pub struct FileCohortManager<P> {
    cohorts_dir: P,
}

impl<P: AsRef<Path>> CohortManager for FileCohortManager<P>
{
    fn new_cohort(&self) -> Result<CohortId, PhenoCohortError> {
        let id = CohortId::new_v4();
        let path = self.get_cohort_path(&id);
        let _ = fs::File::create(&path).expect("expect to have permission to write in cohort dir, since we created it.");
        Ok(id)
    }
    
    fn get(&self, id: &CohortId) -> Result<Option<Cohort>, PhenoCohortError> {
        let path = self.get_cohort_path(id);
        match Cohort::read(&path) {
            Ok(c) => Ok(Some(c)),
            Err(PhenoCohortError::NotFound) => Ok(None),
            Err(e) => Err(e)
        }
    }
    
    // fn add_phenopacket_to_cohort(&self, pp_id: &PhenoId, cohort_id: &CohortId) -> Result<bool, PhenoCohortError> {
    //     //do cohort and phenopacket exist?
    //     if self.store.get(pp_id).map_err(PhenoCohortError::Store).unwrap().is_none()|| self.get(cohort_id)?.is_none() { //TODO is there a way to beautify this? is unwrap legit here?
    //         return Err(PhenoCohortError::InvalidInput)
    //     }
    //     let cohort_path = self.get_cohort_path(&cohort_id);
    //     let cohort = Cohort::read(&cohort_path)?;
    //     cohort.
    // }
    
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
            }
        }
    }
    
}

impl<P: AsRef<Path>> FileCohortManager<P>
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
    pub fn new(dir: PathBuf) -> Result<Self, PhenoCohortError> {
        if !dir.exists() {
            if !dir.is_dir() {
                //TODO return configuration error
            }
        } else {
            fs::create_dir_all(&dir).map_err(PhenoCohortError::Io)?;
        }
        Ok(FileCohortManager { cohorts_dir: dir })
    }

    /// returns the path for a cohort file based on its id
    fn get_cohort_path(&self, id: &CohortId) -> PathBuf {
        self.cohorts_dir.join(format!("{id}.json"))
    }
}

//
// TESTS
//
#[cfg(test)]
mod test_cohort {
    use crate::fs::testutils;
    use super::*;

    
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
        let fps = testutils::example_store_empty().unwrap(); //FilePhenoStore::new("data").expect("example dir `data` should exist and be readable");
        let fcm = FileCohortManager::new(std::path::PathBuf::from("data"), fps).expect("initiation with existing empty dirs should work");

        let id: Uuid = "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().expect("valid UUID");
        let cohort_path = fcm.get_cohort_path(&id);

        assert_eq!(cohort_path.to_str().expect("valid string"), "data/67e55044-10b1-426f-9247-bb680e5fe0c8.json")

    }
}