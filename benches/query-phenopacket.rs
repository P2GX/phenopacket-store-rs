//! Benchmark the `Query<Phenopacket>` examples in `crate::query::phenopacket`

use criterion::{Criterion, criterion_group, criterion_main};
use ontolius::{TermId, ontology::csr::MinimalCsrOntology};
use phenopackets::schema::v2::Phenopacket;
use ppktstore::query::{Query, phenopacket as ppquery};
use prost::Message;

fn criterion_benchmark(c: &mut Criterion) {
    // shared resources and convenience functions
    let pp = example_phenopacket();
    let hpo = load_hpo();
    let observed = |id: &str| ppquery::HasPhenotypicFeature {
        query: id.parse::<TermId>().expect("should be a valid term id."),
        excluded: false,
        taxonomy: &hpo,
    };
    let excluded = |id: &str| ppquery::HasPhenotypicFeature {
        query: id.parse::<TermId>().expect("should be a valid term id."),
        excluded: true,
        taxonomy: &hpo,
    };

    //
    // benches
    //
    let mut overview_group = c.benchmark_group("query-phenopacket-overview");

    overview_group.bench_function("HasSex/hit", |b| {
        let query = ppquery::HasSex::is_male();

        b.iter(|| query.test(&pp));
    });

    overview_group.bench_function("HasVitalStatus/miss", |b| {
        let query = ppquery::HasVitalStatus::is_alive();

        b.iter(|| query.test(&pp));
    });

    overview_group.bench_function("DiagnosedWith/hit", |b| {
        let pfeiffer_syndrome = String::from("OMIM:101600");
        let query = ppquery::DiagnosedWith(pfeiffer_syndrome);

        b.iter(|| query.test(&pp));
    });

    overview_group.bench_function("DiagnosedWith/miss", |b| {
        let feingold_syndrome = String::from("OMIM:164280"); // example patient is not diagnosed with feingold syndrome
        let query = ppquery::DiagnosedWith(feingold_syndrome);

        b.iter(|| query.test(&pp));
    });

    overview_group.bench_function("HasPhenotypicFeature/best", |b| {
        let q_observed: TermId = "HP:0001558".parse().unwrap(); // Decreased fetal movement
        let query = ppquery::HasPhenotypicFeature {
            query: q_observed,
            excluded: false,
            taxonomy: &hpo,
        };

        b.iter(|| query.test(&pp));
    });

    overview_group.bench_function("HasPhenotypicFeature/worst", |b| {
        let q_not_observed: TermId = "HP:0001989".parse().unwrap(); // Fetal akinesia sequence
        let query = ppquery::HasPhenotypicFeature {
            query: q_not_observed,
            excluded: false,
            taxonomy: &hpo,
        };

        b.iter(|| query.test(&pp));
    });
    overview_group.finish();

    let mut observed_phenotypic_feature_group = c.benchmark_group("HasPhenotypicFeature");

    // (benchmark id, query, expected return value)
    let cases = [
        // Decreased fetal movement (HP:0001558) is observed in example at position 0
        ("observed/exact/0", observed("HP:0001558"), true),
        // Prenatal movement abnormality (HP:0001557) is implicitly observed by observed decendent term decreased fetal movement at pos 0
        ("observed/implied/0", observed("HP:0001558"), true),
        // motor delay (HP:0001270) is observed at index 3.
        ("observed/exact/3", observed("HP:0001270"), true),
        // neurodevelopmental delay (HP:0012758) is implicitly observed by motor delay at index 3
        ("observed/implied/3", observed("HP:0012758"), true),
        // Fetal akinesia sequence (HP:0001989) is neither observed nor excluded in example
        ("observed/not_implied", observed("HP:0001989"), false),
        // Abn. cranial nerve physiology is excluded in example at pos 1
        ("excluded/exact/1", excluded("HP:0031910"), true),
        // Cranial nerve paralysis is implicitly excluded by excluded term at pos 1
        ("excluded/implied/1", excluded("HP:0006824"), true),
        // Fetal akinesia sequence (HP:0001989) is neither observed nor excluded in example
        ("excluded/not_implied", excluded("HP:0001989"), false),
    ];

    for (bench_id, query, expected) in cases {
        assert_eq!(query.test(&pp), expected);

        observed_phenotypic_feature_group.bench_with_input(bench_id, &query, |b, query| {
            b.iter(|| query.test(&pp));
        });
    }
    observed_phenotypic_feature_group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

//
// HELPERS
//
/// parse the example phenopacket stored in the repo
fn example_phenopacket() -> Phenopacket {
    const BYTES: &[u8] =
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/phenopacket.pb"));
    Phenopacket::decode(BYTES).expect("data/phenopacket.pb should be a valid phenopacket")
}

fn load_hpo() -> MinimalCsrOntology {
    use ontolius::io::OntologyLoaderBuilder;
    use std::{fs::File, io::BufReader, path::Path};

    let path = Path::new("data").join("hp.json");
    let loader = OntologyLoaderBuilder::new().obographs_parser().build();
    let reader = BufReader::new(File::open(path).expect("setup: failed to open ontology file."));

    loader
        .load_from_buf_read(reader)
        .expect("failed to load from reader.")
}
