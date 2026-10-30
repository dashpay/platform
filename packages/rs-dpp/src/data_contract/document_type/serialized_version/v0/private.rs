//! What a private document type's `private` keyword declares, stored.

use crate::data_contract::document_type::PrivateDocumentConfig;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use std::num::NonZeroU32;

/// [`PrivateDocumentConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredPrivateDocument {
    entry_size: u32,
}

impl From<PrivateDocumentConfig> for StoredPrivateDocument {
    fn from(private_document: PrivateDocumentConfig) -> Self {
        Self {
            entry_size: private_document.entry_size(),
        }
    }
}

/// Only a parsed type is stored, and its parse refused an entry of no bytes: one read back is
/// corrupted storage.
impl TryFrom<StoredPrivateDocument> for PrivateDocumentConfig {
    type Error = ProtocolError;

    fn try_from(stored: StoredPrivateDocument) -> Result<Self, Self::Error> {
        NonZeroU32::new(stored.entry_size)
            .map(PrivateDocumentConfig::new)
            .ok_or_else(|| {
                ProtocolError::CorruptedSerialization(
                    "a stored private document type declares entries of no bytes".to_string(),
                )
            })
    }
}
