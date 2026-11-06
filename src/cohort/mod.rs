/// (De)serialize [`Cohort`]s into different data formats.
pub mod codec;
mod core;
/// An implementation of a [`CohortStore`] backed by a file system.
pub mod fs;

pub use core::{Cohort, CohortStore, CohortStoreError};
