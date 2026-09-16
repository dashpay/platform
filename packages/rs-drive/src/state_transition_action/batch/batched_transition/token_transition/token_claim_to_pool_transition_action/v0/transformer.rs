use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_to_pool_transition_action::{TokenClaimToPoolTransitionAction, TokenClaimToPoolTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::resolve_token_claim;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::shielded::compute_shielded_verification_fee;
use dpp::identifier::Identifier;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_claim_to_pool_transition::v0::TokenClaimToPoolTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::sync::Arc;

impl TokenClaimToPoolTransitionActionV0 {
    /// Resolves the claim exactly like `TokenClaim` (shared helper), with the client's
    /// `claim_up_to` moment bounding a perpetual claim, so the amount the bundle must prove is
    /// known before the proof is verified.
    /// Resolves the version 0 transition against state into an action, or into a nonce bump
    /// carrying the consensus errors.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_claim_to_pool_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenClaimToPoolTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let TokenClaimToPoolTransitionV0 {
            base,
            distribution_type,
            claim_up_to,
            actions,
            anchor,
            proof,
            binding_signature,
            public_note,
        } = value;

        let mut drive_operations = vec![];

        let base_action_validation_result =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?;

        let mut fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        // The Halo 2 verification and per-action work GroveDB cannot meter, charged here so
        // CheckTx admission and block execution price the bundle identically, whether or not
        // the proof is (re)verified on this path.
        fee_result.checked_add_assign(FeeResult {
            processing_fee: compute_shielded_verification_fee(actions.len(), platform_version)?,
            ..Default::default()
        })?;

        let (base_action, _change_note) = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        base_action_validation_result.errors,
                    ),
                    fee_result,
                ));
            }
        };

        let (amount, distribution_info) = match resolve_token_claim(
            drive,
            owner_id,
            base,
            &base_action,
            *distribution_type,
            *claim_up_to,
            block_info,
            transaction,
            &mut fee_result,
            platform_version,
        )? {
            Ok(resolved) => resolved,
            Err(consensus_error) => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);
                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        vec![consensus_error],
                    ),
                    fee_result,
                ));
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::from(
                TokenClaimToPoolTransitionAction::V0(TokenClaimToPoolTransitionActionV0 {
                    base: base_action,
                    amount,
                    public_note: public_note.clone(),
                    distribution_info,
                    actions: actions.clone(),
                    anchor: *anchor,
                    proof: proof.clone(),
                    binding_signature: *binding_signature,
                }),
            ))
            .into(),
            fee_result,
        ))
    }
}
