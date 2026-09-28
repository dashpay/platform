use crate::drive::tokens::lifecycle::add_to_contract_issued_supply::IssuedSupplyChange;
use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::lifecycle::pending_ledger_write;
use crate::drive::tokens::paths::token_contract_lifecycles_root_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::Identifier;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::tokens::contract_lifecycle::ContractTokenLifecycle;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_contract_issued_supply_operations_v0(
        &self,
        token_id: [u8; 32],
        change: IssuedSupplyChange,
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
            Self::add_estimation_costs_for_token_contract_infos(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_token_contract_lifecycles(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let contract_info = self.fetch_token_contract_info_operations(
            token_id,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        if !apply {
            // Price the record read and a write of the largest record under a key of
            // contract id length; the issuer is unknown without state.
            self.fetch_contract_token_lifecycle_operations(
                token_id,
                false,
                transaction,
                &mut drive_operations,
                platform_version,
            )?;
            // An insert rather than a replace: the estimator charges a replace no storage,
            // while the real record grows when its rollup or wipe marker does, and the
            // estimate has to cover the applied cost.
            drive_operations.push(
                LowLevelDriveOperation::insert_for_estimated_path_key_element(
                    KeyInfoPath::from_known_owned_path(token_contract_lifecycles_root_path_vec()),
                    KeyInfo::MaxKeySize {
                        unique_id: token_id.to_vec(),
                        max_size: 32,
                    },
                    Element::new_item(vec![
                        0;
                        ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES as usize
                    ]),
                ),
            );
            return Ok(drive_operations);
        }

        let contract_id = contract_info
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "token {} has no contract info, its issuer cannot be resolved",
                    Identifier::from(token_id)
                )))
            })?
            .contract_id()
            .to_buffer();

        // A write of this issuer's record already pending in the batch (a replacement by an
        // earlier supply write, or the insert of a record the batch creates) carries the
        // rollup as the earlier writes of the batch left it; it is the base, and its slot is
        // rewritten so the batch keeps one write per issuer. Otherwise the stored record is.
        let lifecycles_path = token_contract_lifecycles_root_path_vec();
        let pending_record = pending_ledger_write(
            previous_batch_operations.as_deref(),
            &lifecycles_path,
            &contract_id,
        )?;
        let mut record = match &pending_record {
            Some(pending) => {
                ContractTokenLifecycle::deserialize_from_bytes(pending.bytes.as_slice())?
            }
            None => self
                .fetch_contract_token_lifecycle_operations(
                    contract_id,
                    true,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?
                .ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "token issuer {} has no lifecycle record",
                        Identifier::from(contract_id)
                    )))
                })?,
        };

        if record.is_wiped() {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "token {} belongs to destroyed issuer {}, its supply cannot change",
                Identifier::from(token_id),
                Identifier::from(contract_id)
            ))));
        }

        match change {
            IssuedSupplyChange::Increase(amount) => {
                record.checked_add_issued_supply(amount as u128)?
            }
            IssuedSupplyChange::Decrease(amount) => {
                record.checked_sub_issued_supply(amount as u128)?
            }
        }

        let record_element = Element::new_item(record.serialize_consume_to_bytes()?);
        match (pending_record, previous_batch_operations.as_deref_mut()) {
            // The rewrite keeps the kind of the pending write: a record the batch creates is
            // not stored yet, so its rewrite stays an insert.
            (Some(pending), Some(operations)) => {
                operations[pending.index] = if pending.inserts {
                    LowLevelDriveOperation::insert_for_known_path_key_element(
                        lifecycles_path,
                        contract_id.to_vec(),
                        record_element,
                    )
                } else {
                    LowLevelDriveOperation::replace_for_known_path_key_element(
                        lifecycles_path,
                        contract_id.to_vec(),
                        record_element,
                    )
                }
            }
            _ => drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                lifecycles_path,
                contract_id.to_vec(),
                record_element,
            )),
        }

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::tokens::paths::token_contract_infos_root_path;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use grovedb::batch::GroveOp;

    fn drive_with_token() -> (Drive, Identifier, [u8; 32]) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        let token_id = [1u8; 32];

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        (drive, contract_id, token_id)
    }

    fn apply(drive: &Drive, token_id: [u8; 32], change: IssuedSupplyChange) -> Result<(), Error> {
        let platform_version = PlatformVersion::latest();
        let operations = drive.add_to_contract_issued_supply_operations(
            token_id,
            change,
            &mut None,
            &mut None,
            None,
            platform_version,
        )?;
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    #[test]
    fn should_move_the_rollup_in_both_directions() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();

        apply(&drive, token_id, IssuedSupplyChange::Increase(700)).expect("expected to raise");
        apply(&drive, token_id, IssuedSupplyChange::Decrease(200)).expect("expected to lower");

        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 500);
    }

    #[test]
    fn should_refuse_to_lower_below_zero() {
        let (drive, _, token_id) = drive_with_token();

        let result = apply(&drive, token_id, IssuedSupplyChange::Decrease(1));

        assert!(matches!(result, Err(Error::Protocol(_))));
    }

    #[test]
    fn should_refuse_a_token_without_contract_info() {
        let (drive, _, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();

        drive
            .grove
            .delete(
                &token_contract_infos_root_path(),
                &token_id,
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to delete the contract info");

        let result = apply(&drive, token_id, IssuedSupplyChange::Increase(1));

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_refuse_a_destroyed_issuer() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();

        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let result = apply(&drive, token_id, IssuedSupplyChange::Increase(1));

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_fold_two_changes_of_one_issuer_into_one_pending_replacement() {
        let (drive, contract_id, first_token) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let second_token = [2u8; 32];
        drive
            .create_token_trees(
                contract_id,
                1,
                second_token,
                false,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create the second token");

        let mut batch = drive
            .add_to_contract_issued_supply_operations(
                first_token,
                IssuedSupplyChange::Increase(100),
                &mut None,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the first change");
        let second = drive
            .add_to_contract_issued_supply_operations(
                second_token,
                IssuedSupplyChange::Increase(200),
                &mut Some(&mut batch),
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the second change");
        batch.extend(second);
        let third = drive
            .add_to_contract_issued_supply_operations(
                first_token,
                IssuedSupplyChange::Decrease(50),
                &mut Some(&mut batch),
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the third change");
        batch.extend(third);

        let record_writes = batch
            .iter()
            .filter(|operation| match operation {
                LowLevelDriveOperation::GroveOperation(grove_op) => {
                    grove_op.path == token_contract_lifecycles_root_path_vec()
                }
                _ => false,
            })
            .count();
        assert_eq!(record_writes, 1, "one replacement per issuer");

        // The batch consistency check of the test drive would reject a second write.
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the batch to apply");
        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 250);
    }

    #[test]
    fn should_fold_onto_a_pending_insert_of_the_record_and_keep_it_an_insert() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();

        // A record the batch creates: its write is an insert, and the rewrite has to stay
        // one, because a replacement of a key the batch has not stored yet is refused when
        // the batch applies. (A supply write cannot follow the token creation itself in one
        // batch, since the issuer is resolved through the contract info leaf the creation
        // also inserts; the insert is built by hand here.)
        let mut batch = vec![LowLevelDriveOperation::insert_for_known_path_key_element(
            token_contract_lifecycles_root_path_vec(),
            contract_id.to_vec(),
            Element::new_item(
                ContractTokenLifecycle::new(10, platform_version)
                    .expect("expected a record")
                    .serialize_consume_to_bytes()
                    .expect("expected to serialize"),
            ),
        )];
        let supply_change = drive
            .add_to_contract_issued_supply_operations(
                token_id,
                IssuedSupplyChange::Increase(40),
                &mut Some(&mut batch),
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the rollup move to fold onto the pending insert");
        // The pending insert was rewritten in place, not followed by a second write.
        assert!(supply_change
            .iter()
            .all(|operation| !matches!(operation, LowLevelDriveOperation::GroveOperation(_))));
        assert_eq!(batch.len(), 1);
        assert!(matches!(
            &batch[0],
            LowLevelDriveOperation::GroveOperation(grove_op)
                if matches!(grove_op.op, GroveOp::InsertOrReplace { .. })
        ));
        batch.extend(supply_change);

        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the batch to apply");
        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 50);
    }

    #[test]
    fn should_refuse_a_move_for_an_issuer_destroyed_in_the_same_batch() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();

        let mut batch = drive
            .destroy_token_issuer_operations(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                &mut None,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the destruction operations");
        let result = drive.add_to_contract_issued_supply_operations(
            token_id,
            IssuedSupplyChange::Increase(1),
            &mut Some(&mut batch),
            &mut None,
            None,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedDriveState(ref message))) if message.contains("destroyed issuer")
            ),
            "expected the wiped issuer refusal, got {result:?}"
        );
    }

    #[test]
    fn should_price_the_write_without_state() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let mut estimated_costs = Some(HashMap::new());

        let operations = drive
            .add_to_contract_issued_supply_operations(
                token_id,
                IssuedSupplyChange::Increase(1),
                &mut None,
                &mut estimated_costs,
                None,
                platform_version,
            )
            .expect("expected an estimate");

        assert!(!operations.is_empty());
        assert!(!estimated_costs.expect("expected layer info").is_empty());
        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 0);
    }
}
