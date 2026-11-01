//! The entries of a private document type's store (`getPrivateDocumentEntries`): the type's
//! fixed-size encrypted entries in append order, each with the height and time of the block
//! that appended it, a page at a time. Nodes serve them from protocol version 15, and the SDK
//! sends no such request before the network has proven 15.
//!
//! A page starts at any position of the store and reads to the end of the store's chunk
//! holding it (the type's entry size sets the chunk), or to the store's last entry, as its
//! `total_count` tells. A reader walking the store asks for the page from its cursor, then
//! for the page from the position after the last entry it got.
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
    /// The page's first position.
    pub start_position: u64,
}

impl PrivateDocumentEntriesQuery {
    /// The query for the page of the store of `document_type_name` from `start_position` to
    /// the end of the chunk holding it.
    pub fn new(
        data_contract_id: Identifier,
        document_type_name: impl Into<String>,
        start_position: u64,
    ) -> Self {
        PrivateDocumentEntriesQuery {
            data_contract_id,
            document_type_name: document_type_name.into(),
            start_position,
        }
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

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::{Sdk, SdkBuilder};
    use dpp::version::PlatformVersion;

    #[tokio::test]
    async fn should_fetch_a_page_of_private_entries_from_mock_expectations() {
        let mut sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk");
        let query = PrivateDocumentEntriesQuery::new(Identifier::new([4; 32]), "secret", 64);
        let page = PrivateDocumentEntries {
            entries: (64..67u64)
                .map(|position| PrivateDocumentEntry {
                    position,
                    block_height: 10 + position,
                    block_time_ms: 500 * position,
                    data: vec![position as u8; 585],
                })
                .collect(),
            total_count: 67,
        };
        sdk.mock()
            .expect_fetch(query.clone(), Some(page.clone()))
            .await
            .expect("expected to register the page");

        assert_eq!(
            PrivateDocumentEntries::fetch(&sdk, query)
                .await
                .expect("expected the page"),
            Some(page)
        );
    }
}
