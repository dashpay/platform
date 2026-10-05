use crate::drive::contract::moderation::types::encode_document_removal;
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
    /// record passes to the moderator that replaced it, who pays for the bytes it adds; a
    /// shorter one refunds the bytes it frees to the moderator the record named, and passes to
    /// the one that replaced it only when written in the epoch the record was paid in, or once
    /// the record spans epochs, a record paid in one epoch keeping its flags, and so its
    /// earlier moderator, when shortened in a later one; an equally long one keeps the earlier
    /// moderator's flags.
    ///
    /// An estimate prices a replacement as a fresh insert of what it adds to the record it
    /// replaces (`replaced_record_size`), so never less than the bytes it adds: GroveDB's
    /// average-case replace assumes an item keeps its size and would price no storage for the
    /// restoration a restore adds, which the moderator's balance is then not checked against,
    /// and pricing the whole record would charge the estimate for the reason and the kept
    /// fields the record already holds.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_document_removal_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        removal: &ContractDocumentRemoval,
        replaced_record_size: Option<u32>,
        estimated_kept_fields_size: u32,
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
            Drive::add_estimation_costs_for_contract_document_removal(
                contract_id.to_buffer(),
                document_type_name,
                estimated_kept_fields_size,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(moderator_id.to_buffer()));

        let mut value = encode_document_removal(removal).map_err(lost_bound)?;
        if let (true, Some(replaced_record_size)) = (estimating, replaced_record_size) {
            let added = value
                .len()
                .saturating_sub(usize::try_from(replaced_record_size).unwrap_or(usize::MAX));
            value = vec![0; added];
        }
        let path_key_element = PathFixedSizeKeyRefElement((
            contract_document_type_removals_path(contract_id.as_slice(), document_type_name),
            document_id.as_slice(),
            Element::new_item_with_flags(value, storage_flags.to_some_element_flags()),
        ));

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if replaced_record_size.is_some() && !estimating {
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
