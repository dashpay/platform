use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::token_pool_paid_common::{
    resolve_pooled_token, validate_credit_pool_fee_spend,
};
use dpp::consensus::state::token::{TokenMintPastMaxSupplyError, TokenNotForDirectSale};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use drive::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::required_direct_purchase_price;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::token_purchase_from_shielded_pool::TokenPurchaseFromShieldedPoolTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::token_purchase_from_shielded_pool) trait TokenPurchaseFromShieldedPoolStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl TokenPurchaseFromShieldedPoolStateTransitionTransformIntoActionValidationV0
    for TokenPurchaseFromShieldedPoolTransition
{
    /// Both Orchard proofs and the fee floor were checked by the processor. Here the token
    /// must own a pool and not be paused, the token bundle must spend recorded and unspent
    /// notes of that pool, the fee bundle must spend recorded and unspent notes of the credit
    /// pool which must hold what leaves it, and the transition's own rules apply.
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let TokenPurchaseFromShieldedPoolTransition::V0(v0) = self;
        let (contract_fetch_info, configuration) = match resolve_pooled_token(
            drive,
            v0.data_contract_id,
            v0.token_contract_position,
            v0.token_id,
            transaction,
            platform_version,
        )? {
            Ok(resolved) => resolved,
            Err(rejection) => return Ok(rejection),
        };
        // The agreed price must cover the token's direct purchase price for this count.
        let Some(pricing_schedule) = drive.fetch_token_direct_purchase_price(
            v0.token_id.to_buffer(),
            transaction,
            platform_version,
        )?
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                TokenNotForDirectSale::new(v0.token_id).into(),
            ));
        };
        if let Err(consensus_error) = required_direct_purchase_price(
            v0.token_id,
            &pricing_schedule,
            v0.token_count,
            v0.total_agreed_price,
        ) {
            return Ok(ConsensusValidationResult::new_with_error(consensus_error));
        }
        // Minting into the pool must respect the max supply.
        let total_supply = drive
            .fetch_token_total_supply(v0.token_id.to_buffer(), transaction, platform_version)?
            .unwrap_or_default();
        if let Some(max_supply) = configuration.max_supply() {
            match total_supply.checked_add(v0.token_count) {
                Some(after) if after <= max_supply => {}
                _ => {
                    return Ok(ConsensusValidationResult::new_with_error(
                        TokenMintPastMaxSupplyError::new(
                            v0.token_id,
                            v0.token_count,
                            total_supply,
                            max_supply,
                        )
                        .into(),
                    ));
                }
            }
        }
        // The credit bundle carries the price plus the fee; the fee was pinned by the
        // minimum-fee validation and the price by the schedule, so the split is exact.
        let fee_amount = v0.credit_amount - v0.total_agreed_price;
        let fee_nullifiers: Vec<[u8; 32]> = v0.fee_actions.iter().map(|a| a.nullifier).collect();
        let current_credit_pool_balance = match validate_credit_pool_fee_spend(
            drive,
            &v0.fee_anchor,
            &fee_nullifiers,
            v0.credit_amount,
            transaction,
            platform_version,
        )? {
            Ok(balance) => balance,
            Err(rejection) => return Ok(rejection),
        };
        let result = TokenPurchaseFromShieldedPoolTransitionAction::try_from_transition(
            self,
            fee_amount,
            current_credit_pool_balance,
            contract_fetch_info.contract.owner_id(),
            true,
        );
        Ok(result.map(|action| action.into()))
    }
}
