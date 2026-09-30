use crate::drive::contract::moderation::types::{
    document_removal_kept_fields_encoded_size, encode_document_removal,
};
use crate::drive::contract::paths::contract_document_type_removals_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
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
    /// the document, its restoration if any, the fields its type keeps, and the reason, under
    /// the document's id, flagged
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
        // What the document's type keeps of it was bounded by the document's own size limits,
        // far inside the record's length prefixes, so an encoding refusal is a code path that
        // lost that guarantee, not a moderator's mistake.
        let lost_bound = |_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "the fields a removal record keeps exceed what a record can hold",
            ))
        };
        let estimating = estimated_costs_only_with_layer_info.is_some();
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            let kept_fields_size = document_removal_kept_fields_encoded_size(&removal.kept_fields)
                .map_err(lost_bound)?;
            Drive::add_estimation_costs_for_contract_document_removal(
                contract_id.to_buffer(),
                document_type_name,
                u32::try_from(kept_fields_size).unwrap_or(u32::MAX),
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
                encode_document_removal(removal).map_err(lost_bound)?,
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
