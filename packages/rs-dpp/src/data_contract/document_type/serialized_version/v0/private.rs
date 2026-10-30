//! What a private document type's `private` keyword declares, stored.

use crate::data_contract::document_type::PrivateDocumentConfig;
use bincode::{Decode, DecodeUntrusted, Encode};

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

impl From<StoredPrivateDocument> for PrivateDocumentConfig {
    fn from(stored: StoredPrivateDocument) -> Self {
        PrivateDocumentConfig::new(stored.entry_size)
    }
}
