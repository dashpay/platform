//! Document query surface.
//!
//! The transport-free core (query types, wire encoding, proof decoding)
//! lives in the `dash-platform-queries` crate and is re-exported here at
//! its historical paths; this module keeps the Sdk-bound pieces — `Fetch`
//! bindings, the contract-fetching constructor, and transition builders.

pub use dash_platform_queries::documents::{
    chained_document_query, composite_document_query, document_average, document_count,
    document_having_entries, document_history_query, document_query, document_ranked_entries,
    document_split_averages, document_split_counts, document_split_sums, document_sum,
};

pub mod chained_document_query_sdk;
pub mod composite_document_query_sdk;
pub mod document_query_sdk;
mod fetch_bindings;
pub mod transitions;

pub use document_query_sdk::DocumentQuerySdk;
