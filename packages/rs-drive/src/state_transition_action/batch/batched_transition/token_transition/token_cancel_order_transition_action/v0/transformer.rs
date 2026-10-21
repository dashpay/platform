use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::token::{TokenOrderNotFoundError, TokenOrderNotOwnedError};
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_cancel_order_transition::v0::TokenCancelOrderTransitionV0;
use dpp::tokens::token_order::TokenOrder;
use dpp::ProtocolError;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_cancel_order_transition_action::v0::TokenCancelOrderTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenCancelOrderTransitionActionV0 {
    /// Converts a borrowed `TokenCancelOrderTransitionV0` into an action: reads the order, which
    /// must be open for this token and belong to the signer, else the nonce is bumped with the
    /// error. A cancel is refused for nothing else: a maker always gets back what an order holds,
    /// whether the token is paused, the maker frozen or the trade mode switched off.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_cancel_order_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenCancelOrderTransitionV0,
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
        let TokenCancelOrderTransitionV0 {
            base,
            order_id,
            side,
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

        let bump = |errors: Vec<ConsensusError>, fee_result: FeeResult| {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                    base,
                    owner_id,
                    user_fee_increase,
                );
            Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action),
                    errors,
                ),
                fee_result,
            ))
        };

        let (base_action, _change_note) = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => return bump(base_action_validation_result.errors, fee_result),
        };

        let token_market_contract = drive.token_market_contract_fetch_info(platform_version)?;
        let (order_document, read_cost) = drive.fetch_token_order_document(
            &token_market_contract.contract,
            *side,
            *order_id,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        fee_result.processing_fee = fee_result.processing_fee.saturating_add(read_cost);

        let Some(order_document) = order_document else {
            return bump(
                vec![TokenOrderNotFoundError::new(*order_id, *side).into()],
                fee_result,
            );
        };
        let order = TokenOrder::from_document(&order_document, *side)?;
        if order.token_id != base.token_id() {
            return bump(
                vec![TokenOrderNotFoundError::new(*order_id, *side).into()],
                fee_result,
            );
        }
        if order.maker_id != owner_id {
            return bump(
                vec![TokenOrderNotOwnedError::new(*order_id, order.maker_id, owner_id).into()],
                fee_result,
            );
        }
        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::CancelOrderAction(
                TokenCancelOrderTransitionActionV0 {
                    base: base_action,
                    order,
                    token_market_contract,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
