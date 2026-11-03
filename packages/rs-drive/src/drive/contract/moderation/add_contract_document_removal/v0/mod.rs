use crate::drive::contract::moderation::types::encode_document_removal;
use crate::drive::contract::paths::contract_document_type_removals_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::Element;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// A record is the document owner's id, the moderator's id, the removal time, the hash of
    /// the document, its restoration if any, and the reason, under the document's id, flagged
    /// with `moderator_id`: the moderator that writes it pays for it. Nothing ever deletes it.
    /// It is replaced in place, as a suspension is, when a restore marks it restored and when
    /// a restored document is deleted again; two operations on one key would fail the batch.
    /// A replacement may change size, and its flags follow GroveDB's flag merge: a longer
    /// record passes, with the refund of its removal, to the moderator that replaced it, who
    /// pays for the added bytes; a shorter or an equally long one stays the first moderator's.
    ///
    /// An estimate prices a replacement as a fresh insert of the whole record: GroveDB's
    /// average-case replace assumes an item keeps its size and would price no storage for the
    /// restoration a restore adds, which the moderator's balance is then not checked against.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_document_removal_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        removal: &ContractDocumentRemoval,
        replaces_existing: bool,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let estimating = estimated_costs_only_with_layer_info.is_some();
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_document_removal(
                contract_id.to_buffer(),
                document_type_name,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(moderator_id.to_buffer()));

        let path_key_element = PathFixedSizeKeyRefElement((
            contract_document_type_removals_path(contract_id.as_slice(), document_type_name),
            document_id.as_slice(),
            Element::new_item_with_flags(
                encode_document_removal(removal),
                storage_flags.to_some_element_flags(),
            ),
        ));

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if replaces_existing && !estimating {
            self.batch_replace(
                path_key_element,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        } else {
            self.batch_insert(
                path_key_element,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(batch_operations)
    }
}
