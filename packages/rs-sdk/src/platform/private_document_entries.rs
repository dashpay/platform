//! The entries of a private document type's store (`getPrivateDocumentEntries`): the type's
//! fixed-size encrypted entries in append order, each with the height and time of the block
//! that appended it, a page at a time. Nodes serve them from protocol version 15, and the SDK
//! sends no such request before the network has proven 15.
//!
//! A page starts at a chunk boundary of the store (a multiple of its chunk size, which the
//! type's entry size sets) and holds at most a chunk; a page shorter than asked for ends at the
//! store's last entry, as its `total_count` tells. A reader walking the store from its cursor
//! asks for the chunk holding the cursor and skips the entries before it.
//!
//! [`PrivateDocumentEntries::fetch`] verifies the page against the platform's root hash, with
//! the contract the context provider supplies, so a node can neither leave entries out of it
//! nor change their header times. [`FetchUnproved`] reads it without a proof.

use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_private_document_entries_request::GetPrivateDocumentEntriesRequestV0;
use dapi_grpc::platform::v0::{
    get_private_document_entries_request, GetPrivateDocumentEntriesRequest,
};
pub use drive_proof_verifier::types::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntry,
};

/// Query for a page of the entries of a private document type's store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateDocumentEntriesQuery {
    /// The contract declaring the private type.
    pub data_contract_id: Identifier,
    /// The private type.
    pub document_type_name: String,
    /// The page's first position: a chunk boundary of the store.
    pub start_position: u64,
    /// How many entries to read; 0, or more than a chunk, reads a whole chunk.
    pub count: u32,
}

impl PrivateDocumentEntriesQuery {
    /// The query for the whole chunk of the store of `document_type_name` starting at
    /// `start_position`.
    pub fn new(
        data_contract_id: Identifier,
        document_type_name: impl Into<String>,
        start_position: u64,
    ) -> Self {
        PrivateDocumentEntriesQuery {
            data_contract_id,
            document_type_name: document_type_name.into(),
            start_position,
            count: 0,
        }
    }

    /// The same query for at most `count` entries.
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }
}

impl Query<GetPrivateDocumentEntriesRequest> for PrivateDocumentEntriesQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetPrivateDocumentEntriesRequest, Error> {
        Ok(GetPrivateDocumentEntriesRequest {
            version: Some(get_private_document_entries_request::Version::V0(
                GetPrivateDocumentEntriesRequestV0 {
                    data_contract_id: self.data_contract_id.to_vec(),
                    document_type_name: self.document_type_name.clone(),
                    start_position: self.start_position,
                    count: self.count,
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for PrivateDocumentEntries {
    type Query = GetPrivateDocumentEntriesRequest;
    type Request = GetPrivateDocumentEntriesRequest;
}

impl FetchUnproved for PrivateDocumentEntries {
    type Request = GetPrivateDocumentEntriesRequest;
}
