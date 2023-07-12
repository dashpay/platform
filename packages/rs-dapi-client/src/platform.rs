//! Platform DAPI requests.

mod get_documents;
mod get_identity;

pub use get_identity::{GetIdentity, GetIdentityProof};

/// Error indicates that the transport response contains insufficient information.
#[derive(Debug, thiserror::Error)]
#[error("returned message contains unexpected `None`s")]
pub struct IncompleteMessage;
