//! Platform DAPI requests.

mod get_identity;

#[derive(Debug, thiserror::Error)]
#[error("returned message contains unexpected `None`s")]
pub struct IncompleteMessage;
