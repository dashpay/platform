use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::token::{
    TokenOrderFillAmountError, TokenOrderNotFoundError,
    TokenOrderPriceLimitError, TokenOrderSelfFillError,
};
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_fill_ask_transition::v0::TokenFillAskTransitionV0;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::ProtocolError;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_fill_ask_transition_action::v0::TokenFillAskTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::token_transition::{bump_with_errors, TokenTransitionAction};
use crate::state_transition_action::batch::BatchedTransitionAction;

impl TokenFillAskTransitionActionV0 {
    /// Converts a borrowed `TokenFillAskTransitionV0` into an action: reads the listing, and
    /// works out the amount the fill takes and the credits the buyer pays. A token its contract
    /// does not define, a listing that is not open for this token, the buyer's own listing, a
    /// fill range the listing cannot meet and a cost above the buyer's limit
    /// each bump the nonce with their error instead.
    ///
    /// The listing is read as state holds it, and the fill's operations write its new
    /// `remaining` and escrow as absolute values. That is sound only while a batch carries one
    /// transition (`SystemLimits::max_transitions_in_documents_batch`): two fills of one listing
    /// in a batch would both read it unfilled.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_fill_ask_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenFillAskTransitionV0,
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
        let TokenFillAskTransitionV0 {
            base,
            order_id,
            min_amount,
            max_amount,
            max_total_price,
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
            Ok(bump_with_errors(
                base,
                owner_id,
                user_fee_increase,
                errors,
                fee_result,
            ))
        };

        let (base_action, _change_note) = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => return bump(base_action_validation_result.errors, fee_result),
        };

        // Refused with the token base rules' error before anything is read for a token the
        // contract does not define.
        if let Err(error) = base_action.token_configuration_or_position_error() {
            return bump(vec![error], fee_result);
        }

        let token_market_contract = drive.token_market_contract_fetch_info(platform_version)?;
        let (order_document, read_cost) = drive.fetch_token_order_document(
            &token_market_contract.contract,
            TokenOrderSide::Ask,
            *order_id,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        fee_result.checked_add_assign(FeeResult::new_from_processing_fee(read_cost))?;

        let Some(order_document) = order_document else {
            return bump(
                vec![TokenOrderNotFoundError::new(*order_id, TokenOrderSide::Ask).into()],
                fee_result,
            );
        };
        let order = TokenOrder::from_document(&order_document, TokenOrderSide::Ask)?;
        if order.token_id != base.token_id() {
            return bump(
                vec![TokenOrderNotFoundError::new(*order_id, TokenOrderSide::Ask).into()],
                fee_result,
            );
        }
        if order.maker_id == owner_id {
            return bump(
                vec![TokenOrderSelfFillError::new(*order_id, owner_id).into()],
                fee_result,
            );
        }
        let Some(fill_amount) = order.fill_amount(*min_amount, *max_amount) else {
            return bump(
                vec![TokenOrderFillAmountError::new(
                    *order_id,
                    *min_amount,
                    *max_amount,
                    order.remaining,
                    order.min_fill,
                )
                .into()],
                fee_result,
            );
        };
        // Priced at the decimals the token had when the listing was placed, which the listing
        // records: a later change to the token's decimals does not reprice it.
        let credits = order.fill_credits(fill_amount)?;
        if credits > *max_total_price {
            return bump(
                vec![TokenOrderPriceLimitError::new(*order_id, credits, *max_total_price).into()],
                fee_result,
            );
        }

        let updated_order_document = if fill_amount < order.remaining {
            let filled_order = TokenOrder {
                remaining: order.remaining - fill_amount,
                ..order.clone()
            };
            Some(filled_order.updated_document(&order_document, block_info)?)
        } else {
            None
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::FillAskAction(
                TokenFillAskTransitionActionV0 {
                    base: base_action,
                    order,
                    fill_amount,
                    credits,
                    updated_order_document,
                    token_market_contract,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
