use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_place_bid_transition::v0::TokenPlaceBidTransitionV0;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::ProtocolError;
use dpp::consensus::state::token::TokenOrderDecimalsMismatchError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_bid_transition_action::v0::TokenPlaceBidTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenPlaceBidTransitionActionV0 {
    /// Converts a borrowed `TokenPlaceBidTransitionV0` into an action. The offer's order id
    /// follows from the token, the buyer and the nonce, and its escrow is what it is worth at the
    /// token's decimals, rounded up (the transition's structure refuses an offer worth more than
    /// an order holds); one naming a token its contract
    /// does not define with `ContractHasNoTokensError` or `InvalidTokenPositionError`. The offer
    /// signs the decimals its price is at and is refused with `TokenOrderDecimalsMismatchError`
    /// unless they are the token's; it records them, and they price every sale into it. The buyer's credits are checked with
    /// the batch's fees, which count the escrow as a purchase.
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
            expires_at,
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

        let fee_result = Drive::calculate_fee(
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

        let token_decimals = match base_action.token_configuration_or_position_error() {
            Ok(token_configuration) => token_configuration.conventions().decimals(),
            Err(error) => return bump(vec![error], fee_result),
        };
        // The offer's escrow is what it is worth at the decimals it was signed at, so they must be
        // the token's before anything is worked out from them: a change of the token's decimals
        // between signing and execution would otherwise reprice the offer.
        let decimals = *decimals;
        if decimals != token_decimals {
            return bump(
                vec![TokenOrderDecimalsMismatchError::new(
                    base.token_id(),
                    decimals,
                    token_decimals,
                )
                .into()],
                fee_result,
            );
        }
        // The transition's structure validation refuses an offer worth more than an order holds
        // (`InvalidTokenOrderValueError`), so this is worked out for one that fits.
        let escrow = TokenOrder::order_value(*amount, *price, decimals).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "an offer that passed structure validation is worth what an order holds",
            ),
        ))?;

        let order_id =
            TokenOrderSide::Bid.order_id(base.token_id(), owner_id, base.identity_contract_nonce());
        let order_document = TokenOrder {
            id: order_id,
            side: TokenOrderSide::Bid,
            maker_id: owner_id,
            token_id: base.token_id(),
            price: *price,
            amount: *amount,
            remaining: *amount,
            min_fill: *min_fill,
            expires_at: *expires_at,
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
                    expires_at: *expires_at,
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
