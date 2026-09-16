use crate::drive::tokens::lifecycle::add_to_contract_issued_supply::IssuedSupplyChange;
use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::paths::token_contract_lifecycles_root_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_to_contract_issued_supply_operations_v0(
        &self,
        token_id: [u8; 32],
        change: IssuedSupplyChange,
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

        let mut record = self
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
            })?;

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

        drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
            token_contract_lifecycles_root_path_vec(),
            contract_id.to_vec(),
            Element::new_item(record.serialize_consume_to_bytes()?),
        ));

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
    fn should_price_the_write_without_state() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let mut estimated_costs = Some(HashMap::new());

        let operations = drive
            .add_to_contract_issued_supply_operations(
                token_id,
                IssuedSupplyChange::Increase(1),
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
