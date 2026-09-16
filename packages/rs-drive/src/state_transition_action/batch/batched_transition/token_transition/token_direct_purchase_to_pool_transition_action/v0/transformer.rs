use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_to_pool_transition_action::{TokenDirectPurchaseToPoolTransitionAction, TokenDirectPurchaseToPoolTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::resolve_direct_purchase_price;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::shielded::compute_shielded_verification_fee;
use dpp::identifier::Identifier;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_direct_purchase_to_pool_transition::v0::TokenDirectPurchaseToPoolTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::sync::Arc;

impl TokenDirectPurchaseToPoolTransitionActionV0 {
    /// Resolves the version 0 transition against state into an action, or into a nonce bump
    /// carrying the consensus errors.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_direct_purchase_to_pool_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenDirectPurchaseToPoolTransitionV0,
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
        let TokenDirectPurchaseToPoolTransitionV0 {
            base,
            token_count,
            total_agreed_price,
            actions,
            anchor,
            proof,
            binding_signature,
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

        let required_price = match resolve_direct_purchase_price(
            drive,
            base,
            *token_count,
            *total_agreed_price,
            block_info,
            transaction,
            &mut fee_result,
            platform_version,
        )? {
            Ok(required_price) => required_price,
            Err(error) => {
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
                        vec![error],
                    ),
                    fee_result,
                ));
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::from(
                TokenDirectPurchaseToPoolTransitionAction::V0(
                    TokenDirectPurchaseToPoolTransitionActionV0 {
                        base: base_action,
                        token_count: *token_count,
                        total_agreed_price: required_price,
                        actions: actions.clone(),
                        anchor: *anchor,
                        proof: proof.clone(),
                        binding_signature: *binding_signature,
                    },
                ),
            ))
            .into(),
            fee_result,
        ))
    }
}
