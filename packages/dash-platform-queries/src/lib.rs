//! Transport-free query core of the Dash Platform SDK.
//!
//! This crate carries the pieces of `dash-sdk` that build queries, encode
//! them onto the wire format, and decode/verify proved responses — without
//! any transport dependency (no `rs-dapi-client`, no tokio, no tonic
//! transport stack). Embedders that bring their own transport can depend on
//! this crate alone; `dash-sdk` re-exports everything here at its
//! historical paths.

// Same allowance the code carried in rs-sdk, whose crate root allows
// `result_large_err` for the dpp/drive error types threaded through here.
#![allow(clippy::result_large_err)]

pub mod block_info_from_metadata;
pub mod documents;
pub mod dpns_usernames;
pub mod error;
pub mod mock;
pub mod transition;
pub mod types;

pub use error::Error;
