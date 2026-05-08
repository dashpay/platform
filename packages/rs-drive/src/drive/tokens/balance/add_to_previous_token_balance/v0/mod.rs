use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use crate::drive::tokens::paths::token_balances_path_vec;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds specified amount of credits to identity balance
    /// This function checks for overflows and does not exceed MAX_CREDITS
    ///
    /// Balances are stored in the balance tree under the identity's id
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::tokens) fn add_to_identity_token_balance_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_add: Credits,
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

        let batch_operations = self.add_to_identity_token_balance_operations_v0(
            token_id,
            identity_id,
            balance_to_add,
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

    /// Adds specified amount of credits to identity balance
    /// This function checks for overflows and does not exceed MAX_CREDITS
    ///
    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(in crate::drive::tokens) fn add_to_identity_token_balance_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_add: TokenAmount,
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
        } else {
            None // worse case is that we insert
        };

        let balance_path = token_balances_path_vec(token_id);

        if let Some(previous_balance) = previous_balance {
            if balance_to_add > i64::MAX as u64 {
                return Err(
                    ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                        "Token balance to add over i64 max, is {}",
                        balance_to_add
                    ))
                    .into(),
                );
            }
            // Check for overflow
            let new_balance = (previous_balance as i64)
                .checked_add(balance_to_add as i64)
                .ok_or(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                    "Overflow of total token balance".to_string(),
                ))?;

            drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(new_balance),
            ));
        } else {
            if balance_to_add > i64::MAX as u64 {
                return Err(
                    ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                        "Token balance to add over i64 max, is {}",
                        balance_to_add
                    ))
                    .into(),
                );
            }
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(balance_to_add as i64),
            ));
        }

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    mod add_to_identity_token_balance {
        use super::*;

        #[test]
        fn should_error_when_balance_to_add_exceeds_i64_max_on_existing_balance() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [1u8; 32];
            let identity_id = [2u8; 32];
            let contract_id = Identifier::from([3u8; 32]);

            // Create the token tree structure first
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

            // Insert an initial balance via add_to_identity_token_balance (insert path)
            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    1000,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to add initial token balance");

            // Now try to add a value that exceeds i64::MAX to the existing balance (add path).
            // Before the fix, this would silently wrap to a negative value.
            let overflow_amount: u64 = i64::MAX as u64 + 1;

            let result = drive.add_to_identity_token_balance(
                token_id,
                identity_id,
                overflow_amount,
                &block_info,
                true,
                None,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "expected an error for balance_to_add > i64::MAX but got Ok"
            );
            let err_string = format!("{}", result.unwrap_err());
            assert!(
                err_string.contains("Token balance to add over i64 max"),
                "expected CriticalCorruptedCreditsCodeExecution error, got: {}",
                err_string
            );
        }

        #[test]
        fn should_error_when_insert_path_balance_to_add_exceeds_i64_max() {
            // Insert path: identity has no existing balance entry.
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [40u8; 32];
            let identity_id = [41u8; 32];
            let contract_id = Identifier::from([42u8; 32]);

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

            let overflow_amount: u64 = i64::MAX as u64 + 1;

            let result = drive.add_to_identity_token_balance(
                token_id,
                identity_id,
                overflow_amount,
                &block_info,
                true,
                None,
                platform_version,
                None,
            );

            assert!(
                result.is_err(),
                "expected CriticalCorruptedCreditsCodeExecution on insert path overflow"
            );
            let err_string = format!("{}", result.unwrap_err());
            assert!(
                err_string.contains("Token balance to add over i64 max"),
                "unexpected error: {}",
                err_string
            );
        }

        #[test]
        fn should_successfully_insert_initial_balance_at_i64_max() {
            // Insert path boundary: balance_to_add == i64::MAX (allowed)
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [43u8; 32];
            let identity_id = [44u8; 32];
            let contract_id = Identifier::from([45u8; 32]);

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

            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    i64::MAX as u64,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected boundary insert to succeed");

            let balance = drive
                .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
                .expect("expected to fetch balance");
            assert_eq!(balance, Some(i64::MAX as u64));
        }

        #[test]
        fn should_error_on_sum_overflow_when_adding_to_large_existing_balance() {
            // checked_add overflow branch: previous_balance + balance_to_add overflows i64
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [46u8; 32];
            let identity_id = [47u8; 32];
            let contract_id = Identifier::from([48u8; 32]);

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

            // Seed with near-max balance
            let seed = (i64::MAX as u64) - 5;
            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    seed,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to seed balance");

            // Now add 100 — should overflow and return an error via checked_add
            let result = drive.add_to_identity_token_balance(
                token_id,
                identity_id,
                100,
                &block_info,
                true,
                None,
                platform_version,
                None,
            );

            assert!(result.is_err(), "expected checked_add overflow error");
            let err_string = format!("{}", result.unwrap_err());
            assert!(
                err_string.contains("Overflow of total token balance"),
                "unexpected error: {}",
                err_string
            );
        }

        #[test]
        fn should_estimate_costs_without_mutating_state_when_apply_false() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [49u8; 32];
            let identity_id = [50u8; 32];
            let contract_id = Identifier::from([51u8; 32]);

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

            let app_hash_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected root hash");

            // apply=false: estimation branch, no state change
            let fee_result = drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    777,
                    &block_info,
                    false,
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

            assert_eq!(
                app_hash_before, app_hash_after,
                "estimation must not mutate state"
            );
            assert!(fee_result.processing_fee > 0);

            // State was untouched
            let balance = drive
                .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
                .expect("expected to fetch balance");
            assert_eq!(balance, None);
        }

        #[test]
        fn should_successfully_add_zero_amount() {
            // Edge case: adding zero should be a legal no-op on the insert path
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [52u8; 32];
            let identity_id = [53u8; 32];
            let contract_id = Identifier::from([54u8; 32]);

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

            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    0,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to add zero balance");

            let balance = drive
                .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
                .expect("expected to fetch balance");
            assert_eq!(balance, Some(0));
        }

        #[test]
        fn should_successfully_add_normal_amount_to_existing_balance() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let block_info = BlockInfo::default();
            let token_id = [1u8; 32];
            let identity_id = [2u8; 32];
            let contract_id = Identifier::from([3u8; 32]);

            // Create the token tree structure first
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

            // Insert an initial balance (insert path)
            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    1000,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to add initial token balance");

            // Add a normal amount to the existing balance (add path) -- should succeed
            let result = drive.add_to_identity_token_balance(
                token_id,
                identity_id,
                500,
                &block_info,
                true,
                None,
                platform_version,
                None,
            );

            assert!(
                result.is_ok(),
                "expected normal addition to succeed, got error: {:?}",
                result.err()
            );

            // Verify the balance is now 1500
            let balance = drive
                .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
                .expect("expected to fetch balance");

            assert_eq!(
                balance,
                Some(1500),
                "expected balance to be 1500 after adding 500 to 1000"
            );
        }
    }
}
