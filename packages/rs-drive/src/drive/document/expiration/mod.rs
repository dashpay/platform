//! Document expiry (protocol version 14): documents of a type declaring a `ttl`.
//!
//! Every such document gets an entry in the documents expirations tree under `Misc` when it
//! is created, keyed by the time it expires (`$createdAt` plus the time to live) and its id:
//!
//! ```text
//! Misc / E / <expires at, u64 big endian ms> / <document id> -> contract id ++ document type name
//! ```
//!
//! After each block's state transitions the platform deletes up to
//! `max_document_expirations_per_block` expired documents, oldest first
//! ([`Drive::remove_expired_documents`]). Deleting such a document any other way (its owner,
//! a moderator) removes its entry in the same batch, so an entry exists exactly while its
//! document does. The last entry of an expiry time takes the tree of that time with it, so
//! every tree of an expiry time holds at least one entry.
//!
//! A document of such a type is written without storage flags and refunds nothing when it
//! goes. Its bytes, its index entries' and its expiration entry's, are priced for the time
//! they will live by the fee schedule's `document_ttl` group ([`pricing`]), and its creation
//! prepays its deletion as processing.
//!
//! [`Drive::remove_expired_documents`]: crate::drive::Drive::remove_expired_documents

mod add_document_expiration_operations;
mod add_estimation_costs_for_document_expiration;
mod fetch_expired_documents;
mod insert_documents_expirations_tree;
/// Paths of the documents expirations tree
pub mod paths;
/// Prices of the bytes and the deletion of documents with a time to live
pub mod pricing;
mod remove_document_expiration_operations;
mod remove_expired_documents;

pub use fetch_expired_documents::ExpiredDocument;
pub use remove_expired_documents::RemovedExpiredDocuments;

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;

/// What an entry of the documents expirations tree stores: where the document is. Its id is
/// the entry's key, the time it expires the key of the tree holding the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentExpirationEntry {
    /// The contract of the document
    pub contract_id: Identifier,
    /// The name of the document's type
    pub document_type_name: String,
}

impl DocumentExpirationEntry {
    /// The stored form: the 32 bytes of the contract id, then the document type name.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + self.document_type_name.len());
        bytes.extend_from_slice(self.contract_id.as_slice());
        bytes.extend_from_slice(self.document_type_name.as_bytes());
        bytes
    }

    /// The size of the stored form of an entry for a document type of this name.
    pub fn serialized_size(document_type_name: &str) -> u32 {
        32 + document_type_name.len() as u32
    }

    /// Reads the stored form back.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let Some((contract_id, name)) = bytes.split_first_chunk::<32>() else {
            return Err(Error::Drive(DriveError::CorruptedSerialization(
                "a document expiration entry must start with a 32 byte contract id".to_string(),
            )));
        };
        let document_type_name = String::from_utf8(name.to_vec()).map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "the document type name of a document expiration entry must be UTF-8".to_string(),
            ))
        })?;
        Ok(DocumentExpirationEntry {
            contract_id: Identifier::new(*contract_id),
            document_type_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentExpirationEntry;
    use dpp::identifier::Identifier;

    #[test]
    fn should_round_trip_an_expiration_entry() {
        let entry = DocumentExpirationEntry {
            contract_id: Identifier::new([7; 32]),
            document_type_name: "note".to_string(),
        };
        let bytes = entry.to_bytes();
        assert_eq!(
            bytes.len() as u32,
            DocumentExpirationEntry::serialized_size("note")
        );
        assert_eq!(
            DocumentExpirationEntry::from_bytes(&bytes).expect("decodes"),
            entry
        );
    }

    #[test]
    fn should_refuse_an_entry_shorter_than_a_contract_id() {
        assert!(DocumentExpirationEntry::from_bytes(&[1; 31]).is_err());
    }
}

#[cfg(test)]
mod expiration_tests;
