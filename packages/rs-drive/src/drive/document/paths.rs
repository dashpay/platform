use crate::drive::{constants, RootTree};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "server")]
use grovedb::batch::key_info::KeyInfo;
#[cfg(feature = "server")]
use grovedb::batch::KeyInfoPath;

#[cfg(any(feature = "server", feature = "verify"))]
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

#[cfg(any(feature = "server", feature = "verify"))]
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

#[cfg(any(feature = "server", feature = "verify"))]
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

#[cfg(any(feature = "server", feature = "verify"))]
/// Returns the path to a contract document.
pub fn contract_documents_keeping_history_primary_key_path_for_document_id<'a>(
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

#[cfg(feature = "server")]
/// Returns the path to a contract document when the document id isn't known.
pub fn contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
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

#[cfg(any(feature = "server", feature = "verify"))]
#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
/// Returns the size of the path to a contract document.
fn contract_documents_keeping_history_primary_key_path_for_document_id_size(
    document_type_name_len: u32,
) -> u32 {
    constants::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE
        + document_type_name_len
}

#[cfg(any(feature = "server", feature = "verify"))]
/// Returns the size of the path to the time at which a document type was stored.
pub fn contract_documents_keeping_history_storage_time_reference_path_size(
    document_type_name_len: u32,
) -> u32 {
    constants::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH
        + document_type_name_len
}
