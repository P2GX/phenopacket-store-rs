//! Benchmark the [`FilePhenoStore`] using criterion.

use criterion::{
    self, BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use phenopackets::schema::v2::Phenopacket;
use ppktstore::phenopacket::{PhenoStore, fs::FilePhenoStore};
use prost::Message;
use std::hint::black_box;
use tempfile::TempDir;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("new", |b| {
        let temp_dir = tempfile::TempDir::new().unwrap();
        b.iter(|| FilePhenoStore::new(black_box(temp_dir.path())));
    });

    c.bench_function("add", |b| {
        let ps = example_filephenostore();
        let pp = example_phenopacket();

        b.iter(|| {
            ps.add(black_box(&pp))
                .expect("adding a phenopacket multipe times should not be an issue.")
        });
    });

    c.bench_function("get", |b| {
        let ps = example_filephenostore();
        let pp = example_phenopacket();
        let id = ps
            .add(&pp)
            .expect("should be able to add valid phenopacket.");

        b.iter(|| {
            ps.get(black_box(&id))
                .expect("should be able to retrieve existing phenopacket.")
        })
    });

    c.bench_function("update", |b| {
        let ps = example_filephenostore();
        let pp = example_phenopacket();
        let id = ps
            .add(&pp)
            .expect("should be able to add valid phenopacket.");

        b.iter(|| {
            ps.update(black_box(&id), black_box(&pp))
                .expect("should be able to update with unchanged phenopacket.")
        });
    });

    c.bench_function("remove", |b| {
        let ps = example_filephenostore();
        let pp = example_phenopacket();

        // only the routine (second closure) is timed, the setup (first closure) is not.
        b.iter_batched(
            || {
                ps.add(&pp)
                    .expect("should be able to add valid phenopacket.")
            },
            |id| {
                let removed = ps
                    .remove(black_box(&id))
                    .expect("should be able to remove existing phenopacket.");
                assert!(
                    removed,
                    "the phenopacket should have existed before removal."
                );
            },
            BatchSize::SmallInput,
        );
    });

    let mut group = c.benchmark_group("iter");
    for n in [10, 100, 1_000, 10_000] {
        let ps = filled_filephenostore(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &ps, |b, ps| {
            // the iterator is lazy, so it has to be consumed to measure anything.
            b.iter(|| {
                ps.iter_phenopackets()
                    .expect("should be able to read the store directory.")
                    .count()
            });
        });
    }
    group.finish();
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

/// create an empty store in a temporary directory.
/// the store owns the [`TempDir`], so the directory is removed once the store is dropped.
fn example_filephenostore() -> FilePhenoStore<TempDir> {
    let temp_dir = TempDir::new().expect("should be alowed to create temp dirs.");
    FilePhenoStore::new(temp_dir).expect("tempdir should be a valid path.")
}

/// create a store in a temporary directory that contains `n` copies of the example phenopacket.
fn filled_filephenostore(n: usize) -> FilePhenoStore<TempDir> {
    let ps = example_filephenostore();
    let pp = example_phenopacket();
    for _ in 0..n {
        ps.add(&pp)
            .expect("should be able to add valid phenopacket.");
    }
    ps
}
