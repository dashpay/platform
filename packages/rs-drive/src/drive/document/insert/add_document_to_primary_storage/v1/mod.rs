use crate::drive::constants::DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE;
use crate::drive::document::paths::{
    contract_documents_primary_key_path, DOCUMENT_HISTORY_TREE_KEY,
};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::read_document_sum_contribution;
use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType, QueryTarget};
use crate::util::object_size_info::DocumentInfo;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathKeyElement, PathKeyUnknownElementSize,
};
use crate::util::object_size_info::PathKeyInfo::PathKey;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::{key_info::KeyInfo, KeyInfoPath};
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Stores immutable revisions separately from the current primary-key entry.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_document_to_primary_storage_v1(
        &self,
        info: &DocumentAndContractInfo,
        block_info: &BlockInfo,
        insert_without_check: bool,
        estimated: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
        transaction: TransactionArg,
        operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !info.document_type.documents_keep_history() {
            return self.add_document_to_primary_storage_0(
                info,
                block_info,
                insert_without_check,
                estimated,
                transaction,
                operations,
                platform_version,
            );
        }
        let version = &platform_version.drive;
        let primary_path = contract_documents_primary_key_path(
            info.contract.id_ref().as_bytes(),
            info.document_type.name(),
        );
        if let Some(layers) = estimated {
            Self::add_estimation_costs_for_add_document_to_primary_storage(
                info,
                primary_path,
                layers,
                platform_version,
            )?;
        }
        let document_info = &info.owned_document_info.document_info;
        let document = document_info.get_borrowed_document();
        let document_key = document.map_or_else(
            || KeyInfo::MaxKeySize {
                unique_id: info.document_type.unique_id_for_storage().to_vec(),
                max_size: 32,
            },
            |document| KeyInfo::KnownKey(document.id().to_vec()),
        );
        let mut history_root = primary_path.map(|part| part.to_vec()).to_vec();
        history_root[4] = vec![DOCUMENT_HISTORY_TREE_KEY];
        let mut history_path = KeyInfoPath::from_known_path(history_root.iter().map(Vec::as_slice));
        history_path.push(document_key.clone());
        let flags = document_info.get_storage_flags_ref();
        let tree_apply = if estimated.is_some() {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::ProvableCountTree,
                flags_len: flags.map(StorageFlags::serialized_size).unwrap_or_default(),
            }
        } else {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        };
        if let Some(document) = document {
            let created = self.batch_insert_empty_tree_if_not_exists::<0>(
                PathKey((history_root, document.id().to_vec())),
                TreeType::ProvableCountTree,
                flags,
                tree_apply,
                transaction,
                &mut None,
                operations,
                version,
            )?;
            // An id whose revisions are still retained is taken, even though
            // nothing in the primary-key tree says so. Appending to that
            // history would silently merge a new document into a deleted one's
            // record, so it is refused here rather than only in transition
            // validation: this is the guard that covers writers outside it.
            // A dry run skips the probe, which reports the tree as absent, and
            // pays for it as a fixed cost so estimation and execution agree.
            if !created && !insert_without_check {
                return Err(Error::Drive(DriveError::CorruptedDocumentAlreadyExists(
                    "a document of this id still retains revisions and can not be \
                         created until they are erased",
                )));
            }
        } else {
            operations.push(
                LowLevelDriveOperation::for_estimated_path_key_empty_provable_count_tree(
                    KeyInfoPath::from_known_owned_path(history_root),
                    document_key.clone(),
                    flags,
                ),
            );
        }

        let mut revision_key = encode_u64(block_info.time_ms);
        revision_key.extend(encode_u64(document.and_then(|d| d.revision()).unwrap_or(1)));
        let summable = info.document_type.documents_summable();
        let (revision, pointer) = if let Some(document) = document {
            let bytes = match document_info {
                DocumentInfo::DocumentRefAndSerialization((_, bytes, _)) => bytes.to_vec(),
                DocumentInfo::DocumentAndSerialization((_, bytes, _)) => bytes.clone(),
                _ => document.serialize(info.document_type, info.contract, platform_version)?,
            };
            let element_flags = StorageFlags::map_to_some_element_flags(flags);
            let reference = UpstreamRootHeightReference(
                4,
                vec![
                    vec![DOCUMENT_HISTORY_TREE_KEY],
                    document.id().to_vec(),
                    revision_key.clone(),
                ],
            );
            let pointer = if let Some(property) = summable {
                Element::new_reference_with_sum_item_with_max_hops_and_flags(
                    reference,
                    Some(1),
                    read_document_sum_contribution(document, property)?,
                    element_flags.clone(),
                )
            } else {
                Element::Reference(reference, Some(1), element_flags.clone())
            };
            let mut known_history = primary_path.map(|part| part.to_vec()).to_vec();
            known_history[4] = vec![DOCUMENT_HISTORY_TREE_KEY];
            known_history.push(document.id().to_vec());
            (
                PathKeyElement((
                    known_history,
                    revision_key,
                    Element::Item(bytes, element_flags),
                )),
                PathKeyElement((
                    primary_path.map(|part| part.to_vec()).to_vec(),
                    document.id().to_vec(),
                    pointer,
                )),
            )
        } else {
            let DocumentInfo::DocumentEstimatedAverageSize(size) = document_info else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "missing document size",
                )));
            };
            let flags_size = flags.map(StorageFlags::serialized_size).unwrap_or_default();
            let pointer_size = if summable.is_some() {
                Element::required_reference_with_sum_item_space(
                    DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE,
                    flags_size,
                    &version.grove_version,
                )?
            } else {
                Element::required_item_space(
                    DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE,
                    flags_size,
                    &version.grove_version,
                )?
            };
            (
                PathKeyUnknownElementSize((
                    history_path,
                    KeyInfo::KnownKey(revision_key),
                    Element::required_item_space(*size, flags_size, &version.grove_version)?,
                )),
                PathKeyUnknownElementSize((
                    KeyInfoPath::from_known_path(primary_path),
                    document_key,
                    pointer_size,
                )),
            )
        };
        self.batch_insert::<0>(revision, operations, version)?;
        if insert_without_check {
            self.batch_insert::<0>(pointer, operations, version)?;
        } else {
            let apply_type = if estimated.is_some() {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: info.document_type.primary_key_tree_type(platform_version)?,
                    target: QueryTarget::QueryTargetValue(64),
                }
            } else {
                BatchInsertApplyType::StatefulBatchInsert
            };
            if !self.batch_insert_if_not_exists::<0>(
                pointer,
                apply_type,
                transaction,
                operations,
                version,
            )? {
                return Err(Error::Drive(DriveError::CorruptedDocumentAlreadyExists(
                    "item already exists",
                )));
            }
        }
        Ok(())
    }
}
