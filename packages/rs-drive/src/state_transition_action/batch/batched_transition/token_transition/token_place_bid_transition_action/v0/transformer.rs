use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_place_bid_transition::v0::TokenPlaceBidTransitionV0;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::ProtocolError;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_bid_transition_action::v0::TokenPlaceBidTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::token_transition::{bump_with_errors, derive_order_id, TokenTransitionAction};
use crate::state_transition_action::batch::BatchedTransitionAction;

impl TokenPlaceBidTransitionActionV0 {
    /// Converts a borrowed `TokenPlaceBidTransitionV0` into an action. The offer's order id
    /// follows from the token, the buyer and the nonce, and its escrow is what it is worth at the
    /// token's decimals, rounded up (the transition's structure refuses an offer worth more than
    /// an order holds); one naming a token its contract
    /// does not define with `ContractHasNoTokensError` or `InvalidTokenPositionError`. The offer
    /// signs the decimals its price is at, which this conversion takes as given: the batch's
    /// advanced-structure validation refuses them with `TokenOrderDecimalsMismatchError` unless
    /// they are the token's. The order records them, and they price every sale into it. The
    /// buyer's credits are checked with the batch's fees, which count the escrow as a purchase.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_place_bid_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenPlaceBidTransitionV0,
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
        let TokenPlaceBidTransitionV0 {
            base,
            amount,
            price,
            decimals,
            min_fill,
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

        if let Err(error) = base_action.token_configuration_or_position_error() {
            return bump(vec![error], fee_result);
        }
        // The offer's escrow is what it is worth at the decimals it was signed at. The batch's
        // advanced structure refuses the offer unless they are the token's
        // (`TokenOrderDecimalsMismatchError`), as it does a listing's, before anything executes.
        let decimals = *decimals;
        // The transition's structure validation refuses an offer worth more than an order holds
        // (`InvalidTokenOrderValueError`), so this is worked out for one that fits.
        let escrow = TokenOrder::order_value(*amount, *price, decimals).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "an offer that passed structure validation is worth what an order holds",
            ),
        ))?;

        let (order_id, order_id_fee) = derive_order_id(
            drive,
            TokenOrderSide::Bid,
            base.token_id(),
            owner_id,
            base.identity_contract_nonce(),
            block_info,
            platform_version,
        )?;
        fee_result.checked_add_assign(order_id_fee)?;
        let order_document = TokenOrder {
            id: order_id,
            side: TokenOrderSide::Bid,
            maker_id: owner_id,
            token_id: base.token_id(),
            price: *price,
            amount: *amount,
            remaining: *amount,
            min_fill: *min_fill,
            escrow,
            decimals,
        }
        .new_document(block_info)?;

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::PlaceBidAction(
                TokenPlaceBidTransitionActionV0 {
                    base: base_action,
                    order_id,
                    amount: *amount,
                    price: *price,
                    min_fill: *min_fill,
                    decimals,
                    escrow,
                    order_document,
                    token_market_contract: drive
                        .token_market_contract_fetch_info(platform_version)?,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
