use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::prelude::{BlockHeight, ConsensusValidationResult, TimestampMillis};
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionTransformIntoActionV1
{
    fn transform_into_action_v1(
        &self,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionTransformIntoActionV1 for DataContractUpdateTransition {
    /// v0, then, in check_tx and recheck, the compiled size cap.
    ///
    /// A block holds an update to the cap in state validation, which check_tx and recheck do not
    /// run, once the stored contract's creation fields, which the transition does not carry, are
    /// restored, and with the time fields of the block that includes it. check_tx sees neither:
    /// the contract may be created earlier in the same block, and the block is not yet known. So
    /// it measures the updated contract with all six time fields at their widest encodings,
    /// which no block can exceed, and charges that compile.
    fn transform_into_action_v1(
        &self,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = self.transform_into_action_v0(
            block_info,
            validation_mode,
            execution_context,
            platform_version,
        )?;

        if !matches!(
            validation_mode,
            ValidationMode::CheckTx | ValidationMode::RecheckTx
        ) {
            return Ok(result);
        }
        let Some(StateTransitionAction::DataContractUpdateAction(action)) = result.data.as_ref()
        else {
            return Ok(result);
        };

        let mut widest = action.data_contract_ref().clone();
        widest.set_created_at(Some(TimestampMillis::MAX));
        widest.set_updated_at(Some(TimestampMillis::MAX));
        widest.set_created_at_block_height(Some(BlockHeight::MAX));
        widest.set_updated_at_block_height(Some(BlockHeight::MAX));
        widest.set_created_at_epoch(Some(EpochIndex::MAX));
        widest.set_updated_at_epoch(Some(EpochIndex::MAX));

        let mut validation_operations = vec![];
        let compiled_size_result =
            widest.validate_compiled_size(&mut validation_operations, platform_version)?;
        execution_context.add_dpp_operations(validation_operations);
        if !compiled_size_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                compiled_size_result.errors,
            ));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::DataContract;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

    fn update_transition(platform_version: &PlatformVersion) -> DataContractUpdateTransition {
        let identity_contract_nonce = IdentityNonce::default();
        let mut data_contract = get_data_contract_fixture(
            None,
            identity_contract_nonce,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        data_contract.set_version(2);
        DataContractUpdateTransitionV0 {
            identity_contract_nonce,
            data_contract: data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract"),
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into()
    }

    fn transform(
        transition: &DataContractUpdateTransition,
        validation_mode: ValidationMode,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<StateTransitionAction> {
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("failed to create execution context");
        transition
            .transform_into_action_v1(
                &BlockInfo::default(),
                validation_mode,
                &mut execution_context,
                platform_version,
            )
            .expect("failed to transform the transition")
    }

    fn compiled_size(contract: &DataContract, platform_version: &PlatformVersion) -> u32 {
        contract
            .serialize_to_storage_bytes(platform_version)
            .expect("expected the contract to compile")
            .len() as u32
    }

    /// check_tx and recheck measure the update with its time fields at their widest, which
    /// covers the contract as a block stores it whatever creation fields the stored contract
    /// holds and whichever block includes the update. They read no state, so a contract created
    /// earlier in the same block, absent here, is covered too.
    #[test]
    fn should_hold_an_update_to_the_compiled_size_cap_with_its_time_fields_at_their_widest() {
        let latest = PlatformVersion::latest();
        let transition = update_transition(latest);

        let Some(StateTransitionAction::DataContractUpdateAction(action)) =
            transform(&transition, ValidationMode::Validator, latest).data
        else {
            panic!("expected a contract update action");
        };
        let mut as_stored = action.data_contract_ref().clone();
        as_stored.set_created_at(Some(1_700_000_000_000));
        as_stored.set_updated_at(Some(1_800_000_000_000));
        as_stored.set_created_at_block_height(Some(1_000_000));
        as_stored.set_updated_at_block_height(Some(2_000_000));
        as_stored.set_created_at_epoch(Some(1_000));
        as_stored.set_updated_at_epoch(Some(2_000));
        let as_stored = compiled_size(&as_stored, latest);
        let mut widest = action.data_contract_ref().clone();
        widest.set_created_at(Some(TimestampMillis::MAX));
        widest.set_updated_at(Some(TimestampMillis::MAX));
        widest.set_created_at_block_height(Some(BlockHeight::MAX));
        widest.set_updated_at_block_height(Some(BlockHeight::MAX));
        widest.set_created_at_epoch(Some(EpochIndex::MAX));
        widest.set_updated_at_epoch(Some(EpochIndex::MAX));
        let widest = compiled_size(&widest, latest);
        assert!(widest > as_stored);

        for validation_mode in [ValidationMode::CheckTx, ValidationMode::RecheckTx] {
            for (cap, refused) in [(as_stored, true), (widest - 1, true), (widest, false)] {
                let mut platform_version = latest.clone();
                platform_version.system_limits.max_compiled_contract_size = Some(cap);
                let result = transform(&transition, validation_mode, &platform_version);
                if refused {
                    assert_matches!(
                        result.errors.as_slice(),
                        [ConsensusError::BasicError(
                            BasicError::DataContractCompiledMaxSizeExceededError(error)
                        )] if error.compiled_size() == u64::from(widest)
                    );
                    assert_matches!(
                        result.data,
                        Some(StateTransitionAction::BumpIdentityDataContractNonceAction(
                            _
                        ))
                    );
                } else {
                    assert_matches!(result.errors.as_slice(), []);
                    assert_matches!(
                        result.data,
                        Some(StateTransitionAction::DataContractUpdateAction(_))
                    );
                }
            }
        }
    }

    /// A block checks the cap in state validation, and the verification helper's no-validation
    /// transform checks nothing: v1 is v0 there.
    #[test]
    fn should_not_check_the_compiled_size_outside_check_tx_and_recheck() {
        let latest = PlatformVersion::latest();
        let transition = update_transition(latest);
        let mut platform_version = latest.clone();
        platform_version.system_limits.max_compiled_contract_size = Some(1);
        for validation_mode in [ValidationMode::Validator, ValidationMode::NoValidation] {
            let result = transform(&transition, validation_mode, &platform_version);
            assert_matches!(result.errors.as_slice(), []);
            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractUpdateAction(_))
            );
        }
    }
}
