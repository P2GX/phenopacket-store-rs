//! Import phenopackets and cohorts from a [phenopacket-store release](https://github.com/monarch-initiative/phenopacket-store/releases)  into a PhenoStore and CohortStore.

use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use phenopackets::schema::v2::Phenopacket;
use uuid::Uuid;
use zip::{ZipArchive, result::ZipError};

use crate::{
    cohort::{Cohort, CohortStore, CohortStoreError},
    phenopacket::{PhenoStore, PhenoStoreError},
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
    Zip(ZipError),
    /// the given path is invalid or unsupported
    InvalidInput,
    /// the release folder had an issue, for instance it might not match the expected layout
    InvalidData,
}

impl From<CohortStoreError> for ImportError {
    fn from(error: CohortStoreError) -> Self {
        Self::CohortStore(error)
    }
}

impl From<PhenoStoreError> for ImportError {
    fn from(error: PhenoStoreError) -> Self {
        Self::PhenoStore(error)
    }
}

impl From<ZipError> for ImportError {
    fn from(error: ZipError) -> Self {
        Self::Zip(error)
    }
}

#[derive(Debug, Default)]
/// A successful import might not have been successful for all items. This report gives insight into which items succeeded an which failed.
pub struct ImportReport {
    /// successfully imported items as tuples (CohortStore ID, PhenoStore ID).
    pub imported: Vec<(Uuid, Uuid)>,
    /// items that failed to import.
    pub failed: Vec<ItemError>,
}

/// Import data from a [phenopacket-store release](https://github.com/monarch-initiative/phenopacket-store/releases) into a [`CohortStore`] and  [`PhenoStore`].
///
/// The data must follow the folder structure RELEASE/COHORT/PHENOPACKET.json
/// The release folder can be zipped or unzipped.
///
/// # Returns
/// - a report stating which of the found json files were successfully imported and which failed, along with their error.
pub fn import_from_phenopacket_store<P, C, S>(
    from_path: P,
    to_cohortstore: &C,
    to_phenostore: &S,
) -> Result<ImportReport, ImportError>
where
    P: AsRef<Path>,
    C: CohortStore,
    S: PhenoStore,
{
    let from_path = from_path.as_ref();

    let items: Box<dyn Iterator<Item = Item>> = if from_path.is_dir() {
        Box::new(from_dir(from_path)?)
    } else if from_path.extension().is_some_and(|ext| ext == "zip") {
        Box::new(from_zip(from_path)?)
    } else {
        return Err(ImportError::InvalidInput);
    };
    let mut report = ImportReport::default();

    // collect members per cohort first, so every cohort is written exactly once
    let mut cohorts: HashMap<String, Vec<Uuid>> = HashMap::new();
    for item in items {
        match item {
            Ok((cohort_name, pp)) => {
                let pp_id = to_phenostore.add(&pp)?;
                cohorts.entry(cohort_name).or_default().push(pp_id);
            }
            Err(e) => report.failed.push(e),
        }
    }

    for (cohort_name, members) in cohorts {
        let cohort = Cohort {
            id: cohort_name,
            members,
            ..Default::default()
        };
        let cohort_id = to_cohortstore.add(&cohort)?;
        report
            .imported
            .extend(cohort.members.iter().map(|&pp_id| (cohort_id, pp_id)));
    }
    Ok(report)
}

/// create an iterator over [`Item`]s from an unzipped directory `dir`
fn from_dir(dir: &Path) -> Result<impl Iterator<Item = Item>, ImportError> {
    let cohorts = dir.read_dir().map_err(|_| ImportError::InvalidInput)?;
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

/// read a JSON phenopacket from the specified path
fn read_phenopacket(path: &Path) -> Result<Phenopacket, ItemError> {
    let fail = |kind| ItemError {
        source: path.display().to_string(),
        kind,
    };
    let file = File::open(path).map_err(|_| fail(ItemErrorKind::Read))?;
    serde_json::from_reader(BufReader::new(file)).map_err(|_| fail(ItemErrorKind::Parse))
}

/// create an [`Iterator`] from a zipped Phenopacket-store archive.
fn from_zip(path: &Path) -> Result<impl Iterator<Item = Item>, ImportError> {
    let file = File::open(path).map_err(|_| ImportError::InvalidInput)?;
    let mut archive = ZipArchive::new(file)?;

    Ok((0..archive.len()).filter_map(move |i| {
        let entry = match archive.by_index(i) {
            Ok(entry) => entry,
            Err(_) => {
                return Some(Err(ItemError {
                    source: format!("{}#{i}", path.display()),
                    kind: ItemErrorKind::Read,
                }));
            }
        };
        let Ok(source) = entry.name().map(|name| name.into_owned()) else {
            return Some(Err(ItemError {
                source: format!("{}#{i}", path.display()),
                kind: ItemErrorKind::Read,
            }));
        };
        if !entry.is_file() || !source.ends_with(".json") {
            return None;
        }

        // release/COHORT/PHENOPACKET.json -> COHORT
        let Some(cohort) = Path::new(&source).parent().and_then(Path::file_name) else {
            return Some(Err(ItemError {
                source: source.clone(),
                kind: ItemErrorKind::WrongLayout,
            }));
        };
        let cohort = cohort.to_string_lossy().into_owned();

        Some(
            serde_json::from_reader(BufReader::new(entry))
                .map(|pp| (cohort, pp))
                .map_err(|_| ItemError {
                    source: source.clone(),
                    kind: ItemErrorKind::Parse,
                }),
        )
    }))
}

#[cfg(test)]
mod test_import {
    use std::collections::HashSet;
    use std::path::Path;

    use crate::import::{ImportError, Item, from_dir, from_zip};

    // test data properties
    //-------------------------------------------------------------------------
    // total number of phenopackets
    const N_PHENOPACKETS: usize = 13;
    // total number of invalid files
    const N_INVALID_JSON: usize = 1;
    // total number of json files
    const N_JSON: usize = N_PHENOPACKETS + N_INVALID_JSON;
    // total number of cohorts
    const N_COHORTS: usize = 3;
    //-------------------------------------------------------------------------

    #[test]
    fn test_from_dir() {
        assert_phenopacket_store_iterator(from_dir(Path::new("data/0.1.27_excerpt")));
    }

    #[test]
    fn test_from_zip() {
        assert_phenopacket_store_iterator(from_zip(Path::new(
            "data/phenopacket-store_excerpt.zip",
        )));
    }

    fn assert_phenopacket_store_iterator(it: Result<impl Iterator<Item = Item>, ImportError>) {
        let items: Vec<_> = it.expect("iterator creation failed.").collect();
        assert_eq!(
            items.len(),
            N_JSON,
            "example should contain expected number of json files (valid and invalid)."
        );
        assert_eq!(
            items.iter().filter(|x| x.is_ok()).count(),
            N_PHENOPACKETS,
            "valid example phenopackets should be readable without issues."
        );
        assert_eq!(
            items.iter().filter(|x| x.is_err()).count(),
            N_INVALID_JSON,
            "invalid example phenopackets should be caught as items."
        );

        let cohort_names: HashSet<&str> = items
            .iter()
            .flatten()
            .map(|(cohort, _)| cohort.as_str())
            .collect();
        assert_eq!(
            cohort_names.len(),
            N_COHORTS,
            "example should contain expected number of distinct cohorts."
        );
    }
}
