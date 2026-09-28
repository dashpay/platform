use crate::drive::contract::moderation::types::encode_suspension;
use crate::drive::contract::paths::contract_moderation_list_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::{ContractModerationList, ContractModerationReason};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::Element;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_suspension_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
        reason: &ContractModerationReason,
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

        let batch_operations = self.add_contract_suspension_operations_v0(
            contract_id,
            identity_id,
            until,
            reason,
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

    /// A suspension is `until` as eight big-endian bytes, then its reason, under the identity's
    /// id, flagged with the moderator's identity so the storage refund on removal goes back to
    /// whoever paid. An existing entry is replaced in place; two operations on one key would
    /// fail the batch. The replacement brings its own reason, so the entry may change size, and
    /// its flags follow GroveDB's flag merge: a longer entry passes, with the refund of its
    /// removal, to the moderator that replaced it, who pays for the added bytes; a shorter or
    /// an equally long one stays the first moderator's, who is refunded the removed bytes.
    ///
    /// An estimate prices the replacement as a fresh insert. GroveDB's average-case replace
    /// assumes an item keeps its size and would price no storage for a longer reason, which
    /// the moderator's balance is then not checked against; the whole entry is an upper bound.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_suspension_operations_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
        reason: &ContractModerationReason,
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
                ContractModerationList::Suspensions,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(moderator_id.to_buffer()));

        let path_key_element = PathFixedSizeKeyRefElement((
            contract_moderation_list_path(
                contract_id.as_slice(),
                ContractModerationList::Suspensions,
            ),
            identity_id.as_slice(),
            Element::new_item_with_flags(
                encode_suspension(until, reason),
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
