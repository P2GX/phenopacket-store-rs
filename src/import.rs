//! Import phenopackets and cohorts from a phenopacket-store release into a PhenoStore and CohortStore.

use std::{
    collections::HashMap, fs::File, io::Read, path::{Path, PathBuf},
};

use phenopackets::schema::v2::Phenopacket;
use prost::Message;
use uuid::Uuid;

use crate::{
    cohort::{Cohort, CohortStore, CohortStoreError}, phenopacket::{PhenoStore, PhenoStoreError},
};

type Item = Result<(String, Phenopacket), ItemError>;

#[derive(Debug)]
pub struct ItemError {
    pub source: String,
    pub kind: ItemErrorKind,
}

#[derive(Debug)]
pub enum ItemErrorKind {
    Read,
    Parse,
    /// folder structure does not match /release/cohort/phenopacket.json
    WrongLayout,
}

#[derive(Debug)]
/// reasons the import can fail.
pub enum ImportError {
    /// unexpected issue trying to write to CohortStore
    CohortStore(CohortStoreError),
    /// upexpected issue trying to write to PhenoStore
    PhenoStore(PhenoStoreError),
    /// could not read the archive
    Zip(zip::result::ZipError),
    /// the given path is invalid or unsupported
    InvalidInput,
    /// the release folder had an issue, for instance it might not match the expected layout
    InvalidData,
}

#[derive(Debug, Default)]
/// A successful import might not have been successful for all items. This report gives insight into which items succeeded an which failed.
pub struct ImportReport {
    /// successfully imported items as tuples (CohortStore ID, PhenoStore ID).
    pub imported: Vec<(Uuid, Uuid)>,
    /// items that failed to import.
    pub failed: Vec<ItemError>,
}

pub fn import_from_phenopacket_store<P, C, S>(
    from_path: P,
    to_cohortstore: C,
    to_phenostore: S,
) -> Result<ImportReport, ImportError>
where
    P: AsRef<Path>,
    C: CohortStore,
    S: PhenoStore,
{
    let from_path = from_path.as_ref();

    let items: Box<dyn Iterator<Item=Item>> = if from_path.is_dir() {
        Box::new(from_dir(&from_path)?)
    } else if from_path.extension().is_some_and(|ext| ext == "zip") {
        Box::new(from_zip(&from_path)?)
    } else {
        return Err(ImportError::InvalidInput);
    };
    let mut cohort_name2id: HashMap<String, Uuid> = HashMap::new();
    let mut report = ImportReport::default();

    for (cohort_name, pp) in items? {
        let cohort_id = cohort_name2id.get(&cohort_name).or_else({
            let cohort_id = to_cohortstore.add(&Cohort{ id: cohort_name, Default::default() })?;
            cohort_name2id.insert(cohort_name, cohort_id);
            cohort_id
        })?;
        let pp_id = to_phenostore.add(&pp)?;
        let res = add_phenopacket_to_cohort(pp_id, cohort_id, to_cohortstore);

        match res {
            Ok(_) => report.imported.push((cohort_id, pp_id)),
            Err(e) => report.failed.push(e),    //e is currently an ImportError not an ItemError, but would we even catch all itemErrors here or did we already filter them out by now?
        }
    }
}

fn read_phenopacket_store<P>(path: P)
where
    P: AsRef<Path>,
{
    // does path exist
    if !path.as_ref().exists() {
        todo!("throw error path does not exist.")
    }
    if path.as_ref().extension().unwrap() == "zip" {
        // unzip
        todo!();
    }
    // read dir
}

/// create an iterator over [`Item`]s from an unzipped directory `dir`
fn from_dir(dir: &Path) -> Result<impl Iterator<Item = Item>, ImportError> {
    let cohorts = dir.read_dir().map_err(|e| ImportError::InvalidInput)?;
    let read_err = |p: &Path| ItemError {
        source: p.display().to_string(),
        kind: ItemErrorKind::Read,
    };

    // collect all /COHORT/PHENOPACKET.json files
    let mut json_files: Vec<Result<(String, PathBuf), ItemError>> = Vec::new();
    for cohort in cohorts {
        let Ok(cohort) = cohort else {
            json_files.push(Err(read_err(dir)));
            continue;
        };
        if !cohort.path().is_dir() {
            continue;
        }
        let cohort_name = cohort.file_name().to_string_lossy().into_owned();

        let entries = match cohort.path().read_dir() {
            Ok(entries) => entries,
            Err(_) => {
                json_files.push(Err(read_err(&cohort.path())));
                continue;
            }
        };
        for entry in entries {
            match entry {
                Ok(e) if e.path().extension().is_some_and(|ext| ext == "json") => {
                    json_files.push(Ok((cohort_name.clone(), e.path())));
                }
                Ok(_) => {} // skip non-json files
                Err(_) => json_files.push(Err(read_err(&cohort.path()))),
            }
        }
    }

    Ok(json_files
        .into_iter()
        .map(|x| x.and_then(|(cohort, pp_path)| read_phenopacket(&pp_path).map(|pp| (cohort, pp)))))
}

/// read a phenopacket from the specified path
fn read_phenopacket(path: &Path) -> Result<Phenopacket, ItemError> {
    let path_str: String = path
        .to_str()
        .expect("path should contain only legal characters.")
        .to_owned();
    let mut file = match File::open(path) {
        Ok(it) => it,
        Err(_) => {
            return Err(ItemError {
                source: path_str,
                kind: ItemErrorKind::Read,
            });
        }
    };
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|_| ItemError {
        source: path_str.clone(),
        kind: ItemErrorKind::Read,
    })?;
    let pp = Phenopacket::decode(&buf[..]).map_err(|_| ItemError {
        source: path_str.clone(),
        kind: ItemErrorKind::Parse,
    })?;
    Ok(pp)
}

/// create an [`Iterator`] from a zipped Phenopacket-store archive.
fn from_zip(path: &Path) -> Result<impl Iterator<Item = Item>, ImportError> {
    todo!()
}

fn add_phenopacket_to_cohort(phenopacket_id: Uuid, cohort_id: Uuid, cohortstore: impl CohortStore) -> Result<(), ImportError> {
    if let Some(mut cohort) = cohortstore.get(&cohort_id).map_err(|e| ImportError::CohortStore(e))? {
        cohort.members.push(phenopacket_id);
        cohortstore.update(&cohort_id, &cohort).map_err(|e| ImportError::CohortStore(e));
    } 
    Ok(())
}