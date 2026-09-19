use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::lifecycle::{
    decode_destroyed_supply, encode_destroyed_supply, pending_ledger_write,
    TOKEN_DESTROYED_SUPPLY_SIZE,
};
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
use dpp::serialization::PlatformDeserializable;
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

    /// Two reads (the record and the destroyed supply scalar) and two writes, whatever the
    /// issuer holds: the destroyed supply is the record's rollup, so no token and no holder
    /// is visited. A write of the record or of the scalar already pending in the batch
    /// lowered so far is the base and its slot is rewritten, so several destructions in one
    /// batch, or a destruction after a supply write of the same issuer, leave one write per
    /// key; a record the batch already wiped is refused like a stored one.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn destroy_token_issuer_operations_v0(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
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

        let lifecycles_path = token_contract_lifecycles_root_path_vec();

        let pending_record = pending_ledger_write(
            previous_batch_operations.as_deref(),
            &lifecycles_path,
            &contract_id,
        )?;
        let pending_scalar = pending_ledger_write(
            previous_batch_operations.as_deref(),
            &lifecycles_path,
            &TOKEN_DESTROYED_SUPPLY_KEY,
        )?;

        let record = match &pending_record {
            Some(pending) if apply => Some(ContractTokenLifecycle::deserialize_from_bytes(
                pending.bytes.as_slice(),
            )?),
            _ => self.fetch_contract_token_lifecycle_operations(
                contract_id,
                apply,
                transaction,
                &mut drive_operations,
                platform_version,
            )?,
        };

        let destroyed_supply = match &pending_scalar {
            Some(pending) if apply => {
                Some(decode_destroyed_supply(&pending.bytes).ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "a pending destroyed token supply write has the wrong size",
                    ))
                })?)
            }
            _ => {
                let direct_query_type = if apply {
                    DirectQueryType::StatefulDirectQuery
                } else {
                    DirectQueryType::StatelessDirectQuery {
                        in_tree_type: TreeType::NormalTree,
                        query_target: QueryTargetValue(TOKEN_DESTROYED_SUPPLY_SIZE as u32),
                    }
                };
                self.fetch_token_destroyed_supply_operations(
                    direct_query_type,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?
            }
        };

        if !apply {
            // An insert rather than a replace: the estimator charges a replace no storage,
            // while the real record grows by the wipe marker, and the estimate has to cover
            // the applied cost. The scalar keeps its 16 bytes, so its replace is exact.
            drive_operations.push(
                LowLevelDriveOperation::insert_for_estimated_path_key_element(
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
        // A record the batch inserts is not stored yet, so its rewrite stays an insert.
        let record_exists = record_exists && !pending_record.as_ref().is_some_and(|p| p.inserts);

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

        let record_write = if record_exists {
            LowLevelDriveOperation::replace_for_known_path_key_element(
                lifecycles_path.clone(),
                contract_id.to_vec(),
                record_element,
            )
        } else {
            LowLevelDriveOperation::insert_for_known_path_key_element(
                lifecycles_path.clone(),
                contract_id.to_vec(),
                record_element,
            )
        };
        let scalar_write = LowLevelDriveOperation::replace_for_known_path_key_element(
            lifecycles_path,
            TOKEN_DESTROYED_SUPPLY_KEY.to_vec(),
            Element::new_item(encode_destroyed_supply(new_destroyed_supply)),
        );

        match (pending_record, previous_batch_operations.as_deref_mut()) {
            (Some(pending), Some(operations)) => operations[pending.index] = record_write,
            _ => drive_operations.push(record_write),
        }
        match (pending_scalar, previous_batch_operations.as_deref_mut()) {
            (Some(pending), Some(operations)) => operations[pending.index] = scalar_write,
            _ => drive_operations.push(scalar_write),
        }

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

        drive.assert_token_rollups_consistent(None, platform_version);
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

    fn ledger_writes(batch: &[LowLevelDriveOperation]) -> usize {
        batch
            .iter()
            .filter(|operation| match operation {
                LowLevelDriveOperation::GroveOperation(grove_op) => {
                    grove_op.path == token_contract_lifecycles_root_path_vec()
                }
                _ => false,
            })
            .count()
    }

    #[test]
    fn should_accumulate_the_destroyed_supply_of_two_issuers_in_one_batch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let first_issuer = Identifier::from([3u8; 32]);
        let second_issuer = Identifier::from([4u8; 32]);
        create_token(&drive, first_issuer, 0, [1u8; 32]);
        create_token(&drive, second_issuer, 0, [2u8; 32]);
        mint(&drive, [1u8; 32], [10u8; 32], 100);
        mint(&drive, [2u8; 32], [11u8; 32], 200);

        let block_info = BlockInfo::default();
        let mut batch = drive
            .destroy_token_issuer_operations(
                first_issuer.to_buffer(),
                &block_info,
                &mut None,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the first destruction");
        let second = drive
            .destroy_token_issuer_operations(
                second_issuer.to_buffer(),
                &block_info,
                &mut Some(&mut batch),
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the second destruction");
        batch.extend(second);

        // Two records and one scalar: the second destruction rewrote the pending scalar.
        assert_eq!(ledger_writes(&batch), 3);

        // The batch consistency check of the test drive would reject a second scalar write.
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the batch to apply");

        drive.assert_token_rollups_consistent(None, platform_version);
        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert!(totals.ok().expect("expected a verdict"));
        assert_eq!(totals.total_destroyed_supply, 300);
        for issuer in [first_issuer, second_issuer] {
            assert!(drive
                .fetch_contract_token_lifecycle(issuer.to_buffer(), None, platform_version)
                .expect("expected to read")
                .expect("expected a record")
                .is_wiped());
        }
    }

    #[test]
    fn should_destroy_after_a_supply_write_of_the_same_issuer_in_one_batch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        let token_id = [1u8; 32];
        create_token(&drive, contract_id, 0, token_id);
        mint(&drive, token_id, [10u8; 32], 100);

        let mut batch = drive
            .token_mint_operations(
                token_id,
                [10u8; 32],
                50,
                false,
                false,
                &mut None,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the mint operations");
        let destruction = drive
            .destroy_token_issuer_operations(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                &mut Some(&mut batch),
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the destruction operations");
        batch.extend(destruction);

        // The record write of the mint was rewritten; only the scalar write was added.
        assert_eq!(ledger_writes(&batch), 2);

        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the batch to apply");

        drive.assert_token_rollups_consistent(None, platform_version);
        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert!(totals.ok().expect("expected a verdict"));
        assert_eq!(totals.total_destroyed_supply, 150);
        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 150);
        assert!(record.is_wiped());
    }

    #[test]
    fn should_refuse_a_second_destruction_pending_in_the_same_batch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        create_token(&drive, contract_id, 0, [1u8; 32]);

        let mut batch = drive
            .destroy_token_issuer_operations(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                &mut None,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the first destruction");
        let result = drive.destroy_token_issuer_operations(
            contract_id.to_buffer(),
            &BlockInfo::default(),
            &mut Some(&mut batch),
            &mut None,
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::TokenIssuerAlreadyDestroyed(id))) if id == contract_id
        ));
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

        // The estimate has to cover the applied cost, wipe marker included.
        let applied = drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to apply");
        assert!(
            fees.processing_fee >= applied.processing_fee,
            "estimated {} is below applied {}",
            fees.processing_fee,
            applied.processing_fee
        );
        assert!(
            fees.storage_fee >= applied.storage_fee,
            "estimated storage {} is below applied storage {}",
            fees.storage_fee,
            applied.storage_fee
        );
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
