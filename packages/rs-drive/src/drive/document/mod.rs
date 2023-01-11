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

use crate::contract::document::Document;
use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;
use crate::drive::flags::StorageFlags;
use crate::drive::{defaults, RootTree};
use dpp::data_contract::extra::DocumentType;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
use grovedb::Element;
use std::borrow::Cow;

mod delete;
mod estimation_costs;
mod insert;
mod update;

/// Returns the path to a contract document type.
pub(crate) fn contract_document_type_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
        document_type_name.as_bytes(),
    ]
}

/// Returns the path to a contract document type.
pub(crate) fn contract_document_type_path_vec(
    contract_id: &[u8],
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
    ]
}

/// Returns the path to the primary keys of a contract document type.
pub(crate) fn contract_documents_primary_key_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments), // 1
        contract_id,                                         // 32
        &[1],                                                // 1
        document_type_name.as_bytes(),
        &[0], // 1
    ]
}

/// Returns the path to a contract document.
fn contract_documents_keeping_history_primary_key_path_for_document_id<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
    document_id: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
        document_type_name.as_bytes(),
        &[0],
        document_id,
    ]
}

/// Returns the path to a contract document when the document id isn't known.
fn contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
    contract_id: &[u8],
    document_type: &DocumentType,
) -> KeyInfoPath {
    let mut key_info_path = KeyInfoPath::from_known_path(contract_documents_primary_key_path(
        contract_id,
        document_type.name.as_str(),
    ));
    key_info_path.push(KeyInfo::MaxKeySize {
        unique_id: document_type.unique_id_for_storage().to_vec(),
        max_size: DEFAULT_HASH_SIZE_U8,
    });
    key_info_path
}

/// Returns the size of the path to a contract document.
fn contract_documents_keeping_history_primary_key_path_for_document_id_size(
    document_type_name_len: u32,
) -> u32 {
    defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE
        + document_type_name_len
}

/// Returns the size of the path to the time at which a document type was stored.
fn contract_documents_keeping_history_storage_time_reference_path_size(
    document_type_name_len: u32,
) -> u32 {
    defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH
        + document_type_name_len
}

/// Creates a reference to a document.
fn make_document_reference(
    document: &Document,
    document_type: &DocumentType,
    storage_flags: Option<&StorageFlags>,
) -> Element {
    // we need to construct the reference from the split height of the contract document
    // type which is at 4
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    let mut reference_path = vec![vec![0], Vec::from(document.id)];
    let mut max_reference_hops = 1;
    if document_type.documents_keep_history {
        reference_path.push(vec![0]);
        max_reference_hops += 1;
    }
    // 2 because the contract could allow for history
    // 4 because
    // - ContractDocumentsTree
    // - Contract ID
    // - 1 Documents in Contract
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

/// size of a document reference.
fn document_reference_size(document_type: &DocumentType) -> u32 {
    // we need to construct the reference from the split height of the contract document
    // type which is at 4
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    // vec![vec![0], Vec::from(document.id)];
    // 1 (vec size) + 1 (subvec size) + 1 (0) + 1 (subvec size) + 32 (document id size)
    let mut reference_path_size = 36;
    if document_type.documents_keep_history {
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

fn unique_event_id() -> [u8; 32] {
    rand::random::<[u8; 32]>()
}

/// Tests module
#[cfg(test)]
pub(crate) mod tests {
    use std::option::Option::None;

    use tempfile::TempDir;

    use crate::common::json_document_to_cbor;
    use crate::drive::block_info::BlockInfo;
    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;

    /// Setup Dashpay
    pub fn setup_dashpay(_prefix: &str, mutable_contact_requests: bool) -> (Drive, Vec<u8>) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let dashpay_path = if mutable_contact_requests {
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json"
        } else {
            "tests/supporting_files/contract/dashpay/dashpay-contract.json"
        };

        // let's construct the grovedb structure for the dashpay data contract
        let dashpay_cbor = json_document_to_cbor(dashpay_path, Some(1));
        drive
            .apply_contract_cbor(
                dashpay_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, dashpay_cbor)
    }
}
