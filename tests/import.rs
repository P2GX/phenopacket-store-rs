//! test importing a phenopacket-store release into a [`CohortStore`] and [`PhenoStore`]

use std::path::Path;

use parameterized::parameterized;
use ppktstore::{
    cohort::{CohortStore, codec::JsonCohortCodec, fs::FileCohortStore},
    import::import_from_phenopacket_store,
    phenopacket::{PhenoStore, fs::FilePhenoStore},
};

// test data properties
/// total number of phenopackets
const N_PHENOPACKETS: usize = 13;
// total number of invalid files
const N_INVALID_JSON: usize = 1;
/// total number of cohorts
const N_COHORTS: usize = 3;

#[parameterized(
    import_path = {
        "data/0.1.27_excerpt",
        "data/phenopacket-store_excerpt.zip",
    }
)]
fn test_import_to_stores(import_path: &str) {
    let ps_dir = tempfile::tempdir().expect("should be able to create a temporary directory.");
    let cs_dir = tempfile::tempdir().expect("should be able to create a temporary directory.");
    let ps = FilePhenoStore::new(ps_dir).expect("should be able to create a phenostore.");
    let cs = FileCohortStore::new(cs_dir, JsonCohortCodec)
        .expect("should be able to create a cohortstore.");

    let report = import_from_phenopacket_store(Path::new(import_path), &cs, &ps)
        .expect("import with valid data and stores should not fail.");

    assert_eq!(
        report.failed.len(),
        N_INVALID_JSON,
        "invalid phenopackets should fail to import."
    );
    assert_eq!(
        report.imported.len(),
        N_PHENOPACKETS,
        "reported imports differ from expected imports."
    );
    assert_eq!(
        ps.iter_phenopackets()
            .expect("phenopacket iterator creation should work.")
            .count(),
        N_PHENOPACKETS,
        "stored phenopackets differ from expected imports."
    );
    assert_eq!(
        cs.iter_cohorts()
            .expect("cohort iterator creation should work.")
            .count(),
        N_COHORTS,
        "stored cohohrts differ from expected imports.",
    );
}
