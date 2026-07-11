//! Per-`DocumentSumMode` executor modules. Each module owns a single
//! executor function and the helpers it needs. Mirrors count's
//! `executors/` layout — file names parallel byte-for-byte.

pub mod per_in_value;
pub mod point_lookup_proof;
pub mod range_aggregate_carrier_proof;
pub mod range_distinct_proof;
pub mod range_no_proof;
pub mod range_proof;
pub mod total;
