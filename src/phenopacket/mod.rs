mod core;

/// Phenopacket persistence backend implemented by a file system.
pub mod fs;

pub use core::{IoErrorKind, PhenoStore, PhenoStoreError};
