//! Store, query, update and remove phenopackets.
//!
//! Run with `cargo run --example quickstart`.

use phenopackets::schema::v2::{
    Phenopacket,
    core::{Disease, Individual, OntologyClass, Sex},
};
use ppktstore::{
    cohort::{Cohort, CohortStore, codec::JsonCohortCodec, fs::FileCohortStore},
    phenopacket::{PhenoStore, fs::FilePhenoStore},
    query::{
        Query,
        phenopacket::{DiagnosedWith, HasSex},
    },
};

/// helper function to create a small example [`Phenopacket`] from three properties.
fn patient(id: &str, sex: Sex, disease: &str) -> Phenopacket {
    Phenopacket {
        id: id.into(),
        subject: Some(Individual {
            id: id.into(),
            sex: sex as i32,
            ..Default::default()
        }),
        diseases: vec![Disease {
            term: Some(OntologyClass {
                id: disease.into(),
                label: String::new(),
            }),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A store is a directory. Here, we use a temporary one so this example leaves nothing behind.
    let dir = tempfile::tempdir()?;
    let store = FilePhenoStore::new(&dir)?;

    // Add: every phenopacket gets a UUID.
    let pfeiffer_syndrome = "OMIM:101600".to_string();
    let alice = store.add(&patient("alice", Sex::Female, &pfeiffer_syndrome))?;
    let bob = store.add(&patient("bob", Sex::Male, &pfeiffer_syndrome))?;
    println!("stored alice as {alice}");
    println!("stored bob as {bob}");

    // Get: `Ok(None)` means "not found", `Err` means the backend failed.
    if let Ok(Some(mut pp)) = store.get(&bob) {
        // Update.
        let coffin_siris_syndrome = " OMIM:616938".to_string();
        pp.diseases[0].term.as_mut().unwrap().id = coffin_siris_syndrome;
        assert!(store.update(&bob, &pp)?); // a successful update returns Ok(true)
    }

    // Query phenopackets using provided example query structs
    let query = DiagnosedWith(pfeiffer_syndrome) & !HasSex::is_male();

    let hits: Vec<Phenopacket> = store
        .iter_phenopackets()?
        .filter_map(|id| store.get(&id).ok().flatten())
        // a Query tests a single item, here a phenopacket
        .filter(|pp| query.test(pp))
        .collect();

    println!(
        "{} matches: {:?}",
        hits.len(),
        hits.iter().map(|pp| &pp.id).collect::<Vec<_>>()
    );

    // Remove a phenopacket from the store.
    assert!(store.remove(&alice)?);
    assert!(store.get(&alice)?.is_none());

    // create a CohortStore
    // the FileCohortStore needs a codec to read/write from/to the desired file format.
    let cohort_dir = tempfile::tempdir()?;
    let cohortstore = FileCohortStore::new(&cohort_dir, JsonCohortCodec)?;

    // add phenopackets to a cohort by their UUIDs
    let alice_and_bob = Cohort {
        id: "BobAndAlice".to_string(),
        description: "alice and bob are long friends.".to_string(),
        members: vec![alice, bob],
    };
    let cohort_id = cohortstore.add(&alice_and_bob)?;
    println!(
        "alice and bob are part of the cohort {:?}, which is stored under ID: {:?}",
        cohortstore.get(&cohort_id)?.unwrap().id,
        cohort_id
    );

    Ok(())
}
