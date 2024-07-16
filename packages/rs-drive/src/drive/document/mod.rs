//! General Drive Document Functions
//!
//! This module defines general functions relevant to Documents in Drive.
//! Namely functions to return the paths to certain objects and the path sizes.
//!

#[cfg(feature = "server")]
use crate::drive::votes::paths::CONTESTED_DOCUMENT_STORAGE_TREE_KEY;
#[cfg(feature = "server")]
use crate::util::storage_flags::StorageFlags;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "server")]
use dpp::document::Document;
#[cfg(feature = "server")]
use dpp::document::DocumentV0Getters;
#[cfg(feature = "server")]
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
#[cfg(feature = "server")]
use grovedb::Element;

#[cfg(feature = "server")]
mod delete;
#[cfg(feature = "server")]
mod estimation_costs;
#[cfg(feature = "server")]
mod index_uniqueness;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod insert;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod insert_contested;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
pub mod query;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod update;

/// paths
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

#[cfg(feature = "server")]
/// Creates a reference to a document.
fn make_document_reference(
    document: &Document,
    document_type: DocumentTypeRef,
    storage_flags: Option<&StorageFlags>,
) -> Element {
    // we need to construct the reference from the split height of the contract document
    // type which is at 4
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    let mut reference_path = vec![vec![0], document.id().to_vec()];
    let mut max_reference_hops = 1;
    if document_type.documents_keep_history() {
        reference_path.push(vec![0]);
        max_reference_hops += 1;
    }
    // 2 because the contract could allow for history
    // 4 because
    // -DataContractDocumentsTree
    // -DataContract ID
    // - 1 Documents inDataContract
    // - DocumentType
    // We add 2 or 3
    // - 0 Storage
    // - Document id
    // -(Optional) 0 (means latest) in the case of documents_keep_history
    Element::Reference(
        UpstreamRootHeightReference(4, reference_path),
        Some(max_reference_hops),
        StorageFlags::map_to_some_element_flags(storage_flags),
    )
}

#[cfg(feature = "server")]
/// Creates a reference to a contested document.
fn make_document_contested_reference(
    document: &Document,
    storage_flags: Option<&StorageFlags>,
) -> Element {
    // we need to construct the reference from the split height of the contract document
    // type which is at 5 for the contested tree
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    let reference_path = vec![
        vec![CONTESTED_DOCUMENT_STORAGE_TREE_KEY],
        document.id().to_vec(),
    ];
    let max_reference_hops = 1;
    // 2 because the contract could allow for history
    // 5 because
    // -VotesTree
    // -ContestedResourceTree
    // -ActivePolls
    // -DataContract ID
    // - DocumentType
    // We add 2
    // - 0 Storage
    // - Document id
    Element::Reference(
        UpstreamRootHeightReference(5, reference_path),
        Some(max_reference_hops),
        StorageFlags::map_to_some_element_flags(storage_flags),
    )
}

#[cfg(feature = "server")]
/// size of a document reference.
fn document_reference_size(document_type: DocumentTypeRef) -> u32 {
    // we need to construct the reference from the split height of the contract document
    // type which is at 4
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    // vec![vec![0], Vec::from(document.id)];
    // 1 (vec size) + 1 (subvec size) + 1 (0) + 1 (subvec size) + 32 (document id size)
    let mut reference_path_size = 36;
    if document_type.documents_keep_history() {
        reference_path_size += 2;
    }

    // 1 for type reference
    // 1 for reference type
    // 1 for root height offset
    // reference path size
    // 1 reference_hops options
    // 1 reference_hops count
    // 1 element flags option
    6 + reference_path_size
}

#[cfg(feature = "server")]
fn unique_event_id() -> [u8; 32] {
    rand::random::<[u8; 32]>()
}

/// Tests module
#[cfg(feature = "server")]
#[cfg(test)]
pub(crate) mod tests {
    use std::option::Option::None;

    use crate::drive::Drive;
    use crate::util::storage_flags::StorageFlags;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::json_document_to_contract;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    /// Setup Dashpay
    pub fn setup_dashpay(_prefix: &str, mutable_contact_requests: bool) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let dashpay_path = if mutable_contact_requests {
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json"
        } else {
            "tests/supporting_files/contract/dashpay/dashpay-contract.json"
        };

        // let's construct the grovedb structure for the dashpay data contract
        let dashpay = json_document_to_contract(dashpay_path, false, platform_version)
            .expect("expected to get cbor document");
        drive
            .apply_contract(
                &dashpay,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, dashpay)
    }
}
