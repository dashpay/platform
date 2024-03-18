// MIT LICENSE
//
// Copyright (c) 2022 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! General Drive Document Functions
//!
//! This module defines general functions relevant to Documents in Drive.
//! Namely functions to return the paths to certain objects and the path sizes.
//!

#[cfg(feature = "full")]
use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;
#[cfg(feature = "full")]
use crate::drive::flags::StorageFlags;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::{defaults, RootTree};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "full")]
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
#[cfg(feature = "full")]
use grovedb::batch::key_info::KeyInfo;
#[cfg(feature = "full")]
use grovedb::batch::KeyInfoPath;
#[cfg(feature = "full")]
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
#[cfg(feature = "full")]
use grovedb::Element;

#[cfg(feature = "full")]
mod delete;
#[cfg(feature = "full")]
mod estimation_costs;
#[cfg(feature = "full")]
mod index_uniqueness;
#[cfg(any(feature = "full", feature = "fixtures-and-mocks"))]
mod insert;
#[cfg(any(feature = "full", feature = "fixtures-and-mocks"))]
pub mod query;
#[cfg(any(feature = "full", feature = "fixtures-and-mocks"))]
mod update;

#[cfg(any(feature = "full", feature = "verify"))]
/// Returns the path to a contract document type.
pub(crate) fn contract_document_type_path<'a>(
    contract_id: &'a [u8; 32],
    document_type_name: &'a str,
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[1],
        document_type_name.as_bytes(),
    ]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Returns the path to a contract document type.
pub(crate) fn contract_document_type_path_vec(
    contract_id: &[u8],
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
    ]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Returns the path to the primary keys of a contract document type.
pub(crate) fn contract_documents_primary_key_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments), // 1
        contract_id,                                             // 32
        &[1],                                                    // 1
        document_type_name.as_bytes(),
        &[0], // 1
    ]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Returns the path to a contract document.
fn contract_documents_keeping_history_primary_key_path_for_document_id<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
    document_id: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[1],
        document_type_name.as_bytes(),
        &[0],
        document_id,
    ]
}

#[cfg(feature = "full")]
/// Returns the path to a contract document when the document id isn't known.
fn contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
    contract_id: &[u8],
    document_type: DocumentTypeRef,
) -> KeyInfoPath {
    let mut key_info_path = KeyInfoPath::from_known_path(contract_documents_primary_key_path(
        contract_id,
        document_type.name().as_str(),
    ));
    key_info_path.push(KeyInfo::MaxKeySize {
        unique_id: document_type.unique_id_for_storage().to_vec(),
        max_size: DEFAULT_HASH_SIZE_U8,
    });
    key_info_path
}

#[cfg(any(feature = "full", feature = "verify"))]
#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
/// Returns the size of the path to a contract document.
fn contract_documents_keeping_history_primary_key_path_for_document_id_size(
    document_type_name_len: u32,
) -> u32 {
    defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE
        + document_type_name_len
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Returns the size of the path to the time at which a document type was stored.
fn contract_documents_keeping_history_storage_time_reference_path_size(
    document_type_name_len: u32,
) -> u32 {
    defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH
        + document_type_name_len
}

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
fn unique_event_id() -> [u8; 32] {
    rand::random::<[u8; 32]>()
}

/// Tests module
#[cfg(feature = "full")]
#[cfg(test)]
pub(crate) mod tests {
    use std::option::Option::None;

    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::json_document_to_contract;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
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
