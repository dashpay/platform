use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_place_ask_transition::v0::TokenPlaceAskTransitionV0;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::ProtocolError;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::v0::TokenPlaceAskTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenPlaceAskTransitionActionV0 {
    /// Converts a borrowed `TokenPlaceAskTransitionV0` into an action. The listing's order id
    /// follows from the token, the seller and the nonce; everything the listing needs from state
    /// (the token's rules, the seller's balance) is checked by the state validation.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_place_ask_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenPlaceAskTransitionV0,
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
        let TokenPlaceAskTransitionV0 {
            base,
            amount,
            price,
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

        let (base_action, _change_note) = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action),
                        base_action_validation_result.errors,
                    ),
                    fee_result,
                ));
            }
        };

        let order_id =
            TokenOrderSide::Ask.order_id(base.token_id(), owner_id, base.identity_contract_nonce());
        let order_document = TokenOrder {
            id: order_id,
            side: TokenOrderSide::Ask,
            maker_id: owner_id,
            token_id: base.token_id(),
            price: *price,
            amount: *amount,
            remaining: *amount,
            min_fill: *min_fill,
            expires_at: *expires_at,
            escrow: 0,
        }
        .new_document(block_info)?;

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::PlaceAskAction(
                TokenPlaceAskTransitionActionV0 {
                    base: base_action,
                    order_id,
                    amount: *amount,
                    price: *price,
                    min_fill: *min_fill,
                    expires_at: *expires_at,
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
