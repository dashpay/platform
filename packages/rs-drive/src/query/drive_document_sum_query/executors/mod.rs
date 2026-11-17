//! Sum-query executor modules. One per no-proof `DocumentSumMode`, each
//! owning a single executor function and the helpers it needs, and one
//! ([`proof`]) for every proof mode and every proved average. Mirrors
//! count's `executors/` layout — file names parallel byte-for-byte.

pub mod per_in_value;
pub mod proof;
pub mod range_no_proof;
pub mod total;
