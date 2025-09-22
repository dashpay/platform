pub(crate) mod advanced_structure;
mod basic_structure;
mod nonce;
mod state;

use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::drive::subscriptions::DriveSubscriptionFilter;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_update::basic_structure::v0::IdentityUpdateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::identity_update::state::v0::IdentityUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
};

use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for IdentityUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityUpdateTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for IdentityUpdateTransition {
    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        _block_info: &BlockInfo,
        _execution_context: &mut StateTransitionExecutionContext,
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::tests::{
        setup_add_key_to_identity, setup_identity_return_master_key,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::dashcore::key::{Keypair, Secp256k1};
    use dpp::dashcore::signer;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::signer::Signer;
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use dpp::state_transition::StateTransition;
    use drive::drive::Drive;
    use platform_version::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_identity_update_that_disables_an_authentication_key() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![1],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_identity_update_that_adds_an_authentication_key() {
        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        update_transition.set_signature(
            signer
                .sign(&key, signable_bytes.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let proof_result = platform
            .platform
            .drive
            .prove_state_transition(&update_transition, None, platform_version)
            .map_err(|e| e.to_string())
            .expect("expected to create proof");

        if let Some(proof_error) = proof_result.first_error() {
            panic!("proof_result is not valid with error {}", proof_error);
        }

        let proof_data = proof_result
            .into_data()
            .map_err(|e| e.to_string())
            .expect("expected to get proof data");

        let (_, verification_result) = Drive::verify_state_transition_was_executed_with_proof(
            &update_transition,
            &BlockInfo::default(),
            &proof_data,
            &|_id: &Identifier| Ok(None),
            platform_version,
        )
        .map_err(|e| e.to_string())
        .expect("expected to verify state transition");

        let StateTransitionProofResult::VerifiedPartialIdentity(_document) = verification_result
        else {
            panic!(
                "verification_result expected partial identity, but got: {:?}",
                verification_result
            );
        };
    }

    #[test]
    fn test_identity_update_that_disables_an_encryption_key() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (mut identity, mut signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();

        let key = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            4,
            2,
            Purpose::ENCRYPTION,
            SecurityLevel::MEDIUM,
            KeyType::ECDSA_SECP256K1,
            Some(ContractBounds::SingleContractDocumentType {
                id: dashpay.id(),
                document_type_name: "contactRequest".to_string(),
            }),
        );

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );

        let platform_state = platform.state.load();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![key.id()],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&master_key, data.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    fn test_identity_update_adding_owner_key_not_allowed() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::HIGH,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![new_key.into()],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect there to be an error because you should not be able to add owner keys
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_identity_update_adding_contract_bound_key() {
        use crate::execution::validation::state_transition::tests::{
            register_contract_from_bytes, IdentityTestInfo,
        };

        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // {
        //   "$format_version": "1",
        //   "id": "5pkMhyeaFjJfVMkFhLtJdDp2ofx6iqt7i9k6ckkHBwbs",
        //   "config": {
        //     "$format_version": "1",
        //     "canBeDeleted": false,
        //     "readonly": false,
        //     "keepsHistory": false,
        //     "documentsKeepHistoryContractDefault": false,
        //     "documentsMutableContractDefault": true,
        //     "documentsCanBeDeletedContractDefault": true,
        //     "requiresIdentityEncryptionBoundedKey": 0,
        //     "requiresIdentityDecryptionBoundedKey": 0,
        //     "sizedIntegerTypes": true
        //   },
        //   "version": 1,
        //   "ownerId": "DicUmimv71VqxNBzZHXb887RgssSEjjx7DyLfxrt8q1X",
        //   "schemaDefs": null,
        //   "documentSchemas": {
        //     "preorder": {
        //       "documentsMutable": false,
        //       "canBeDeleted": true,
        //       "type": "object",
        //       "indices": [
        //         {
        //           "name": "saltedHash",
        //           "properties": [
        //             {
        //               "saltedDomainHash": "asc"
        //             }
        //           ],
        //           "unique": true
        //         }
        //       ],
        //       "properties": {
        //         "saltedDomainHash": {
        //           "type": "array",
        //           "byteArray": true,
        //           "minItems": 32,
        //           "maxItems": 32,
        //           "position": 0,
        //           "description": "Double sha-256 of the concatenation of a 32 byte random salt and a normalized domain name"
        //         }
        //       },
        //       "required": [
        //         "saltedDomainHash"
        //       ],
        //       "additionalProperties": false,
        //       "$comment": "Preorder documents are immutable: modification and deletion are restricted"
        //     }
        //   },
        //   "createdAt": 1749816974718,
        //   "updatedAt": null,
        //   "createdAtBlockHeight": 159130,
        //   "updatedAtBlockHeight": null,
        //   "createdAtEpoch": 7906,
        //   "updatedAtEpoch": null,
        //   "groups": {},
        //   "tokens": {},
        //   "keywords": [],
        //   "description": null
        // }
        let contract_bytes = hex::decode("0147aa11d517710d509edaf84bb54902394dcb8f6cc68775138d1cdd8334600d2e01000000000101010001000101bcf52c1c5d57d2e21530c5d03ef4c6e7b39a91da7c444fdb17e0a7746b6285860001087072656f7264657216081210646f63756d656e74734d757461626c651300120c63616e426544656c65746564130012047479706512066f626a6563741207696e64696365731501160312046e616d65120a73616c74656448617368120a70726f7065727469657315011601121073616c746564446f6d61696e4861736812036173631206756e697175651301120a70726f706572746965731601121073616c746564446f6d61696e486173681606120474797065120561727261791209627974654172726179130112086d696e4974656d73022012086d61784974656d7302201208706f736974696f6e0200120b6465736372697074696f6e1259446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65120872657175697265641501121073616c746564446f6d61696e4861736812146164646974696f6e616c50726f706572746965731300120824636f6d6d656e74124a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656401fd0000019769381d7e0001fc00026d9a0001fb1ee20000000000").expect("expected to decode contract bytes");

        let (identity, signer, critical_key, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(5.0));

        let platform_state = platform.state.load();

        // Register the contract
        let data_contract = register_contract_from_bytes(
            &mut platform,
            &platform_state,
            contract_bytes,
            IdentityTestInfo::Given {
                identity: &identity,
                signer: &signer,
                public_key: &critical_key,
                identity_nonce: 1,
            },
            platform_version,
        );

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(ContractBounds::SingleContract {
                id: data_contract.id(),
            }),
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2, // Use nonce 2 since we used 1 for contract creation
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        // Sign the new key with its own private key
        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        // Create the transition again with the signed key
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        // Sign the transition with the master key
        update_transition.set_signature(
            signer
                .sign(&master_key, signable_bytes.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect success - contract bound keys are allowed
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Verify the key was added
        use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};

        let identity_keys_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let updated_partial_identity = platform
            .drive
            .fetch_identity_keys_as_partial_identity(identity_keys_request, None, platform_version)
            .expect("expected to fetch identity")
            .expect("expected identity to exist");

        assert_eq!(updated_partial_identity.loaded_public_keys.len(), 3); // Original 2 + new contract bound key

        let contract_bound_key = updated_partial_identity
            .loaded_public_keys
            .get(&2)
            .expect("expected to find key with id 2");

        assert_eq!(
            contract_bound_key.contract_bounds(),
            Some(&ContractBounds::SingleContract {
                id: data_contract.id()
            })
        );
    }

    #[test]
    fn test_identity_update_adding_contract_bound_key_on_document_level() {
        use crate::execution::validation::state_transition::tests::{
            register_contract_from_bytes, IdentityTestInfo,
        };

        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // {
        //   "$format_version": "1",
        //   "id": "8m7H1EScryPTeJzck2qbSckTbEjCg2vu7PRth2LCwsHo",
        //   "config": {
        //     "$format_version": "1",
        //     "canBeDeleted": false,
        //     "readonly": false,
        //     "keepsHistory": false,
        //     "documentsKeepHistoryContractDefault": false,
        //     "documentsMutableContractDefault": true,
        //     "documentsCanBeDeletedContractDefault": true,
        //     "requiresIdentityEncryptionBoundedKey": null,
        //     "requiresIdentityDecryptionBoundedKey": null,
        //     "sizedIntegerTypes": true
        //   },
        //   "version": 1,
        //   "ownerId": "Eh5SUY1wHovQn5xoW9gEpA3ABfHSmq2bg9pcSiybkBWW",
        //   "schemaDefs": null,
        //   "documentSchemas": {
        //     "liquidityPool": {
        //       "type": "object",
        //       "requiresIdentityEncryptionBoundedKey": 0,
        //       "requiresIdentityDecryptionBoundedKey": 0,
        //       "properties": {
        //         "tokenA": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The symbol of the first token in the pool.",
        //           "maxLength": 10
        //         },
        //         "tokenB": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The symbol of the second token in the pool.",
        //           "maxLength": 10
        //         },
        //         "reserveA": {
        //           "position": 2,
        //           "type": "number",
        //           "description": "The amount of token A in the pool.",
        //           "minimum": 0
        //         },
        //         "reserveB": {
        //           "position": 3,
        //           "type": "number",
        //           "description": "The amount of token B in the pool.",
        //           "minimum": 0
        //         },
        //         "liquidityTokens": {
        //           "position": 4,
        //           "type": "number",
        //           "description": "The total liquidity tokens issued for this pool.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "tokenPair",
        //           "properties": [
        //             {
        //               "tokenA": "asc"
        //             },
        //             {
        //               "tokenB": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "tokenA",
        //         "tokenB",
        //         "reserveA",
        //         "reserveB",
        //         "liquidityTokens"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a liquidity pool for token trading."
        //     },
        //     "swapTransaction": {
        //       "type": "object",
        //       "properties": {
        //         "fromToken": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The symbol of the token being swapped from.",
        //           "maxLength": 10
        //         },
        //         "toToken": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The symbol of the token being swapped to.",
        //           "maxLength": 10
        //         },
        //         "amountIn": {
        //           "position": 2,
        //           "type": "number",
        //           "description": "The amount of the fromToken being swapped.",
        //           "minimum": 0
        //         },
        //         "amountOut": {
        //           "position": 3,
        //           "type": "number",
        //           "description": "The amount of the toToken received.",
        //           "minimum": 0
        //         },
        //         "timestamp": {
        //           "position": 4,
        //           "type": "integer",
        //           "description": "The timestamp of the transaction.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "fromToken",
        //           "properties": [
        //             {
        //               "fromToken": "asc"
        //             }
        //           ]
        //         },
        //         {
        //           "name": "toToken",
        //           "properties": [
        //             {
        //               "toToken": "asc"
        //             }
        //           ]
        //         },
        //         {
        //           "name": "timestamp",
        //           "properties": [
        //             {
        //               "timestamp": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "fromToken",
        //         "toToken",
        //         "amountIn",
        //         "amountOut",
        //         "timestamp"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a swap transaction between two tokens."
        //     },
        //     "token": {
        //       "type": "object",
        //       "properties": {
        //         "symbol": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The symbol of the token, e.g., 'DASH'.",
        //           "maxLength": 10
        //         },
        //         "name": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The full name of the token.",
        //           "maxLength": 63
        //         },
        //         "decimals": {
        //           "position": 2,
        //           "type": "integer",
        //           "description": "The number of decimal places the token uses.",
        //           "minimum": 0,
        //           "maximum": 18
        //         },
        //         "totalSupply": {
        //           "position": 3,
        //           "type": "number",
        //           "description": "The total supply of the token.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "symbol",
        //           "properties": [
        //             {
        //               "symbol": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "symbol",
        //         "name",
        //         "decimals",
        //         "totalSupply"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a token available for trading on the platform."
        //     },
        //     "userProfile": {
        //       "type": "object",
        //       "properties": {
        //         "username": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The unique username of the user.",
        //           "maxLength": 63
        //         },
        //         "walletAddress": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The Dash wallet address of the user.",
        //           "maxLength": 63
        //         },
        //         "createdAt": {
        //           "position": 2,
        //           "type": "integer",
        //           "description": "The timestamp when the user profile was created.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "username",
        //           "properties": [
        //             {
        //               "username": "asc"
        //             }
        //           ]
        //         },
        //         {
        //           "name": "walletAddress",
        //           "properties": [
        //             {
        //               "walletAddress": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "username",
        //         "walletAddress",
        //         "createdAt"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a user profile within the application."
        //     }
        //   },
        //   "createdAt": 1750216267336,
        //   "updatedAt": null,
        //   "createdAtBlockHeight": 161570,
        //   "updatedAtBlockHeight": null,
        //   "createdAtEpoch": 8017,
        //   "updatedAtEpoch": null,
        //   "groups": {},
        //   "tokens": {},
        //   "keywords": [],
        //   "description": null
        // }
        let contract_bytes = hex::decode("01734e75d6522ecc1161b1594bc8706c2fe32450c26a6680612c51e1f4806d39160100000000010100000101cb6c2e8d89ad3ec7bbfcef8b02eb85b8f27ef7ac847c3d461c898d6929fb6daf00040d6c6971756964697479506f6f6c160812047479706512066f626a656374122472657175697265734964656e74697479456e6372797074696f6e426f756e6465644b65790200122472657175697265734964656e7469747944656372797074696f6e426f756e6465644b65790200120a70726f7065727469657316051206746f6b656e4116041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e122a5468652073796d626f6c206f662074686520666972737420746f6b656e20696e2074686520706f6f6c2e12096d61784c656e677468020a1206746f6b656e4216041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e122b5468652073796d626f6c206f6620746865207365636f6e6420746f6b656e20696e2074686520706f6f6c2e12096d61784c656e677468020a1208726573657276654116041208706f736974696f6e020212047479706512066e756d626572120b6465736372697074696f6e122254686520616d6f756e74206f6620746f6b656e204120696e2074686520706f6f6c2e12076d696e696d756d02001208726573657276654216041208706f736974696f6e020312047479706512066e756d626572120b6465736372697074696f6e122254686520616d6f756e74206f6620746f6b656e204220696e2074686520706f6f6c2e12076d696e696d756d0200120f6c6971756964697479546f6b656e7316041208706f736974696f6e020412047479706512066e756d626572120b6465736372697074696f6e123054686520746f74616c206c697175696469747920746f6b656e732069737375656420666f72207468697320706f6f6c2e12076d696e696d756d02001207696e64696365731501160212046e616d651209746f6b656e50616972120a70726f70657274696573150216011206746f6b656e41120361736316011206746f6b656e4212036173631208726571756972656415051206746f6b656e411206746f6b656e421208726573657276654112087265736572766542120f6c6971756964697479546f6b656e7312146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e122e526570726573656e74732061206c697175696469747920706f6f6c20666f7220746f6b656e2074726164696e672e0f737761705472616e73616374696f6e160612047479706512066f626a656374120a70726f706572746965731605120966726f6d546f6b656e16041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e122b5468652073796d626f6c206f662074686520746f6b656e206265696e6720737761707065642066726f6d2e12096d61784c656e677468020a1207746f546f6b656e16041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e12295468652073796d626f6c206f662074686520746f6b656e206265696e67207377617070656420746f2e12096d61784c656e677468020a1208616d6f756e74496e16041208706f736974696f6e020212047479706512066e756d626572120b6465736372697074696f6e122a54686520616d6f756e74206f66207468652066726f6d546f6b656e206265696e6720737761707065642e12076d696e696d756d02001209616d6f756e744f757416041208706f736974696f6e020312047479706512066e756d626572120b6465736372697074696f6e122354686520616d6f756e74206f662074686520746f546f6b656e2072656365697665642e12076d696e696d756d0200120974696d657374616d7016041208706f736974696f6e02041204747970651207696e7465676572120b6465736372697074696f6e12215468652074696d657374616d70206f6620746865207472616e73616374696f6e2e12076d696e696d756d02001207696e64696365731503160212046e616d65120966726f6d546f6b656e120a70726f7065727469657315011601120966726f6d546f6b656e1203617363160212046e616d651207746f546f6b656e120a70726f70657274696573150116011207746f546f6b656e1203617363160212046e616d65120974696d657374616d70120a70726f7065727469657315011601120974696d657374616d701203617363120872657175697265641505120966726f6d546f6b656e1207746f546f6b656e1208616d6f756e74496e1209616d6f756e744f7574120974696d657374616d7012146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e1231526570726573656e747320612073776170207472616e73616374696f6e206265747765656e2074776f20746f6b656e732e05746f6b656e160612047479706512066f626a656374120a70726f706572746965731604120673796d626f6c16041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e12265468652073796d626f6c206f662074686520746f6b656e2c20652e672e2c202744415348272e12096d61784c656e677468020a12046e616d6516041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e121b5468652066756c6c206e616d65206f662074686520746f6b656e2e12096d61784c656e677468023f1208646563696d616c7316051208706f736974696f6e02021204747970651207696e7465676572120b6465736372697074696f6e122c546865206e756d626572206f6620646563696d616c20706c616365732074686520746f6b656e20757365732e12076d696e696d756d020012076d6178696d756d0212120b746f74616c537570706c7916041208706f736974696f6e020312047479706512066e756d626572120b6465736372697074696f6e121e54686520746f74616c20737570706c79206f662074686520746f6b656e2e12076d696e696d756d02001207696e64696365731501160212046e616d65120673796d626f6c120a70726f7065727469657315011601120673796d626f6c1203617363120872657175697265641504120673796d626f6c12046e616d651208646563696d616c73120b746f74616c537570706c7912146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e1239526570726573656e7473206120746f6b656e20617661696c61626c6520666f722074726164696e67206f6e2074686520706c6174666f726d2e0b7573657250726f66696c65160612047479706512066f626a656374120a70726f7065727469657316031208757365726e616d6516041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e122054686520756e6971756520757365726e616d65206f662074686520757365722e12096d61784c656e677468023f120d77616c6c65744164647265737316041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e122454686520446173682077616c6c65742061646472657373206f662074686520757365722e12096d61784c656e677468023f120963726561746564417416041208706f736974696f6e02021204747970651207696e7465676572120b6465736372697074696f6e12305468652074696d657374616d70207768656e2074686520757365722070726f66696c652077617320637265617465642e12076d696e696d756d02001207696e64696365731502160212046e616d651208757365726e616d65120a70726f70657274696573150116011208757365726e616d651203617363160212046e616d65120d77616c6c657441646472657373120a70726f7065727469657315011601120d77616c6c65744164647265737312036173631208726571756972656415031208757365726e616d65120d77616c6c657441646472657373120963726561746564417412146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e1231526570726573656e7473206120757365722070726f66696c652077697468696e20746865206170706c69636174696f6e2e01fd000001978104d6480001fc000277220001fb1f510000000000").expect("expected to decode contract bytes");

        let (identity, signer, critical_key, master_key) =
            setup_identity_return_master_key(&mut platform, 959, dash_to_credits!(5.0));

        let platform_state = platform.state.load();

        // Register the contract
        let data_contract = register_contract_from_bytes(
            &mut platform,
            &platform_state,
            contract_bytes,
            IdentityTestInfo::Given {
                identity: &identity,
                signer: &signer,
                public_key: &critical_key,
                identity_nonce: 1,
            },
            platform_version,
        );

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                id: data_contract.id(),
                document_type_name: "liquidityPool".to_string(),
            }),
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2, // Use nonce 2 since we used 1 for contract creation
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        // Sign the new key with its own private key
        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        // Create the transition again with the signed key
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        // Sign the transition with the master key
        update_transition.set_signature(
            signer
                .sign(&master_key, signable_bytes.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect success - contract bound keys are allowed
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Verify the key was added
        use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};

        let identity_keys_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let updated_partial_identity = platform
            .drive
            .fetch_identity_keys_as_partial_identity(identity_keys_request, None, platform_version)
            .expect("expected to fetch identity")
            .expect("expected identity to exist");

        assert_eq!(updated_partial_identity.loaded_public_keys.len(), 3); // Original 2 + new contract bound key

        let contract_bound_key = updated_partial_identity
            .loaded_public_keys
            .get(&2)
            .expect("expected to find key with id 2");

        assert_eq!(
            contract_bound_key.contract_bounds(),
            Some(&ContractBounds::SingleContractDocumentType {
                id: data_contract.id(),
                document_type_name: "liquidityPool".to_string(),
            })
        );
    }
}
