use crate::drive::{constants, RootTree};

/// Reserved document-type key containing the revision trees.
pub const DOCUMENT_HISTORY_TREE_KEY: u8 = 2;

/// Where a keep-history document type stores its retained revisions.
///
/// Decoded from the primary-storage writer's method version so every path,
/// reference and query agrees with the writer that produced the stored state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepHistoryStorage {
    /// Every revision lives in the document's own subtree under the primary
    /// key tree, keyed by block time, with the current revision at `[0]`.
    DocumentSubtree,
    /// The primary key tree points at the current revision; the revisions
    /// live in the type's history tree under `DOCUMENT_HISTORY_TREE_KEY`.
    HistoryTree,
}

impl KeepHistoryStorage {
    /// The layout selected by the primary-storage writer at this drive version.
    pub fn for_drive_version(
        drive_version: &dpp::version::drive_versions::DriveVersion,
    ) -> Result<Self, crate::error::Error> {
        match drive_version
            .methods
            .document
            .insert
            .add_document_to_primary_storage
        {
            0 => Ok(Self::DocumentSubtree),
            1 => Ok(Self::HistoryTree),
            received => Err(crate::error::Error::Drive(
                crate::error::drive::DriveError::UnknownVersionMismatch {
                    method: "add_document_to_primary_storage".to_string(),
                    known_versions: vec![0, 1],
                    received,
                },
            )),
        }
    }
}

/// Path to all retained revisions of one document.
pub fn document_history_path(
    contract_id: &[u8],
    document_type_name: &str,
    document_id: &[u8],
) -> Vec<Vec<u8>> {
    let mut path = contract_document_type_path_vec(contract_id, document_type_name);
    path.extend([vec![DOCUMENT_HISTORY_TREE_KEY], document_id.to_vec()]);
    path
}
#[cfg(feature = "server")]
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
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

#[cfg(test)]
mod tests {
    use super::KeepHistoryStorage;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_reject_unknown_primary_storage_writer_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .document
            .insert
            .add_document_to_primary_storage = 2;

        let error = KeepHistoryStorage::for_drive_version(&platform_version.drive)
            .expect_err("an unknown writer version must not imply a storage layout");

        assert!(matches!(
            error,
            Error::Drive(DriveError::UnknownVersionMismatch {
                method,
                known_versions,
                received: 2,
            }) if method == "add_document_to_primary_storage" && known_versions == vec![0, 1]
        ));
    }
}
