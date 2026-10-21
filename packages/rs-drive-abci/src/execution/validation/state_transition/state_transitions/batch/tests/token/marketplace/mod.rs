//! Token marketplace listings (protocol version 15): a holder of a tradeable token lists an
//! amount for sale (`PlaceAsk`), which moves the tokens into escrow and records an `ask` in the
//! token market contract; a buyer takes part or all of it (`FillAsk`), paying the seller in
//! credits; the seller withdraws what is left (`CancelOrder`).
use super::*;
use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Setters;
use dpp::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Setters;
use dpp::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::prelude::{Identity, IdentityPublicKey};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use drive::drive::Drive;
use simple_signer::signer::SimpleSigner;
use std::sync::Arc;

/// 10 credits per base unit of a token with 8 decimals.
const PRICE: Credits = 1_000_000_000;

/// The basic token contract gives its owner this many base units.
const BASE_SUPPLY: u64 = 100_000;

fn make_token_tradeable(token_configuration: &mut TokenConfiguration) {
    token_configuration
        .as_v0_mut()
        .marketplace_rules
        .set_trade_mode(TokenTradeMode::Tradeable);
}

fn make_token_tradeable_and_pausable(token_configuration: &mut TokenConfiguration) {
    make_token_tradeable(token_configuration);
    token_configuration.set_emergency_action_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
        admin_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_admin_action_takers_to_no_one_allowed: false,
        self_changing_admin_action_takers_allowed: false,
    }));
}

struct Party {
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    /// The next identity contract nonce of this party for the token's contract.
    nonce: u64,
}

impl Party {
    fn id(&self) -> Identifier {
        self.identity.id()
    }

    fn take_nonce(&mut self) -> u64 {
        let nonce = self.nonce;
        self.nonce += 1;
        nonce
    }
}

struct Market {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: DataContract,
    token_id: Identifier,
    /// The contract owner, who holds the whole base supply.
    seller: Party,
    buyer: Party,
}

fn market_with(
    token_configuration_modification: impl FnOnce(&mut TokenConfiguration),
    buyer_credits: Credits,
) -> Market {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let mut rng = StdRng::seed_from_u64(70415);
    let (seller, seller_signer, seller_key) =
        setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let (buyer, buyer_signer, buyer_key) = setup_identity(&mut platform, rng.gen(), buyer_credits);

    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        seller.id(),
        Some(token_configuration_modification),
        None,
        None,
        None,
        platform_version,
    );

    Market {
        platform,
        contract,
        token_id,
        seller: Party {
            identity: seller,
            signer: seller_signer,
            key: seller_key,
            nonce: 2,
        },
        buyer: Party {
            identity: buyer,
            signer: buyer_signer,
            key: buyer_key,
            nonce: 2,
        },
    }
}

fn tradeable_market() -> Market {
    market_with(make_token_tradeable, dash_to_credits!(1.0))
}

fn at_time(time_ms: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        ..BlockInfo::default()
    }
}

impl Market {
    fn process_at(
        &mut self,
        transition: &StateTransition,
        block_info: &BlockInfo,
    ) -> StateTransitionsProcessingResult {
        let platform_version = PlatformVersion::latest();
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the batch to serialize");
        let platform_state = self.platform.state.load();
        let transaction = self.platform.drive.grove.start_transaction();
        let processing_result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                &platform_state,
                block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the batch");
        self.platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the batch");
        processing_result
    }

    fn process(&mut self, transition: &StateTransition) -> StateTransitionsProcessingResult {
        self.process_at(transition, &BlockInfo::default())
    }

    fn assert_executes(&mut self, transition: &StateTransition, block_info: &BlockInfo) {
        let processing_result = self.process_at(transition, block_info);
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
            "expected the batch to execute: {:?}",
            processing_result.execution_results()
        );
    }

    /// A listing by the seller, and its order id.
    async fn place_ask(
        &mut self,
        amount: u64,
        price: Credits,
        min_fill: u64,
        expires_at: Option<u64>,
    ) -> (StateTransition, Identifier) {
        let platform_version = PlatformVersion::latest();
        let nonce = self.seller.take_nonce();
        let order_id = TokenOrderSide::Ask.order_id(self.token_id, self.seller.id(), nonce);
        let transition = BatchTransition::new_token_place_ask_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            amount,
            price,
            min_fill,
            expires_at,
            &self.seller.key,
            nonce,
            0,
            &self.seller.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the listing");
        (transition, order_id)
    }

    /// A fill by the buyer.
    async fn fill_ask(
        &mut self,
        order_id: Identifier,
        min_amount: u64,
        max_amount: u64,
        max_total_price: Credits,
    ) -> StateTransition {
        let platform_version = PlatformVersion::latest();
        let nonce = self.buyer.take_nonce();
        BatchTransition::new_token_fill_ask_transition(
            self.token_id,
            self.buyer.id(),
            self.contract.id(),
            0,
            order_id,
            min_amount,
            max_amount,
            max_total_price,
            &self.buyer.key,
            nonce,
            0,
            &self.buyer.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the fill")
    }

    /// A fill by the seller of their own listing.
    async fn seller_fill_ask(&mut self, order_id: Identifier) -> StateTransition {
        let platform_version = PlatformVersion::latest();
        let nonce = self.seller.take_nonce();
        BatchTransition::new_token_fill_ask_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            order_id,
            1,
            u64::MAX >> 1,
            u64::MAX >> 1,
            &self.seller.key,
            nonce,
            0,
            &self.seller.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the fill")
    }

    /// A cancel by the seller, or by the buyer when `by_buyer`.
    async fn cancel(&mut self, order_id: Identifier, by_buyer: bool) -> StateTransition {
        let platform_version = PlatformVersion::latest();
        let party = if by_buyer {
            &mut self.buyer
        } else {
            &mut self.seller
        };
        let nonce = party.take_nonce();
        BatchTransition::new_token_cancel_order_transition(
            self.token_id,
            party.id(),
            self.contract.id(),
            0,
            order_id,
            TokenOrderSide::Ask,
            &party.key,
            nonce,
            0,
            &party.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the cancel")
    }

    async fn pause(&mut self) {
        let platform_version = PlatformVersion::latest();
        let nonce = self.seller.take_nonce();
        let transition = BatchTransition::new_token_emergency_action_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            TokenEmergencyAction::Pause,
            None,
            None,
            &self.seller.key,
            nonce,
            0,
            &self.seller.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the pause");
        self.assert_executes(&transition, &BlockInfo::default());
    }

    fn token_balance(&self, identity_id: Identifier) -> Option<u64> {
        self.platform
            .drive
            .fetch_identity_token_balance(
                self.token_id.to_buffer(),
                identity_id.to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the token balance")
    }

    fn credit_balance(&self, identity_id: Identifier) -> Credits {
        self.platform
            .drive
            .fetch_identity_balance(identity_id.to_buffer(), None, PlatformVersion::latest())
            .expect("expected to fetch the credit balance")
            .expect("expected the identity to have a balance")
    }

    fn escrow(&self, order_id: Identifier) -> Option<u64> {
        self.platform
            .drive
            .fetch_token_order_escrow(
                self.token_id.to_buffer(),
                order_id.to_buffer(),
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the escrow")
    }

    fn listing(&self, order_id: Identifier) -> Option<TokenOrder> {
        let platform_version = PlatformVersion::latest();
        let token_market_contract = self
            .platform
            .drive
            .token_market_contract_fetch_info(platform_version)
            .expect("expected the token market contract");
        let (document, _) = self
            .platform
            .drive
            .fetch_token_order_document(
                &token_market_contract.contract,
                TokenOrderSide::Ask,
                order_id,
                &BlockInfo::default().epoch,
                None,
                platform_version,
            )
            .expect("expected to fetch the listing");
        document.map(|document| {
            TokenOrder::from_document(&document, TokenOrderSide::Ask)
                .expect("expected the listing to read as an order")
        })
    }

    fn assert_supply_balanced(&self) {
        let total = self
            .platform
            .drive
            .calculate_total_tokens_balance(None, PlatformVersion::latest())
            .expect("expected to calculate the total tokens balance");
        assert!(
            total.ok().expect("expected the totals to add up"),
            "expected the token supply to stay balanced: {total:?}"
        );
    }

    /// Proves the executed transition the way the node answers a wait, and verifies the proof
    /// the way the SDK does.
    fn prove_and_verify(&self, transition: &StateTransition) -> StateTransitionProofResult {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the executed batch")
            .into_data()
            .expect("expected proof bytes");
        let contract = Arc::new(self.contract.clone());
        let lookup = |_id: &Identifier| Ok(Some(Arc::clone(&contract)));
        let (root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            proof.as_slice(),
            &lookup,
            platform_version,
        )
        .expect("expected the proof to verify");
        assert_ne!(root_hash, [0u8; 32]);
        outcome.into_result()
    }
}

#[tokio::test]
async fn should_move_listed_tokens_into_escrow_and_record_the_listing() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 100, Some(9_000)).await;
    market.assert_executes(&place, &BlockInfo::default());

    assert_eq!(
        market.token_balance(market.seller.id()),
        Some(BASE_SUPPLY - 1_000)
    );
    assert_eq!(market.escrow(order_id), Some(1_000));
    let listing = market.listing(order_id).expect("expected the listing");
    assert_eq!(
        listing,
        TokenOrder {
            id: order_id,
            side: TokenOrderSide::Ask,
            maker_id: market.seller.id(),
            token_id: market.token_id,
            price: PRICE,
            amount: 1_000,
            remaining: 1_000,
            min_fill: 100,
            expires_at: Some(9_000),
            escrow: 0,
        }
    );
    market.assert_supply_balanced();

    let StateTransitionProofResult::VerifiedDocuments(documents) = market.prove_and_verify(&place)
    else {
        panic!("expected the listing's document to be proved");
    };
    let document: &Document = documents
        .get(&order_id)
        .and_then(Option::as_ref)
        .expect("expected the proof to hold the listing");
    assert_eq!(
        TokenOrder::from_document(document, TokenOrderSide::Ask).expect("expected an order"),
        listing
    );
}

#[tokio::test]
async fn should_sell_part_of_a_listing_and_then_the_rest() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 100, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    let seller_credits = market.credit_balance(market.seller.id());
    let buyer_credits = market.credit_balance(market.buyer.id());

    // 400 base units at 10 credits each.
    let fill = market.fill_ask(order_id, 1, 400, 4_000).await;
    market.assert_executes(&fill, &BlockInfo::default());

    assert_eq!(market.token_balance(market.buyer.id()), Some(400));
    assert_eq!(market.escrow(order_id), Some(600));
    assert_eq!(
        market.listing(order_id).map(|listing| listing.remaining),
        Some(600)
    );
    assert_eq!(
        market.credit_balance(market.seller.id()),
        seller_credits + 4_000,
        "the seller receives the price and pays nothing for the fill"
    );
    let buyer_paid = buyer_credits - market.credit_balance(market.buyer.id());
    assert!(
        buyer_paid > 4_000,
        "the buyer pays the price and the fee, paid {buyer_paid}"
    );
    market.assert_supply_balanced();
    assert_matches!(
        market.prove_and_verify(&fill),
        StateTransitionProofResult::VerifiedTokenBalance(buyer_id, 400) if buyer_id == market.buyer.id()
    );

    // A fill may ask for more than is open: it takes what is left.
    let fill = market.fill_ask(order_id, 1, 10_000, 1_000_000).await;
    market.assert_executes(&fill, &BlockInfo::default());

    assert_eq!(market.token_balance(market.buyer.id()), Some(1_000));
    assert_eq!(
        market.token_balance(market.seller.id()),
        Some(BASE_SUPPLY - 1_000)
    );
    assert_eq!(market.escrow(order_id), None);
    assert_eq!(market.listing(order_id), None, "a sold-out listing is gone");
    // The seller receives the price, and the storage of the listing they paid for.
    let seller_gain = market.credit_balance(market.seller.id()) - seller_credits;
    assert!(
        seller_gain > 10_000,
        "expected the price and the listing's storage refund, got {seller_gain}"
    );
    market.assert_supply_balanced();
}

/// 1.5 credits per base unit: two fills of 3 cost 5 and 4, the 9 the 6 units are worth, as
/// each fill pays the change in the listing's rounded-up running total. The listing keeps one
/// unit open, so no storage refund mixes into the seller's balance.
#[tokio::test]
async fn should_charge_each_fill_the_change_in_the_running_total() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(7, 150_000_000, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    let seller_credits = market.credit_balance(market.seller.id());

    let fill = market.fill_ask(order_id, 3, 3, 5).await;
    market.assert_executes(&fill, &BlockInfo::default());
    assert_eq!(
        market.credit_balance(market.seller.id()),
        seller_credits + 5
    );

    // The second half costs 4, so a limit of 4 is enough.
    let fill = market.fill_ask(order_id, 3, 3, 4).await;
    market.assert_executes(&fill, &BlockInfo::default());
    assert_eq!(
        market.credit_balance(market.seller.id()),
        seller_credits + 9
    );
    assert_eq!(market.token_balance(market.buyer.id()), Some(6));
    assert_eq!(market.escrow(order_id), Some(1));
}

#[tokio::test]
async fn should_return_the_tokens_of_a_cancelled_listing() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    let fill = market.fill_ask(order_id, 1, 250, 2_500).await;
    market.assert_executes(&fill, &BlockInfo::default());

    let cancel = market.cancel(order_id, false).await;
    market.assert_executes(&cancel, &BlockInfo::default());

    assert_eq!(
        market.token_balance(market.seller.id()),
        Some(BASE_SUPPLY - 250)
    );
    assert_eq!(market.escrow(order_id), None);
    assert_eq!(market.listing(order_id), None);
    market.assert_supply_balanced();
    assert_matches!(
        market.prove_and_verify(&cancel),
        StateTransitionProofResult::VerifiedDocuments(documents)
            if documents.len() == 1 && documents.get(&order_id) == Some(&None)
    );
}

#[tokio::test]
async fn should_refuse_listing_a_token_that_is_not_tradeable() {
    let mut market = market_with(|_| {}, dash_to_credits!(1.0));
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    let processing_result = market.process(&place);

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: error @ ConsensusError::StateError(StateError::TokenNotTradeableError(inner)),
            ..
        }] if inner.token_id() == market.token_id && error.code() == 40750
    );
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
    assert_eq!(market.listing(order_id), None);
}

#[tokio::test]
async fn should_refuse_listing_more_than_the_seller_holds() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(BASE_SUPPLY + 1, PRICE, 1, None).await;
    let processing_result = market.process(&place);

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(
                StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
            ),
            ..
        }]
    );
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
    assert_eq!(market.listing(order_id), None);
}

#[tokio::test]
async fn should_refuse_a_listing_worth_more_than_an_order_holds() {
    let mut market = market_with(
        |token_configuration: &mut TokenConfiguration| {
            make_token_tradeable(token_configuration);
            token_configuration.conventions_mut().set_decimals(0);
        },
        dash_to_credits!(1.0),
    );
    // Two whole tokens at i64::MAX credits each.
    let (place, _) = market.place_ask(2, i64::MAX as u64, 1, None).await;
    let processing_result = market.process(&place);

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: error @ ConsensusError::StateError(StateError::TokenOrderValueTooLargeError(_)),
            ..
        }] if error.code() == 40757
    );
}

#[tokio::test]
async fn should_refuse_a_listing_that_expires_before_the_block() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, Some(5_000)).await;
    let processing_result = market.process_at(&place, &at_time(5_000));

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenOrderExpiredError(inner)),
            ..
        }] if inner.order_id() == order_id
    );
    assert_eq!(market.listing(order_id), None);
}

#[tokio::test]
async fn should_refuse_filling_an_expired_listing_but_still_cancel_it() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, Some(5_000)).await;
    market.assert_executes(&place, &at_time(1_000));

    let fill = market.fill_ask(order_id, 1, 1_000, 10_000).await;
    let processing_result = market.process_at(&fill, &at_time(5_000));
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: error @ ConsensusError::StateError(StateError::TokenOrderExpiredError(_)),
            ..
        }] if error.code() == 40752
    );
    assert_eq!(market.token_balance(market.buyer.id()), None);

    let cancel = market.cancel(order_id, false).await;
    market.assert_executes(&cancel, &at_time(5_000));
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
}

#[tokio::test]
async fn should_refuse_filling_ones_own_listing() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let fill = market.seller_fill_ask(order_id).await;
    let processing_result = market.process(&fill);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenOrderSelfFillError(_)),
            ..
        }]
    );
    assert_eq!(market.escrow(order_id), Some(1_000));
}

#[tokio::test]
async fn should_refuse_a_fill_above_the_buyers_price_limit() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let fill = market.fill_ask(order_id, 1, 400, 3_999).await;
    let processing_result = market.process(&fill);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenOrderPriceLimitError(inner)),
            ..
        }] if inner.total() == 4_000 && inner.limit() == 3_999
    );
    assert_eq!(market.escrow(order_id), Some(1_000));
}

/// A listing of 1,000 with a minimum fill of 100: a fill of 950 would leave 50 open, so the
/// fill takes 900 instead, which a buyer asking for at least 950 does not accept.
#[tokio::test]
async fn should_refuse_a_fill_the_minimum_fill_cannot_meet() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 100, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let fill = market.fill_ask(order_id, 950, 950, 1_000_000).await;
    let processing_result = market.process(&fill);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenOrderFillAmountError(_)),
            ..
        }]
    );

    // Accepting 900 is enough.
    let fill = market.fill_ask(order_id, 900, 950, 1_000_000).await;
    market.assert_executes(&fill, &BlockInfo::default());
    assert_eq!(market.token_balance(market.buyer.id()), Some(900));
    assert_eq!(market.escrow(order_id), Some(100));
}

#[tokio::test]
async fn should_refuse_filling_a_listing_that_does_not_exist() {
    let mut market = tradeable_market();
    let fill = market
        .fill_ask(Identifier::from([7; 32]), 1, 1_000, 1_000_000)
        .await;
    let processing_result = market.process(&fill);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenOrderNotFoundError(_)),
            ..
        }]
    );
}

#[tokio::test]
async fn should_refuse_cancelling_someone_elses_listing() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let cancel = market.cancel(order_id, true).await;
    let processing_result = market.process(&cancel);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenOrderNotOwnedError(_)),
            ..
        }]
    );
    assert_eq!(market.escrow(order_id), Some(1_000));
}

#[tokio::test]
async fn should_refuse_a_fill_of_a_paused_token_but_still_cancel() {
    let mut market = market_with(make_token_tradeable_and_pausable, dash_to_credits!(1.0));
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    market.pause().await;

    let fill = market.fill_ask(order_id, 1, 1_000, 10_000).await;
    let processing_result = market.process(&fill);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenIsPausedError(_)),
            ..
        }]
    );

    let cancel = market.cancel(order_id, false).await;
    market.assert_executes(&cancel, &BlockInfo::default());
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
}

#[tokio::test]
async fn should_not_fill_a_listing_the_buyer_cannot_pay_for() {
    let mut market = market_with(make_token_tradeable, dash_to_credits!(0.0001));
    // 10,000,000 credits, more than the buyer's balance.
    let (place, order_id) = market.place_ask(1_000, 1_000_000_000_000, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let fill = market.fill_ask(order_id, 1, 1_000, u64::MAX >> 1).await;
    let processing_result = market.process(&fill);
    assert!(
        !matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        ),
        "a fill the buyer cannot pay for must not execute: {:?}",
        processing_result.execution_results()
    );
    assert_eq!(market.token_balance(market.buyer.id()), None);
    assert_eq!(market.escrow(order_id), Some(1_000));
}
