mod core;

/// Phenopacket persistence backend implemented by a file system.
pub mod fs;

/// Selection of convenient query structs
pub mod query;

pub use core::{IoErrorKind, PhenoStore, PhenoStoreError};
