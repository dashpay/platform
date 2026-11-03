use super::v0::IdentityUpdateStateTransitionStateValidationV0;
use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::identity_already_has_hecate_signer_key_error::IdentityAlreadyHasHecateSignerKeyError;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Purpose;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_identity_public_key_contract_bounds::validate_identity_public_keys_contract_bounds;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_dont_exist_in_state::validate_identity_public_key_ids_dont_exist_in_state;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_exist_in_state::validate_identity_public_key_ids_exist_in_state;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_limits::validate_identity_public_keys_limits;
use crate::execution::validation::state_transition::common::validate_not_disabling_last_master_key::validate_master_key_uniqueness;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;

pub(in crate::execution::validation::state_transition::state_transitions::identity_update) trait IdentityUpdateStateTransitionStateValidationV2
{
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        state_transition_execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityUpdateStateTransitionStateValidationV2 for IdentityUpdateTransition {
    /// Generation 1's checks, then: an identity keeps at most one enabled Hecate signer key
    /// (protocol version 15). An update adding one is refused while the identity keeps another
    /// enabled that the update does not disable; disabling the old one in the same update
    /// rotates it. Only an update adding a signer key reads the identity's keys, and it pays
    /// for each key read.
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        state_transition_execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_not_in_state(
                self.public_keys_to_add(),
                drive,
                state_transition_execution_context,
                tx,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        validation_result.add_errors(
            validate_identity_public_key_ids_dont_exist_in_state(
                self.identity_id(),
                self.public_keys_to_add(),
                drive,
                tx,
                state_transition_execution_context,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        // Now we should check to make sure any keys that are added are valid for the contract
        // bounds they refer to
        validation_result.add_errors(
            validate_identity_public_keys_contract_bounds(
                self.identity_id(),
                self.public_keys_to_add(),
                drive,
                &block_info.epoch,
                tx,
                state_transition_execution_context,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        // A key must not already be expired in the block that registers it
        validation_result.add_errors(
            validate_identity_public_keys_limits(
                self.public_keys_to_add(),
                block_info,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        if !self.public_key_ids_to_disable().is_empty() {
            let validation_result_and_keys_to_disable =
                validate_identity_public_key_ids_exist_in_state(
                    self.identity_id(),
                    self.public_key_ids_to_disable(),
                    drive,
                    state_transition_execution_context,
                    tx,
                    platform_version,
                )?;
            // We need to validate that all keys removed existed
            if !validation_result_and_keys_to_disable.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result_and_keys_to_disable.errors,
                ));
            }

            let keys_to_disable = validation_result_and_keys_to_disable.into_data()?;

            let validation_result = validate_master_key_uniqueness(
                self.public_keys_to_add(),
                keys_to_disable.as_slice(),
                platform_version,
            )?;
            if !validation_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
        }

        // Key structure validation already refused an update adding two signer keys.
        if self
            .public_keys_to_add()
            .iter()
            .any(|key| key.purpose() == Purpose::HECATE_SIGNER)
        {
            let keys = drive.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                IdentityKeysRequest {
                    identity_id: self.identity_id().to_buffer(),
                    request_type: KeyRequestType::AllKeys,
                    limit: None,
                    offset: None,
                },
                tx,
                platform_version,
            )?;
            state_transition_execution_context.add_operation(
                ValidationOperation::RetrieveIdentity(RetrieveIdentityInfo::keys(
                    u16::try_from(keys.len()).unwrap_or(u16::MAX),
                )),
            );
            if let Some(existing) = keys.values().find(|key| {
                key.purpose() == Purpose::HECATE_SIGNER
                    && !key.is_disabled()
                    && !self.public_key_ids_to_disable().contains(&key.id())
            }) {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![IdentityAlreadyHasHecateSignerKeyError::new(
                        self.identity_id(),
                        existing.id(),
                    )
                    .into()],
                ));
            }
        }

        self.transform_into_action_v0()
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::tests::setup_identity_return_master_key;
    use crate::platform_types::state_transitions_processing_result::{
        StateTransitionExecutionResult, StateTransitionsProcessingResult,
    };
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::redpallas_hecate::RedPallasHecateSecretKey;
    use dpp::identity::signer::Signer;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;

    /// An identity update adding the Hecate signer key derived from `secret` as key `key_id`,
    /// disabling `disable`, signed by `signing_key`, at `revision` and `nonce`. With `prove`, the
    /// key carries its proof of possession; without it, a proof by another secret.
    #[allow(clippy::too_many_arguments)]
    async fn add_signer_key_update(
        identity: &Identity,
        signer: &SimpleSigner,
        signing_key: &IdentityPublicKey,
        key_id: KeyID,
        secret: [u8; 32],
        disable: Vec<KeyID>,
        revision: u64,
        nonce: u64,
        prove: bool,
    ) -> Vec<u8> {
        let secret_key = RedPallasHecateSecretKey::from_secret(&secret).expect("a valid secret");
        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: key_id,
            purpose: Purpose::HECATE_SIGNER,
            security_level: SecurityLevel::MEDIUM,
            key_type: KeyType::REDPALLAS_HECATE,
            read_only: false,
            data: secret_key.public_key_data().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };
        let transition = |new_key: IdentityPublicKeyInCreationV0| -> StateTransition {
            IdentityUpdateTransition::from(IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision,
                nonce,
                add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
                disable_public_keys: disable.clone(),
                user_fee_increase: 0,
                signature_public_key_id: signing_key.id(),
                signature: Default::default(),
            })
            .into()
        };
        let signable_bytes = transition(new_key.clone())
            .signable_bytes()
            .expect("expected signable bytes");
        let possession = if prove {
            secret_key.sign(&signable_bytes)
        } else {
            RedPallasHecateSecretKey::from_secret(&[0xEE; 32])
                .expect("a valid secret")
                .sign(&signable_bytes)
        };
        new_key.signature = possession.to_vec().into();

        let mut update = transition(new_key);
        update.set_signature(
            signer
                .sign(signing_key, &signable_bytes)
                .await
                .expect("expected to sign"),
        );
        update.serialize_to_bytes().expect("expected to serialize")
    }

    fn process(
        platform: &TempPlatform<MockCoreRPCLike>,
        bytes: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
        result
    }

    fn fetch_identity(platform: &TempPlatform<MockCoreRPCLike>, identity: &Identity) -> Identity {
        platform
            .drive
            .fetch_full_identity(identity.id().to_buffer(), None, PlatformVersion::latest())
            .expect("expected to fetch")
            .expect("expected the identity")
    }

    #[tokio::test]
    async fn should_add_a_hecate_signer_key_with_its_proof_of_possession() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let update = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            2,
            [7; 32],
            vec![],
            1,
            1,
            true,
        )
        .await;
        let result = process(&platform, update, platform_version);
        assert_eq!(result.valid_count(), 1, "{:?}", result.execution_results());

        let stored = fetch_identity(&platform, &identity);
        let key = stored.public_keys().get(&2).expect("the signer key");
        assert_eq!(key.key_type(), KeyType::REDPALLAS_HECATE);
        assert_eq!(key.purpose(), Purpose::HECATE_SIGNER);
        assert_eq!(
            key.data().as_slice(),
            RedPallasHecateSecretKey::from_secret(&[7; 32])
                .unwrap()
                .public_key_data()
                .as_slice()
        );

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to verify");
        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn should_refuse_a_hecate_signer_key_whose_proof_of_possession_does_not_verify() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let update = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            2,
            [7; 32],
            vec![],
            1,
            1,
            false,
        )
        .await;
        let result = process(&platform, update, platform_version);
        assert_eq!(result.valid_count(), 0);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(_)]
        );
        assert!(!fetch_identity(&platform, &identity)
            .public_keys()
            .contains_key(&2));
    }

    #[tokio::test]
    async fn should_refuse_a_second_enabled_hecate_signer_key_and_rotate_by_disabling_the_first() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let first = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            2,
            [7; 32],
            vec![],
            1,
            1,
            true,
        )
        .await;
        assert_eq!(process(&platform, first, platform_version).valid_count(), 1);

        // A second signer key while the first is enabled: refused, and paid.
        let second = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            3,
            [8; 32],
            vec![],
            2,
            2,
            true,
        )
        .await;
        let result = process(&platform, second, platform_version);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError { error, .. }]
                if error.code() == 40223
        );
        assert!(!fetch_identity(&platform, &identity)
            .public_keys()
            .contains_key(&3));

        // Disabling the first in the same update rotates it. The refused update used nonce 2
        // and left the revision at 1.
        let rotation = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            3,
            [8; 32],
            vec![2],
            2,
            3,
            true,
        )
        .await;
        let result = process(&platform, rotation, platform_version);
        assert_eq!(result.valid_count(), 1, "{:?}", result.execution_results());
        let stored = fetch_identity(&platform, &identity);
        assert!(stored.public_keys()[&2].is_disabled());
        assert!(!stored.public_keys()[&3].is_disabled());
    }

    #[tokio::test]
    async fn should_refuse_a_hecate_signer_key_as_the_signer_of_a_transition() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, mut signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let first = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            2,
            [7; 32],
            vec![],
            1,
            1,
            true,
        )
        .await;
        assert_eq!(process(&platform, first, platform_version).valid_count(), 1);

        let signer_key = fetch_identity(&platform, &identity).public_keys()[&2].clone();
        signer.add_identity_public_key(signer_key.clone(), [7; 32]);

        // An update signed by the signer key itself.
        let update = add_signer_key_update(
            &identity,
            &signer,
            &signer_key,
            3,
            [8; 32],
            vec![2],
            2,
            2,
            true,
        )
        .await;
        let result = process(&platform, update, platform_version);
        assert_eq!(result.valid_count(), 0);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::SignatureError(_)
            )]
        );
    }

    #[tokio::test]
    async fn should_not_admit_a_hecate_signer_key_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(14)
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let update = add_signer_key_update(
            &identity,
            &signer,
            &master_key,
            2,
            [7; 32],
            vec![],
            1,
            1,
            true,
        )
        .await;
        let result = process(&platform, update, platform_version);
        assert_eq!(result.valid_count(), 0);
        assert_eq!(result.invalid_paid_count(), 0);
        assert!(!fetch_identity(&platform, &identity)
            .public_keys()
            .contains_key(&2));
    }
}
