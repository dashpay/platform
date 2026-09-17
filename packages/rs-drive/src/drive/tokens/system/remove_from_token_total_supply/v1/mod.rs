use crate::drive::balances::{total_tokens_root_supply_path, total_tokens_root_supply_path_vec};
use crate::drive::tokens::lifecycle::add_to_contract_issued_supply::IssuedSupplyChange;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element::SumItem;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Generation 1: the same supply write as v0, plus the issuer's lifecycle rollup lowered
    /// by the same amount, in the same batch.
    pub(super) fn remove_from_token_total_supply_v1(
        &self,
        token_id: [u8; 32],
        amount: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.remove_from_token_total_supply_add_to_operations_v1(
            token_id,
            amount,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok(fees)
    }

    pub(super) fn remove_from_token_total_supply_add_to_operations_v1(
        &self,
        token_id: [u8; 32],
        amount: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.remove_from_token_total_supply_operations_v1(
            token_id,
            amount,
            &mut None,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    pub(super) fn remove_from_token_total_supply_operations_v1(
        &self,
        token_id: [u8; 32],
        amount: u64,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        // If we only estimate, add estimation costs
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_total_supply(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::BigSumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let path_holding_total_token_supply = total_tokens_root_supply_path();
        let total_token_supply_in_platform = self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path_holding_total_token_supply).into(),
            &token_id,
            direct_query_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let new_total = if estimated_costs_only_with_layer_info.is_none() {
            let total_token_supply_in_platform = total_token_supply_in_platform.ok_or(
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "Total supply for Token not found in Platform for token {}",
                    Identifier::from(token_id)
                ))),
            )?;

            total_token_supply_in_platform.checked_sub(amount)
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    format!("trying to subtract an amount {} from current amount {} that would underflow total supply for token {}", amount, total_token_supply_in_platform, Identifier::from(token_id)),
                )))?
        } else {
            u64::MAX // This would error if we were not in estimated costs, which is what we want
        };

        let path_holding_total_token_supply_vec = total_tokens_root_supply_path_vec();
        let replace_op = QualifiedGroveDbOp::replace_op(
            path_holding_total_token_supply_vec,
            token_id.to_vec(),
            SumItem(new_total as i64, None),
        );
        drive_operations.push(GroveOperation(replace_op));

        // The issuer's rollup moves by the same amount in the same batch; a destroyed issuer
        // is refused inside.
        drive_operations.extend(self.add_to_contract_issued_supply_operations(
            token_id,
            IssuedSupplyChange::Decrease(amount),
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::version::PlatformVersion;

    fn setup_token_with_supply(initial_supply: u64) -> (Drive, Identifier, [u8; 32]) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [1u8; 32];
        let contract_id = Identifier::from([3u8; 32]);

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        if initial_supply > 0 {
            drive
                .add_to_token_total_supply(
                    token_id,
                    initial_supply,
                    false,
                    false,
                    true,
                    &block_info,
                    None,
                    platform_version,
                )
                .expect("expected to seed supply");
        }

        (drive, contract_id, token_id)
    }

    fn rollup(drive: &Drive, contract_id: Identifier) -> u128 {
        drive
            .fetch_contract_token_lifecycle(
                contract_id.to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to read")
            .expect("expected a record")
            .issued_supply()
    }

    #[test]
    fn should_lower_the_supply_and_the_rollup_together() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract_id, token_id) = setup_token_with_supply(1000);

        drive
            .remove_from_token_total_supply(
                token_id,
                300,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to remove from total supply");

        assert_eq!(
            drive
                .fetch_token_total_supply(token_id, None, platform_version)
                .expect("expected to fetch supply"),
            Some(700)
        );
        assert_eq!(rollup(&drive, contract_id), 700);
    }

    #[test]
    fn should_error_on_underflow_and_leave_the_rollup_untouched() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract_id, token_id) = setup_token_with_supply(100);

        let result = drive.remove_from_token_total_supply(
            token_id,
            200,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
        assert_eq!(rollup(&drive, contract_id), 100);
    }

    #[test]
    fn should_refuse_a_destroyed_issuer() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract_id, token_id) = setup_token_with_supply(100);
        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let result = drive.remove_from_token_total_supply(
            token_id,
            1,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
        assert_eq!(
            drive
                .fetch_token_total_supply(token_id, None, platform_version)
                .expect("expected to fetch supply"),
            Some(100)
        );
    }

    #[test]
    fn should_estimate_costs_without_mutating_state_when_apply_false() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract_id, token_id) = setup_token_with_supply(5_000);
        let root_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        let estimated = drive
            .remove_from_token_total_supply(
                token_id,
                100,
                &BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("expected estimation to succeed");

        assert!(estimated.processing_fee > 0);
        let root_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");
        assert_eq!(root_hash_before, root_hash_after);
        assert_eq!(rollup(&drive, contract_id), 5_000);

        let applied = drive
            .remove_from_token_total_supply(
                token_id,
                100,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to apply");
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "estimated {} is below applied {}",
            estimated.processing_fee,
            applied.processing_fee
        );
    }
}
