mod core;
pub use core::{AndQuery, NotQuery, OrQuery, Query, impl_bool_algebra};

/// A selection of convenient phenopacket queries.
pub mod phenopacket;
