use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use crate::drive::tokens::paths::token_balances_path_vec;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::tokens) fn remove_from_identity_token_balance_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_remove: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.remove_from_identity_token_balance_operations_v0(
            token_id,
            identity_id,
            balance_to_remove,
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
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )?;
        Ok(fees)
    }

    /// Removes specified amount of credits from identity balance
    /// This function doesn't go below nil balance (negative balance)
    ///
    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(in crate::drive::tokens) fn remove_from_identity_token_balance_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_remove: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_balances(
                token_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let apply = estimated_costs_only_with_layer_info.is_none();

        let previous_balance = if apply {
            self.fetch_identity_token_balance_operations(
                token_id,
                identity_id,
                apply,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?
        } else {
            MAX_CREDITS
        };

        // we do not have enough balance
        // there is a part we absolutely need to pay for
        if balance_to_remove > previous_balance {
            return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                format!(
                    "identity with token balance {} does not have the required balance to remove {}",
                    previous_balance, balance_to_remove
                ),
            )));
        }

        let balance_path = token_balances_path_vec(token_id);

        drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item((previous_balance - balance_to_remove) as i64),
        ));

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::identity::IdentityError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    fn setup_token_with_balance(
        initial_balance: u64,
    ) -> (crate::drive::Drive, [u8; 32], [u8; 32], BlockInfo) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [101u8; 32];
        let identity_id = [102u8; 32];
        let contract_id = Identifier::from([103u8; 32]);

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

        if initial_balance > 0 {
            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    initial_balance,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to seed balance");
        }

        (drive, token_id, identity_id, block_info)
    }

    #[test]
    fn should_remove_partial_balance_and_retain_remainder() {
        let platform_version = PlatformVersion::latest();
        let (drive, token_id, identity_id, block_info) = setup_token_with_balance(1_000);

        drive
            .remove_from_identity_token_balance(
                token_id,
                identity_id,
                400,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to remove partial balance");

        let balance = drive
            .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(600));
    }

    #[test]
    fn should_remove_entire_balance_to_zero() {
        let platform_version = PlatformVersion::latest();
        let (drive, token_id, identity_id, block_info) = setup_token_with_balance(500);

        drive
            .remove_from_identity_token_balance(
                token_id,
                identity_id,
                500,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to remove full balance");

        let balance = drive
            .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(0));
    }

    #[test]
    fn should_error_on_underflow_when_removing_more_than_balance() {
        let platform_version = PlatformVersion::latest();
        let (drive, token_id, identity_id, block_info) = setup_token_with_balance(100);

        let result = drive.remove_from_identity_token_balance(
            token_id,
            identity_id,
            200,
            &block_info,
            true,
            None,
            platform_version,
            None,
        );

        assert!(matches!(
            result,
            Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                _
            )))
        ));
    }

    #[test]
    fn should_error_when_identity_has_no_balance_yet() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [110u8; 32];
        let identity_id_without_balance = [111u8; 32];
        let contract_id = Identifier::from([112u8; 32]);

        // Create trees but DO NOT add any balance for identity_id_without_balance
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

        // Attempting to remove when there is no balance entry -> CorruptedCodeExecution
        let result = drive.remove_from_identity_token_balance(
            token_id,
            identity_id_without_balance,
            1,
            &block_info,
            true,
            None,
            platform_version,
            None,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_estimate_costs_only_without_mutating_state_when_apply_false() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [120u8; 32];
        let identity_id = [121u8; 32];
        let contract_id = Identifier::from([122u8; 32]);

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

        // Estimation path: apply=false. No seeded balance necessary because the
        // stateless path assumes MAX_CREDITS when apply is false.
        let app_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        let fee_result = drive
            .remove_from_identity_token_balance_v0(
                token_id,
                identity_id,
                1_000,
                &block_info,
                false, // apply=false, cost-only branch
                None,
                platform_version,
                None,
            )
            .expect("expected estimation to succeed");

        let app_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        // State must not change in estimation mode
        assert_eq!(app_hash_before, app_hash_after);
        assert!(fee_result.processing_fee > 0);
    }
}
