//! How a compiled document type stores its documents.

use super::stored_error;
use crate::data_contract::document_type::PrivateDocumentConfig;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use std::num::NonZeroU32;

/// How a document type stores its documents: whole, as index entries alone (`indexOnly`), or
/// as the fixed-size encrypted entries of a private type (`private`, protocol version 15).
///
/// Written where this generation of the stored form first kept `indexOnly` as a boolean, and
/// encoded as that boolean for the two kinds it had: bincode writes a variant's index as one
/// byte, 0 or 1 as `false` or `true`, so a type that is not private takes exactly the bytes it
/// took before private types existed, and only a private type pays for its entry size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredDocumentStorage {
    /// Documents stored whole.
    Documents,
    /// Documents stored as their index entries alone.
    IndexOnly,
    /// A private type's encrypted entries, each of this many bytes.
    Private(u32),
}

impl StoredDocumentStorage {
    /// The storage of a parsed type, which is never both: the parse refuses `indexOnly` on a
    /// private type.
    pub(super) fn of(
        index_only: bool,
        private_document: Option<PrivateDocumentConfig>,
    ) -> Result<Self, ProtocolError> {
        match (index_only, private_document) {
            (false, None) => Ok(Self::Documents),
            (true, None) => Ok(Self::IndexOnly),
            (false, Some(private_document)) => Ok(Self::Private(private_document.entry_size())),
            (true, Some(_)) => Err(ProtocolError::CorruptedCodeExecution(
                "a parsed document type is both indexOnly and private".to_string(),
            )),
        }
    }

    /// The parsed type's `index_only` and `private_document`. Only a parsed type is stored,
    /// and its parse refused an entry of no bytes: one read back is corrupted storage.
    pub(super) fn into_parts(self) -> Result<(bool, Option<PrivateDocumentConfig>), ProtocolError> {
        match self {
            Self::Documents => Ok((false, None)),
            Self::IndexOnly => Ok((true, None)),
            Self::Private(entry_size) => NonZeroU32::new(entry_size)
                .map(|entry_size| (false, Some(PrivateDocumentConfig::new(entry_size))))
                .ok_or_else(|| {
                    stored_error("a private type declares entries of no bytes".to_string())
                }),
        }
    }
}
