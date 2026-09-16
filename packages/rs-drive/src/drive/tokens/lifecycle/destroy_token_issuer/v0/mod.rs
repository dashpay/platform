use crate::drive::tokens::lifecycle::encode_destroyed_supply;
use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::paths::{
    token_contract_lifecycles_root_path_vec, TOKEN_DESTROYED_SUPPLY_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
use dpp::tokens::contract_lifecycle::{ContractTokenLifecycle, ContractWipe};
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn destroy_token_issuer_v0(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.destroy_token_issuer_add_to_operations_v0(
            contract_id,
            block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
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

    pub(super) fn destroy_token_issuer_add_to_operations_v0(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.destroy_token_issuer_operations_v0(
            contract_id,
            block_info,
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

    /// Two reads (the record and the destroyed supply scalar) and two writes, whatever the
    /// issuer holds: the destroyed supply is the record's rollup, so no token and no holder
    /// is visited.
    pub(super) fn destroy_token_issuer_operations_v0(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let apply = estimated_costs_only_with_layer_info.is_none();

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_contract_lifecycles(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let record = self.fetch_contract_token_lifecycle_operations(
            contract_id,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(
                    crate::drive::tokens::lifecycle::TOKEN_DESTROYED_SUPPLY_SIZE as u32,
                ),
            }
        };
        let destroyed_supply = self.fetch_token_destroyed_supply_operations(
            direct_query_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let lifecycles_path = token_contract_lifecycles_root_path_vec();

        if !apply {
            drive_operations.push(
                LowLevelDriveOperation::replace_for_estimated_path_key_element(
                    KeyInfoPath::from_known_owned_path(lifecycles_path.clone()),
                    KeyInfo::MaxKeySize {
                        unique_id: contract_id.to_vec(),
                        max_size: 32,
                    },
                    Element::new_item(vec![
                        0;
                        ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES as usize
                    ]),
                ),
            );
            drive_operations.push(
                LowLevelDriveOperation::replace_for_estimated_path_key_element(
                    KeyInfoPath::from_known_owned_path(lifecycles_path),
                    KeyInfo::KnownKey(TOKEN_DESTROYED_SUPPLY_KEY.to_vec()),
                    Element::new_item(encode_destroyed_supply(0)),
                ),
            );
            return Ok(drive_operations);
        }

        // A contract that issues no tokens has no record yet; its destruction still leaves a
        // permanent marker so the contract's storage principal is known to be wiped.
        let (mut record, record_exists) = match record {
            Some(record) => (record, true),
            None => (ContractTokenLifecycle::new(0, platform_version)?, false),
        };

        if record.is_wiped() {
            return Err(Error::Drive(DriveError::TokenIssuerAlreadyDestroyed(
                Identifier::from(contract_id),
            )));
        }

        let destroyed_supply = destroyed_supply.ok_or_else(|| {
            Error::Drive(DriveError::CorruptedDriveState(
                "the destroyed token supply ledger is missing".to_string(),
            ))
        })?;
        let new_destroyed_supply = destroyed_supply
            .checked_add(record.issued_supply())
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "the destroyed token supply ledger overflowed".to_string(),
                ))
            })?;

        record.set_wiped(ContractWipe::new(
            block_info.height,
            block_info.time_ms,
            platform_version,
        )?);
        let record_element = Element::new_item(record.serialize_consume_to_bytes()?);

        if record_exists {
            drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                lifecycles_path.clone(),
                contract_id.to_vec(),
                record_element,
            ));
        } else {
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                lifecycles_path.clone(),
                contract_id.to_vec(),
                record_element,
            ));
        }

        drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
            lifecycles_path,
            TOKEN_DESTROYED_SUPPLY_KEY.to_vec(),
            Element::new_item(encode_destroyed_supply(new_destroyed_supply)),
        ));

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::tokens::contract_lifecycle::v0::ContractWipeV0Accessors;

    fn create_token(drive: &Drive, contract_id: Identifier, position: u16, token_id: [u8; 32]) {
        drive
            .create_token_trees(
                contract_id,
                position,
                token_id,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to create token trees");
    }

    fn mint(drive: &Drive, token_id: [u8; 32], identity_id: [u8; 32], amount: u64) {
        drive
            .token_mint(
                token_id,
                identity_id,
                amount,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to mint");
    }

    #[test]
    fn should_move_the_rollup_into_the_destroyed_supply_and_keep_raw_totals() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        let other_contract_id = Identifier::from([4u8; 32]);
        let token_a = [1u8; 32];
        let token_b = [2u8; 32];
        let other_token = [5u8; 32];
        create_token(&drive, contract_id, 0, token_a);
        create_token(&drive, contract_id, 1, token_b);
        create_token(&drive, other_contract_id, 0, other_token);
        mint(&drive, token_a, [10u8; 32], 300);
        mint(&drive, token_b, [11u8; 32], 700);
        mint(&drive, other_token, [12u8; 32], 55);

        let before = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert!(before.ok().expect("expected a verdict"));
        assert_eq!(before.total_destroyed_supply, 0);

        let block_info = BlockInfo {
            height: 12,
            time_ms: 34,
            ..Default::default()
        };
        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let after = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert!(after.ok().expect("expected a verdict"));
        assert_eq!(
            after.total_tokens_in_platform,
            before.total_tokens_in_platform
        );
        assert_eq!(
            after.total_identity_token_balances,
            before.total_identity_token_balances
        );
        assert_eq!(after.total_destroyed_supply, 1_000);
        assert_eq!(
            after.active_supply().expect("expected an active supply"),
            55
        );

        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 1_000);
        let wipe = record.wiped().expect("expected the wipe marker");
        assert_eq!(wipe.block_height(), 12);
        assert_eq!(wipe.block_time_ms(), 34);

        // Neither the supply leaves nor the holder leaves moved.
        assert_eq!(
            drive
                .fetch_token_total_supply(token_a, None, platform_version)
                .expect("expected the supply"),
            Some(300)
        );
        assert_eq!(
            drive
                .fetch_identity_token_balance(token_b, [11u8; 32], None, platform_version)
                .expect("expected the balance"),
            Some(700)
        );
    }

    #[test]
    fn should_destroy_only_once() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        create_token(&drive, contract_id, 0, [1u8; 32]);

        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let result = drive.destroy_token_issuer(
            contract_id.to_buffer(),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::TokenIssuerAlreadyDestroyed(id))) if id == contract_id
        ));
    }

    #[test]
    fn should_record_a_contract_without_tokens_as_wiped() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = [8u8; 32];

        drive
            .destroy_token_issuer(
                contract_id,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the contract");

        let record = drive
            .fetch_contract_token_lifecycle(contract_id, None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 0);
        assert!(record.is_wiped());
        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert_eq!(totals.total_destroyed_supply, 0);
    }

    #[test]
    fn should_price_the_destruction_without_touching_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        create_token(&drive, contract_id, 0, [1u8; 32]);
        let root_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash");

        let fees = drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("expected an estimate");

        assert!(fees.processing_fee > 0);
        let root_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash");
        assert_eq!(root_hash_before, root_hash_after);
        assert!(!drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record")
            .is_wiped());
    }

    #[test]
    fn should_not_be_active_before_the_ledger_exists() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive.destroy_token_issuer(
            [1u8; 32],
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}
