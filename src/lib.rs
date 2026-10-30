//! # Feature Flags
//! 
//! - `ontolius`: Enables `query::phenopackets::HasPhenotypicFeature`, which allows to query for phenotypic features that are not explicitly included, but implied by another –included– term and the ontology used.

/// Routines for creating, updating and persisting phenopacket cohorts.
pub mod cohort;
/// Import phenopackets from phenopackets-store.
pub mod import;
/// Phenopacket persistence backends.
pub mod phenopacket;
/// Queries for filtering Phenopacket Schema elements.
pub mod query;
/// Utility functions for unit tests.
#[cfg(test)]
pub(crate) mod testutils;
