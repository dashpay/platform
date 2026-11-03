use crate::drive::contract::moderation::types::encode_warnings;
use crate::drive::contract::paths::contract_moderation_list_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::{ContractModerationList, ContractWarning};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::Element;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_warning_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        warnings: &[ContractWarning],
        replaces_existing: bool,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_contract_warning_operations_v0(
            contract_id,
            identity_id,
            warnings,
            replaces_existing,
            moderator_id,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// A warning list entry is every warning the identity carries, oldest first, each its
    /// block time and its reason (see [`encode_warnings`]), under the identity's id, flagged
    /// with the moderator's identity so the storage refund on removal goes back to whoever
    /// paid. A warn on an identity that already carries warnings replaces the entry in place
    /// with one warning more; two operations on one key would fail the batch. The entry is
    /// then longer, so its flags follow GroveDB's flag merge for a grown item: it passes, with
    /// the refund of its removal, to the moderator that warned last, who pays for the bytes
    /// the warning added.
    ///
    /// An estimate prices the replacement as a fresh insert. GroveDB's average-case replace
    /// assumes an item keeps its size and would price no storage for the added warning, which
    /// the moderator's balance is then not checked against; the whole entry is an upper bound.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_warning_operations_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        warnings: &[ContractWarning],
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
            Drive::add_estimation_costs_for_contract_moderation_entry(
                contract_id.to_buffer(),
                ContractModerationList::Warnings,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(moderator_id.to_buffer()));

        let path_key_element = PathFixedSizeKeyRefElement((
            contract_moderation_list_path(contract_id.as_slice(), ContractModerationList::Warnings),
            identity_id.as_slice(),
            Element::new_item_with_flags(
                // Every reason was bounded by basic structure validation, so an encoding
                // refusal is a code path that lost that guarantee, not a moderator's mistake.
                encode_warnings(warnings).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "a warning's reason is longer than a warning list entry can hold",
                    ))
                })?,
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
