use crate::drive::balances::total_tokens_root_supply_path;
use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::paths::{
    token_balances_root_path, token_contract_infos_root_path, token_contract_lifecycles_root_path,
    token_contract_lifecycles_root_path_vec, token_identity_infos_root_path,
    token_statuses_root_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType, QueryTarget};
use crate::util::object_size_info::PathKeyElementInfo;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::TokenContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::contract_lifecycle::ContractTokenLifecycle;
use dpp::tokens::status::TokenStatus;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveOp, KeyInfoPath};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Creates a new token root subtree at `TokenBalances` keyed by `token_id`, and the
    /// issuer's lifecycle record when the contract has none yet. A destroyed issuer is refused
    /// as corrupted state: a contract update that adds a token to it is rejected by validation
    /// before it reaches Drive.
    /// This function applies the operations directly, calculates fees, and returns the fee result.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_token_trees_v1(
        &self,
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // Add operations to create the token root tree
        self.create_token_trees_add_to_operations_v1(
            contract_id,
            token_contract_position,
            token_id,
            start_as_paused,
            allow_already_exists,
            apply,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        // If applying, calculate fees
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

    /// Adds the token root creation operations to the provided `drive_operations` vector without
    #[allow(clippy::too_many_arguments)]
    /// calculating or returning fees. If `apply` is false, it will only estimate costs.
    pub(super) fn create_token_trees_add_to_operations_v1(
        &self,
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        // Get the operations required to create the token tree
        let batch_operations = self.create_token_trees_operations_v1(
            contract_id,
            token_contract_position,
            token_id,
            start_as_paused,
            allow_already_exists,
            previous_batch_operations,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        // Apply or estimate the operations
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Gathers the operations needed to create the token root subtree. If `apply` is false, it
    /// populates `estimated_costs_only_with_layer_info` instead of applying.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_token_trees_operations_v1(
        &self,
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: [u8; 32],
        start_as_paused: bool,
        allow_already_exists: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        // Every layer this generation writes, so the estimate stands on its own; the
        // callers that already added some of these layers overwrite them with the same
        // values.
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_balances(
                token_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_token_identity_infos(
                token_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_token_status_infos(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_token_contract_infos(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_token_total_supply(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_token_contract_lifecycles(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let non_sum_tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        let item_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::NormalTree,
                target: QueryTarget::QueryTargetValue(8),
            }
        };

        let token_balance_tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::BigSumTree,
                tree_type: TreeType::SumTree,
                flags_len: 0,
            }
        };

        // Insert an empty tree for this token if it doesn't exist
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKeyRef::<2>((token_balances_root_path(), token_id.as_slice())),
            TreeType::SumTree,
            None,
            token_balance_tree_apply_type,
            transaction,
            previous_batch_operations,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted && !allow_already_exists {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token balance root tree already exists".to_string(),
            )));
        }

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKeyRef::<2>((token_identity_infos_root_path(), token_id.as_slice())),
            TreeType::NormalTree,
            None,
            non_sum_tree_apply_type,
            transaction,
            &mut None,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted && !allow_already_exists {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token balance tree already exists".to_string(),
            )));
        }

        let starting_status = TokenStatus::new(start_as_paused, platform_version)?;
        let token_status_bytes = starting_status.serialize_consume_to_bytes()?;

        let inserted = self.batch_insert_if_not_exists(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                token_statuses_root_path(),
                token_id.as_slice(),
                Element::Item(token_status_bytes, None),
            )),
            item_apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted && !allow_already_exists {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token info tree already exists".to_string(),
            )));
        }

        let token_contract_info =
            TokenContractInfo::new(contract_id, token_contract_position, platform_version)?;
        let token_contract_info_bytes = token_contract_info.serialize_consume_to_bytes()?;

        self.batch_insert(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                token_contract_infos_root_path(),
                token_id.as_slice(),
                Element::Item(token_contract_info_bytes, None),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        self.batch_insert_sum_item_if_not_exists(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                total_tokens_root_supply_path(),
                token_id.as_slice(),
                Element::SumItem(0, None),
            )),
            !allow_already_exists,
            item_apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        // The issuer's lifecycle record. A token starts with no supply, so a record created
        // here starts at zero; a contract that already has one (an issuer adding a token, or
        // an earlier token of the same contract in this batch) keeps it, unless it is wiped:
        // a destroyed issuer never gains a token. The batch scan is what keeps a contract
        // update that adds several tokens from inserting the record once per token; the
        // first token of the batch is the one that reads the stored record, and a record
        // already written by the batch is checked from the pending write, so a destruction
        // composed into the same batch refuses the token like a stored one.
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_contract_lifecycles(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let record_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::NormalTree,
                target: QueryTarget::QueryTargetValue(
                    ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES,
                ),
            }
        };
        let pending_record_write = previous_batch_operations.as_deref().and_then(|operations| {
            let lifecycles_path =
                KeyInfoPath::from_known_owned_path(token_contract_lifecycles_root_path_vec());
            let contract_key = Some(KeyInfo::KnownKey(contract_id.to_vec()));
            operations.iter().find_map(|operation| match operation {
                LowLevelDriveOperation::GroveOperation(grove_op)
                    if grove_op.path == lifecycles_path && grove_op.key == contract_key =>
                {
                    Some(&grove_op.op)
                }
                _ => None,
            })
        });
        match pending_record_write {
            Some(pending) if estimated_costs_only_with_layer_info.is_none() => {
                let record = match pending {
                    GroveOp::InsertOrReplace {
                        element: Element::Item(bytes, _),
                    }
                    | GroveOp::Replace {
                        element: Element::Item(bytes, _),
                    } => ContractTokenLifecycle::deserialize_from_bytes(bytes.as_slice())?,
                    _ => {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a pending contract token lifecycle write is not an item insert or replacement",
                        )))
                    }
                };
                if record.is_wiped() {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "contract {} is destroyed in this batch, it cannot issue a new token",
                        contract_id
                    ))));
                }
            }
            // Priced without state: the earlier token of the batch priced the record.
            Some(_) => {}
            None => {
                let record_bytes = ContractTokenLifecycle::new(0, platform_version)?
                    .serialize_consume_to_bytes()?;
                let existing_record = self.batch_insert_if_not_exists_return_existing_element(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                        token_contract_lifecycles_root_path(),
                        contract_id.as_slice(),
                        Element::Item(record_bytes, None),
                    )),
                    record_apply_type,
                    transaction,
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
                if let Some(existing_record) = existing_record {
                    let record = match existing_record {
                        Element::Item(bytes, _) => {
                            ContractTokenLifecycle::deserialize_from_bytes(bytes.as_slice())?
                        }
                        _ => {
                            return Err(Error::Drive(DriveError::CorruptedElementType(
                                "contract token lifecycle was present but was not an item",
                            )))
                        }
                    };
                    if record.is_wiped() {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                            "contract {} was destroyed, it cannot issue a new token",
                            contract_id
                        ))));
                    }
                }
            }
        }

        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::prelude::Identifier;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn create(
        drive: &Drive,
        contract_id: Identifier,
        position: u16,
        token_id: [u8; 32],
        allow_already_exists: bool,
    ) {
        drive
            .create_token_trees(
                contract_id,
                position,
                token_id,
                false,
                allow_already_exists,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to create token trees");
    }

    #[test]
    fn should_create_the_token_trees_and_a_zero_lifecycle_record() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([52u8; 32]);
        let token_id = [51u8; 32];

        create(&drive, contract_id, 0, token_id, false);

        assert_eq!(
            drive
                .fetch_token_total_supply(token_id, None, platform_version)
                .expect("expected to fetch supply"),
            Some(0)
        );
        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 0);
        assert!(!record.is_wiped());
    }

    #[test]
    fn should_keep_the_record_of_an_issuer_adding_a_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([52u8; 32]);
        let first_token = [51u8; 32];
        let second_token = [53u8; 32];

        create(&drive, contract_id, 0, first_token, false);
        drive
            .token_mint(
                first_token,
                [1u8; 32],
                120,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint");

        create(&drive, contract_id, 1, second_token, true);

        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 120);
    }

    #[test]
    fn should_insert_the_record_once_for_two_tokens_in_one_batch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([52u8; 32]);
        let mut batch_operations = vec![];

        for (position, token_id) in [(0, [51u8; 32]), (1, [53u8; 32])] {
            let operations = drive
                .create_token_trees_operations(
                    contract_id,
                    position,
                    token_id,
                    false,
                    false,
                    &mut Some(&mut batch_operations),
                    &mut None,
                    None,
                    platform_version,
                )
                .expect("expected operations");
            batch_operations.extend(operations);
        }

        // The batch consistency check of the test drive rejects a key written twice.
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the batch to apply");

        let record = drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 0);
    }

    #[test]
    fn should_estimate_without_touching_state_and_cover_the_applied_cost() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([52u8; 32]);

        let estimated = drive
            .create_token_trees(
                contract_id,
                0,
                [51u8; 32],
                false,
                false,
                &BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("expected an estimate");

        assert!(estimated.processing_fee > 0);
        assert_eq!(
            drive
                .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
                .expect("expected to read"),
            None
        );

        let applied = drive
            .create_token_trees(
                contract_id,
                0,
                [51u8; 32],
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to apply");
        // Block execution requires the estimated total to cover the applied total. The
        // estimator sizes the three empty tree inserts a few bytes short in storage (the
        // shipped contract insert shows the same gap at protocol version 14), so the check
        // is on the total, which the processing estimate covers many times over; the
        // lifecycle record itself is an item priced at its largest size.
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "estimated {} is below applied {}",
            estimated.processing_fee,
            applied.processing_fee
        );
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "estimated total {} is below applied total {}",
            estimated.total_base_fee(),
            applied.total_base_fee()
        );
    }

    #[test]
    fn should_refuse_a_new_token_for_a_destroyed_issuer() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([52u8; 32]);
        let new_token = [53u8; 32];

        create(&drive, contract_id, 0, [51u8; 32], false);
        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let result = drive.create_token_trees(
            contract_id,
            1,
            new_token,
            false,
            true,
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
                .fetch_token_total_supply(new_token, None, platform_version)
                .expect("expected to fetch supply"),
            None
        );
        assert!(drive
            .fetch_contract_token_lifecycle(contract_id.to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record")
            .is_wiped());
    }

    fn expect_refused_after_a_pending_destruction(
        drive: &Drive,
        contract_id: Identifier,
        position: u16,
        new_token: [u8; 32],
    ) {
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

        let result = drive.create_token_trees_operations(
            contract_id,
            position,
            new_token,
            false,
            false,
            &mut Some(&mut batch),
            &mut None,
            None,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedDriveState(_)))
            ),
            "expected the token to be refused, got {result:?}"
        );
    }

    #[test]
    fn should_refuse_a_new_token_when_the_issuer_is_destroyed_in_the_same_batch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract_id = Identifier::from([52u8; 32]);
        create(&drive, contract_id, 0, [51u8; 32], false);

        // The pending destruction replaces the stored record.
        expect_refused_after_a_pending_destruction(&drive, contract_id, 1, [53u8; 32]);
    }

    #[test]
    fn should_refuse_a_first_token_when_the_contract_is_destroyed_in_the_same_batch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract_id = Identifier::from([52u8; 32]);

        // A contract without tokens has no record; the pending destruction inserts a wiped one.
        expect_refused_after_a_pending_destruction(&drive, contract_id, 0, [51u8; 32]);
    }

    #[test]
    fn should_refuse_a_contract_update_adding_a_token_to_a_destroyed_issuer() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(false);
        let token_config = || {
            TokenConfiguration::V0(
                TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
            )
        };
        contract.set_tokens(BTreeMap::from([(0, token_config())]));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to insert the contract");

        drive
            .destroy_token_issuer(
                contract.id().to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        contract.set_tokens(BTreeMap::from([(0, token_config()), (1, token_config())]));
        contract.increment_version();
        let result = drive.update_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedDriveState(_)))
            ),
            "expected the update to be refused, got {result:?}"
        );
        let second_token = contract.token_id(1).expect("expected a token id");
        assert_eq!(
            drive
                .fetch_token_total_supply(second_token.to_buffer(), None, platform_version)
                .expect("expected to fetch supply"),
            None
        );
    }
}
