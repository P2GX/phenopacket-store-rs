//! Shared benchmark helpers.

use criterion::profiler::Profiler;
use pprof::ProfilerGuard;
use std::{fs::File, path::Path};

/// Criterion [`Profiler`] that writes a flamegraph for each benchmark.
///
/// Active only with `cargo bench ... -- --profile-time <seconds>`.
/// The output is written to `target/criterion/<group>/<bench>/profile/flamegraph.svg`.
///
/// `pprof`'s own `criterion` integration targets criterion 0.5, hence this small re-implementation.
pub struct FlamegraphProfiler<'a> {
    frequency: i32,
    active: Option<ProfilerGuard<'a>>,
}

impl FlamegraphProfiler<'_> {
    pub fn new(frequency: i32) -> Self {
        Self {
            frequency,
            active: None,
        }
    }
}

impl Profiler for FlamegraphProfiler<'_> {
    fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
        self.active = Some(ProfilerGuard::new(self.frequency).expect("profiler should start"));
    }

    fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
        std::fs::create_dir_all(benchmark_dir).expect("benchmark dir should be creatable");
        let path = benchmark_dir.join("flamegraph.svg");
        let file = File::create(&path).expect("flamegraph file should be creatable");
        let guard = self.active.take().expect("profiler should be running");
        let report = guard.report().build().expect("report should build");
        report
            .flamegraph(file)
            .expect("flamegraph should be written");
        eprintln!("flamegraph written to {}", path.display());
    }
}
