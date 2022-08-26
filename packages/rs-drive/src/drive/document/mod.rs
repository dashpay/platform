use crate::contract::document::Document;
use crate::drive::flags::StorageFlags;
use crate::drive::{defaults, RootTree};
use dpp::data_contract::extra::DocumentType;
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
use grovedb::Element;

mod delete;
mod insert;
mod update;

fn contract_document_type_path<'a>(
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

fn contract_documents_primary_key_path<'a>(
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

fn contract_documents_keeping_history_primary_key_path_for_document_id_size(
    document_type_name_len: usize,
) -> usize {
    defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE
        + document_type_name_len
}

fn contract_documents_keeping_history_storage_time_reference_path_size(
    document_type_name_len: usize,
) -> usize {
    defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH
        + document_type_name_len
}

fn make_document_reference(
    document: &Document,
    document_type: &DocumentType,
    storage_flags: &StorageFlags,
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
        storage_flags.to_element_flags(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use std::option::Option::None;

    use tempfile::TempDir;

    use crate::common::json_document_to_cbor;
    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;

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
                0f64,
                true,
                StorageFlags::default(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, dashpay_cbor)
    }
}
