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
    /// A record is the document owner's id, the moderator's id, the removal time and the
    /// reason, under the document's id, flagged with the moderator's identity: the moderator
    /// pays for it. Nothing ever deletes it, and nothing replaces it: a document id is produced
    /// at most once.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_document_removal_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        removal: &ContractDocumentRemoval,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_document_removal(
                contract_id.to_buffer(),
                document_type_name,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags = StorageFlags::new_single_epoch(
            block_info.epoch.index,
            Some(removal.moderator_id.to_buffer()),
        );

        let path_key_element = PathFixedSizeKeyRefElement((
            contract_document_type_removals_path(contract_id.as_slice(), document_type_name),
            document_id.as_slice(),
            Element::new_item_with_flags(
                encode_document_removal(removal),
                storage_flags.to_some_element_flags(),
            ),
        ));

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        self.batch_insert(
            path_key_element,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
