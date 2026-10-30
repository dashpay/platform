//! Private document types: the `private` keyword (protocol version 15).
//!
//! A private document type's documents are fixed-size encrypted entries. An anonymous
//! transition paid from the shielded pool appends them: nobody owns, replaces or deletes one,
//! and the chain never reads inside one, so it can check nothing a document type usually
//! promises about its documents. Recipients find their entries by trying to decrypt each one.
//!
//! The type's `properties` describe the plaintext, so that clients can decode it with types.
//! The chain checks them at registration like any schema, and never against an entry.

use crate::data_contract::document_type::property_names;

/// What a document type's `private` keyword declares (`{ "entrySize": n }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateDocumentConfig {
    entry_size: u32,
}

impl PrivateDocumentConfig {
    /// A private document type whose entries take `entry_size` bytes each.
    pub fn new(entry_size: u32) -> Self {
        Self { entry_size }
    }

    /// How many bytes every entry a client appends takes: fixed for the type, so that entries
    /// do not differ in size by what they say. Drive stores each with a header of its own,
    /// which this does not count.
    pub fn entry_size(&self) -> u32 {
        self.entry_size
    }
}

/// The doctype-level keywords a private document type may declare. Each other keyword is a
/// promise about the type's documents (an index, a history, a transfer, a cost, a rule, a
/// reference, ...) that the chain could only keep by reading them, and is refused. A keyword
/// added later is refused here until someone decides what it means on a private type.
///
/// `documentsMutable`, `canBeDeleted` and `documentsKeepHistory` are admitted only to be
/// written out as `false`, against a contract default saying otherwise. A document type's own
/// `$defs` is not listed: every parse refuses it on any type, the contract's `$defs` being
/// where definitions go.
pub(crate) const PRIVATE_DOCUMENT_TYPE_KEYWORDS: [&str; 15] = [
    "type",
    "properties",
    "required",
    "additionalProperties",
    "description",
    "$comment",
    property_names::MIN_PROPERTIES,
    property_names::MAX_PROPERTIES,
    property_names::DEPENDENT_REQUIRED,
    property_names::PRIVATE,
    property_names::DOCUMENTS_MUTABLE,
    property_names::CAN_BE_DELETED,
    property_names::DOCUMENTS_KEEP_HISTORY,
    property_names::REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY,
    property_names::REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY,
];
