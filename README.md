
# ppktstore-rs

[![Crates.io](https://img.shields.io/crates/v/ppktstore.svg)](https://crates.io/crates/ppktstore)
[![docs.rs](https://img.shields.io/docsrs/ppktstore)](https://docs.rs/ppktstore)
[![Tests](https://img.shields.io/github/actions/workflow/status/P2GX/phenopacket-store-rs/ci.yml?branch=development&label=tests)](https://github.com/P2GX/phenopacket-store-rs/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/ppktstore)](https://crates.io/crates/ppktstore)
[![License](https://img.shields.io/crates/l/ppktstore)](https://crates.io/crates/ppktstore)
[![GA4GH](https://img.shields.io/badge/GA4GH-Phenopackets-orange.svg)](https://github.com/phenopackets)

---

A Rust crate, providing an interface to store, organise and query [GA4GH Phenopackets](https://phenopacket-schema.readthedocs.io).

- **Store** phenopackets and cohorts behind small traits (`PhenoStore`, `CohortStore`), currently backed by files.
- **Import** a [Phenopacket Store](https://github.com/monarch-initiative/phenopacket-store) release.
- **Query** with composable building blocks. combinable via the boolean operators `&`, `|` and `!`.

This crate is pre-1.0, so the API may still change.

## Quickstart

Add `ppktstore` to your project, you will likely want `phenopackets` aswell.

```shell
cargo add ppktstore phenopackets
```

Creating a store and adding a phenopacket to it is one line each. In the following snippet, we create a `FilePhenoStore`, add three `Phenopacket`s to it, remove one, define a composed query and use it to filter our stored `Pheonpackets`.

```rust
use phenopackets::schema::v2::{
    Phenopacket,
    core::{Disease, Individual, OntologyClass, Sex},
};
use ppktstore::{
    phenopacket::{PhenoStore, fs::FilePhenoStore},
    query::{
        Query,
        phenopacket::{DiagnosedWith, HasSex},
    },
};

/// a helper to create example phenopackets
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

    // a file store is a directory, a temporary one keeps the example self-contained.
    let dir = tempfile::tempdir()?;
    let store = FilePhenoStore::new(&dir)?;

    // Every stored phenopacket gets a UUID.
    let pfeiffer_syndrome = "OMIM:101600";
    let alice_id = store.add(&patient("alice", Sex::Female, pfeiffer_syndrome))?;
    let bob_id = store.add(&patient("bob", Sex::Male, pfeiffer_syndrome))?;
    let charline_id = store.add(&patient("charline", Sex::UnknownSex, pfeiffer_syndrome))?;

    // remove a phenopacket from the store using its id
    store.remove(&charline_id)?;

    // Queries are plain values that can be combined.
    let query_not_male_pfeiffer = DiagnosedWith(pfeiffer_syndrome.into()) & !HasSex::is_male();

    let hits: Vec<Phenopacket> = store
        .iter_phenopackets()?
        .filter_map(|id| store.get(&id).ok().flatten())
        .filter(|pp| query_not_male_pfeiffer.test(pp))
        .collect();

    // we removed charline, so alice is the only non-male with pfeiffer syndrome.
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "alice");
    Ok(())
}
```

See the [`examples/`](https://github.com/P2GX/phenopacket-store-rs/tree/master/examples) directory for longer, runnable programs. You can run them via:

```shell
cargo run --example <name_of_example> [--all-features]
```
