//! Token marketplace listings (protocol version 15): a holder of a tradeable token lists an
//! amount for sale (`PlaceAsk`), which moves the tokens into escrow and records an `ask` in the
//! token market contract; a buyer takes part or all of it (`FillAsk`), paying the seller in
//! credits; the seller withdraws what is left (`CancelOrder`).
use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::{
    TokenConfigurationConventionV0Getters, TokenConfigurationConventionV0Setters,
};
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Setters;
use dpp::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::group::GroupStateTransitionInfo;
use dpp::prelude::{Identity, IdentityPublicKey};
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::SystemDataContract;
use dpp::tokens::calculate_token_id;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::error::Error as DriveError;
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

    /// What check_tx refuses `transition` with.
    fn check_tx(&self, transition: &StateTransition) -> Vec<ConsensusError> {
        let platform_version = PlatformVersion::latest();
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the batch to serialize");
        let platform_state = self.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.platform.drive,
            state: &platform_state,
            config: &self.platform.config,
            core_rpc: &self.platform.core_rpc,
        };
        self.platform
            .check_tx(
                &serialized,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                platform_version,
            )
            .expect("expected to be able to check tx")
            .errors
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
        let decimals = self.token_decimals();
        let nonce = self.seller.take_nonce();
        let order_id = TokenOrderSide::Ask.order_id(self.token_id, self.seller.id(), nonce);
        let transition = BatchTransition::new_token_place_ask_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            amount,
            price,
            decimals,
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

    /// The seller, the contract owner, changes the token's decimals.
    /// The seller transfers `amount` to the buyer.
    async fn transfer_to_buyer(&mut self, amount: u64) {
        let platform_version = PlatformVersion::latest();
        let nonce = self.seller.take_nonce();
        let transfer = BatchTransition::new_token_transfer_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            amount,
            self.buyer.id(),
            None,
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
        .expect("expected to build the transfer");
        self.assert_executes(&transfer, &BlockInfo::default());
    }

    /// The seller, the contract owner, freezes the buyer's token account.
    async fn freeze_the_buyer(&mut self) {
        let platform_version = PlatformVersion::latest();
        let nonce = self.seller.take_nonce();
        let freeze = BatchTransition::new_token_freeze_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            self.buyer.id(),
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
        .expect("expected to build the freeze");
        self.assert_executes(&freeze, &BlockInfo::default());
    }

    /// A listing by the buyer at `decimals`, and its order id.
    async fn buyer_places_ask(
        &mut self,
        amount: u64,
        price: Credits,
        decimals: u8,
    ) -> (StateTransition, Identifier) {
        let platform_version = PlatformVersion::latest();
        let nonce = self.buyer.take_nonce();
        let order_id = TokenOrderSide::Ask.order_id(self.token_id, self.buyer.id(), nonce);
        let transition = BatchTransition::new_token_place_ask_transition(
            self.token_id,
            self.buyer.id(),
            self.contract.id(),
            0,
            amount,
            price,
            decimals,
            1,
            None,
            &self.buyer.key,
            nonce,
            0,
            &self.buyer.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the listing");
        (transition, order_id)
    }

    /// A fill by the seller of up to `max_amount` of the listing `order_id`.
    async fn seller_fills_ask(&mut self, order_id: Identifier, max_amount: u64) -> StateTransition {
        let platform_version = PlatformVersion::latest();
        let nonce = self.seller.take_nonce();
        BatchTransition::new_token_fill_ask_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            order_id,
            1,
            max_amount,
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

    /// Replaces the contract a client verifies with by the one stored now, as a client that
    /// fetches it again sees it.
    fn refresh_contract(&mut self) {
        let platform_version = PlatformVersion::latest();
        self.contract = self
            .platform
            .drive
            .get_contract_with_fetch_info(
                self.contract.id().to_buffer(),
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the token's contract")
            .expect("expected the token's contract")
            .contract
            .clone();
    }

    /// The token's decimals as stored now, which a placement signs.
    fn token_decimals(&self) -> u8 {
        let platform_version = PlatformVersion::latest();
        self.platform
            .drive
            .get_contract_with_fetch_info(
                self.contract.id().to_buffer(),
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the token's contract")
            .expect("expected the token's contract")
            .contract
            .expected_token_configuration(0)
            .expect("expected the token")
            .conventions()
            .decimals()
    }

    async fn set_decimals(&mut self, decimals: u8) {
        let platform_version = PlatformVersion::latest();
        let mut conventions = self
            .contract
            .tokens()
            .get(&0)
            .expect("expected the token")
            .conventions()
            .clone();
        conventions.set_decimals(decimals);
        let nonce = self.seller.take_nonce();
        let transition = BatchTransition::new_token_config_update_transition(
            self.token_id,
            self.seller.id(),
            self.contract.id(),
            0,
            TokenConfigurationChangeItem::Conventions(conventions),
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
        .expect("expected to build the conventions update");
        self.assert_executes(&transition, &BlockInfo::default());
    }

    /// A listing by the seller of the token at `position` of the contract `contract_id`, with
    /// the seller's identity contract nonce `nonce` for that contract.
    async fn place_ask_naming(
        &mut self,
        contract_id: Identifier,
        position: u16,
        nonce: u64,
    ) -> StateTransition {
        let platform_version = PlatformVersion::latest();
        BatchTransition::new_token_place_ask_transition(
            Identifier::from(calculate_token_id(contract_id.as_bytes(), position)),
            self.seller.id(),
            contract_id,
            position,
            1_000,
            PRICE,
            8,
            1,
            None,
            &self.seller.key,
            nonce,
            0,
            &self.seller.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the listing")
    }

    /// The identity contract nonce `identity_id` last used for the token's contract.
    fn contract_nonce(&self, identity_id: Identifier) -> Option<u64> {
        self.platform
            .drive
            .fetch_identity_contract_nonce(
                identity_id.to_buffer(),
                self.contract.id().to_buffer(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the nonce")
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
    /// the way the SDK does, returning the verifier's error rather than panicking on it.
    /// Proves `proved` the way the node answers a wait and verifies that proof against
    /// `claimed`, with the verifier's contract lookup answering `contract`.
    fn verify_proof_of_as(
        &self,
        proved: &StateTransition,
        claimed: &StateTransition,
        contract: &DataContract,
    ) -> Result<StateTransitionProofResult, DriveError> {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .platform
            .drive
            .prove_state_transition(proved, None, platform_version)
            .expect("expected to prove the executed batch")
            .into_data()
            .expect("expected proof bytes");
        let contract = Arc::new(contract.clone());
        let lookup = |_id: &Identifier| Ok(Some(Arc::clone(&contract)));
        Drive::verify_state_transition_was_executed_with_proof(
            claimed,
            &BlockInfo::default(),
            proof.as_slice(),
            &lookup,
            platform_version,
        )
        .map(|(root_hash, outcome)| {
            assert_eq!(
                root_hash,
                self.committed_root(),
                "the proof authenticates the committed state"
            );
            outcome.into_result()
        })
    }

    fn prove_and_try_verify(
        &self,
        transition: &StateTransition,
    ) -> Result<StateTransitionProofResult, DriveError> {
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
        Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            proof.as_slice(),
            &lookup,
            platform_version,
        )
        .map(|(root_hash, outcome)| {
            assert_eq!(
                root_hash,
                self.committed_root(),
                "the proof authenticates the committed state"
            );
            outcome.into_result()
        })
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
        assert_eq!(
            root_hash,
            self.committed_root(),
            "the proof authenticates the committed state"
        );
        outcome.into_result()
    }

    /// The root of the state committed so far, which a client authenticates a proof against.
    fn committed_root(&self) -> [u8; 32] {
        self.platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .value
            .expect("expected the committed Drive root")
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
            decimals: 8,
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

/// `transition` with group action information on its one token transition, which no executed
/// placement carries: the node refuses it on a placement.
fn with_group_info(transition: &StateTransition) -> StateTransition {
    let StateTransition::Batch(BatchTransition::V1(mut batch)) = transition.clone() else {
        panic!("expected a batch of token transitions");
    };
    let BatchedTransition::Token(token_transition) = &mut batch.transitions[0] else {
        panic!("expected a token transition");
    };
    token_transition
        .base_mut()
        .set_using_group_info(Some(GroupStateTransitionInfo {
            group_contract_position: 0,
            action_id: Identifier::from([3; 32]),
            action_is_proposer: false,
        }));
    StateTransition::Batch(BatchTransition::V1(batch))
}

/// `transition` with the base of its one token transition changed by `change`.
fn with_base(
    transition: &StateTransition,
    change: impl FnOnce(&mut TokenBaseTransition),
) -> StateTransition {
    let StateTransition::Batch(BatchTransition::V1(mut batch)) = transition.clone() else {
        panic!("expected a batch of token transitions");
    };
    let BatchedTransition::Token(token_transition) = &mut batch.transitions[0] else {
        panic!("expected a token transition");
    };
    change(token_transition.base_mut());
    StateTransition::Batch(BatchTransition::V1(batch))
}

/// A marketplace transition's proof is read by its token id, which must be the one its contract
/// and position derive. The genuine proof of an executed listing does not verify the same
/// request re-signed naming another position of a two-token contract, or another contract,
/// with the same token id: the node refuses such a request, so it never executed.
#[tokio::test]
async fn should_not_prove_a_listing_whose_token_id_its_contract_and_position_do_not_derive() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    let mut two_tokens = market.contract.clone();
    let token = market
        .contract
        .expected_token_configuration(0)
        .expect("expected the token")
        .clone();
    two_tokens.add_token(1, token);

    for claimed in [
        with_base(&place, |base| base.set_token_contract_position(1)),
        with_base(&place, |base| {
            base.set_data_contract_id(Identifier::from([9; 32]))
        }),
    ] {
        assert_matches!(
            market.verify_proof_of_as(&place, &claimed, &two_tokens),
            Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
        );
    }
    assert_matches!(
        market.verify_proof_of_as(&place, &place, &two_tokens),
        Ok(StateTransitionProofResult::VerifiedDocuments(documents))
            if documents.get(&order_id).is_some_and(Option::is_some)
    );
}

/// A placement is never a group action: the node refuses group information on it. A listing
/// that executed, re-sent with group information and otherwise the same, is refused rather
/// than proved by its `ask`; the listing itself still proves.
#[tokio::test]
async fn should_not_prove_a_listing_carrying_group_information() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    assert_matches!(
        market.prove_and_try_verify(&with_group_info(&place)),
        Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
    );
    assert_matches!(
        market.prove_and_verify(&place),
        StateTransitionProofResult::VerifiedDocuments(documents)
            if documents.get(&order_id).is_some_and(Option::is_some)
    );
}

/// A client waiting for its listing may ask for the proof after the listing was filled. Its
/// `ask` still proves the placement after a partial fill: the order id is the transition's
/// alone, and the document holds its terms with less open. A listing filled entirely is gone:
/// its `orderPlaced` record, which the token keeps as it keeps transfer history, proves it then.
#[tokio::test]
async fn should_prove_a_listing_after_it_is_partly_filled() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    let fill = market.fill_ask(order_id, 1, 400, 4_000).await;
    market.assert_executes(&fill, &BlockInfo::default());

    let StateTransitionProofResult::VerifiedDocuments(documents) = market.prove_and_verify(&place)
    else {
        panic!("expected the listing's document to be proved");
    };
    let order = TokenOrder::from_document(
        documents
            .get(&order_id)
            .and_then(Option::as_ref)
            .expect("expected the proof to hold the listing"),
        TokenOrderSide::Ask,
    )
    .expect("expected an order");
    assert_eq!((order.amount, order.remaining), (1_000, 600));

    let fill = market.fill_ask(order_id, 1, 600, 6_000).await;
    market.assert_executes(&fill, &BlockInfo::default());
    assert_matches!(
        market.prove_and_verify(&place),
        StateTransitionProofResult::VerifiedTokenActionWithDocument(document)
            if document.properties().get("orderId")
                == Some(&dpp::platform_value::Value::Identifier(order_id.to_buffer()))
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

/// 0.5 credits per base unit: of a listing of 3, the first unit costs 1 credit (its running
/// total, 0.5, rounds up to 1) and the second 0 (the total for 2 units is exactly 1). A fill
/// paying 0 credits, with a limit of 0, still takes its unit out of escrow, advances the buyer's
/// nonce and charges its fee; the seller's credits do not move. The listing keeps one unit open,
/// so no storage refund mixes into the seller's balance.
#[tokio::test]
async fn should_fill_a_unit_that_leaves_the_running_total_unchanged() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(3, 50_000_000, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    let fill = market.fill_ask(order_id, 1, 1, 1).await;
    market.assert_executes(&fill, &BlockInfo::default());
    let seller_credits = market.credit_balance(market.seller.id());
    let buyer_credits = market.credit_balance(market.buyer.id());
    let buyer_nonce = market.contract_nonce(market.buyer.id());

    let fill = market.fill_ask(order_id, 1, 1, 0).await;
    let processing_result = market.process(&fill);
    let [StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. }] =
        processing_result.execution_results().as_slice()
    else {
        panic!(
            "expected the fill to execute: {:?}",
            processing_result.execution_results()
        );
    };
    assert!(fee_result.processing_fee > 0);
    let refund = fee_result
        .fee_refunds
        .calculate_refunds_amount_for_identity(market.buyer.id())
        .unwrap_or_default();
    assert_eq!(
        market.credit_balance(market.buyer.id()) as i128,
        buyer_credits as i128 - fee_result.storage_fee as i128 - fee_result.processing_fee as i128
            + refund as i128,
        "the buyer pays the fee and nothing for the unit"
    );
    assert_eq!(market.credit_balance(market.seller.id()), seller_credits);
    assert_eq!(market.token_balance(market.buyer.id()), Some(2));
    assert_eq!(market.escrow(order_id), Some(1));
    assert_eq!(
        market.listing(order_id).map(|listing| listing.remaining),
        Some(1)
    );
    assert_ne!(market.contract_nonce(market.buyer.id()), buyer_nonce);
    market.assert_supply_balanced();
}

/// A listing of more base units than a signed 8-byte value holds is refused by the transition's
/// structure, by check_tx and in a block, before anything is charged, and nothing moves.
#[tokio::test]
async fn should_refuse_a_listing_amount_above_i64_max() {
    let mut market = tradeable_market();
    let seller_credits = market.credit_balance(market.seller.id());
    let (place, order_id) = market.place_ask(i64::MAX as u64 + 1, 1, 1, None).await;
    assert_matches!(
        market.check_tx(&place).as_slice(),
        [ConsensusError::BasicError(
            BasicError::InvalidTokenAmountError(_)
        )]
    );
    assert_matches!(
        market.process(&place).execution_results().as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        )]
    );
    assert_eq!(market.credit_balance(market.seller.id()), seller_credits);
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
    assert_eq!(market.listing(order_id), None);
}

/// Every integer of an order is a signed 8-byte value, but its `$revision` is stored as a
/// varint: the partial fill that takes a listing from revision 127 to 128 grows the `ask` by a
/// byte, which that fill's buyer pays for. The fills before it, and the ones after, add no
/// storage. The listing stays its seller's: when it closes, its storage is refunded to the
/// seller alone.
#[tokio::test]
async fn should_charge_the_fill_that_widens_a_listings_revision() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(200, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let mut fills_adding_storage = vec![];
    for fill_number in 1..=130u64 {
        let fill = market.fill_ask(order_id, 1, 1, 10).await;
        let processing_result = market.process(&fill);
        let [StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. }] =
            processing_result.execution_results().as_slice()
        else {
            panic!(
                "expected fill {fill_number} to execute: {:?}",
                processing_result.execution_results()
            );
        };
        // The first fill also creates the buyer's token balance.
        if fill_number > 1 && fee_result.storage_fee > 0 {
            fills_adding_storage.push(fill_number);
        }
    }
    // The listing was created at revision 1, so fill 127 takes it to 128.
    assert_eq!(fills_adding_storage, vec![127]);
    assert_eq!(
        market.listing(order_id).map(|listing| listing.remaining),
        Some(70)
    );

    let cancel = market.cancel(order_id, false).await;
    let processing_result = market.process(&cancel);
    let [StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. }] =
        processing_result.execution_results().as_slice()
    else {
        panic!("expected the cancel to execute");
    };
    assert!(
        fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(market.seller.id())
            .is_some_and(|refund| refund > 0),
        "the seller gets the listing's storage back"
    );
    assert_eq!(
        fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(market.buyer.id()),
        None,
        "the buyer who paid for the wider revision is not refunded: the listing is the seller's"
    );
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

    // The rule reads only the contract the action carries, so the mempool refuses it too.
    assert_matches!(
        market.check_tx(&place).as_slice(),
        [ConsensusError::StateError(
            StateError::TokenNotTradeableError(_)
        )]
    );

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

/// The contract owner (the seller) may change the token's conventions and freeze accounts.
fn make_token_tradeable_with_owner_conventions_and_freezes(
    token_configuration: &mut TokenConfiguration,
) {
    make_token_tradeable(token_configuration);
    let owner_rules = ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
        admin_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_admin_action_takers_to_no_one_allowed: false,
        self_changing_admin_action_takers_allowed: false,
    });
    token_configuration.set_conventions_change_rules(owner_rules.clone());
    token_configuration.set_freeze_rules(owner_rules);
}

/// A listing's price is per whole token, so the decimals set what it is worth: 1,000 base
/// units at 1,000,000,000 credits per whole token are 10,000 credits at 8 decimals and 1 credit
/// at 16. A holder signs a listing at the token's 8 decimals; the identity controlling the
/// token's conventions (the contract owner, not the holder) changes them to 16 before the
/// listing executes. The listing is refused with `TokenOrderDecimalsMismatchError`, by check_tx
/// and in a block, and nothing moves into escrow.
#[tokio::test]
async fn should_refuse_a_listing_signed_at_other_decimals() {
    let mut market = market_with(
        make_token_tradeable_with_owner_conventions_and_freezes,
        dash_to_credits!(1.0),
    );
    market.transfer_to_buyer(5_000).await;
    let (listing, order_id) = market.buyer_places_ask(1_000, 1_000_000_000, 8).await;

    market.set_decimals(16).await;

    assert_matches!(
        market.check_tx(&listing).as_slice(),
        [ConsensusError::StateError(StateError::TokenOrderDecimalsMismatchError(error))]
            if error.signed_decimals() == 8 && error.token_decimals() == 16
    );
    assert_matches!(
        market.process(&listing).execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: error @ ConsensusError::StateError(
                StateError::TokenOrderDecimalsMismatchError(inner)
            ),
            ..
        }] if inner.token_id() == market.token_id && error.code() == 40758
    );
    assert_eq!(market.token_balance(market.buyer.id()), Some(5_000));
    assert_eq!(market.escrow(order_id), None);
    assert_eq!(market.listing(order_id), None);
}

/// A listing whose maker's account is frozen cannot be filled: the fill is refused with the
/// maker's `IdentityTokenAccountFrozenError` and nothing moves. The frozen maker can still
/// cancel it, and gets back what it holds in escrow.
#[tokio::test]
async fn should_refuse_filling_a_frozen_makers_listing_and_let_the_maker_cancel() {
    let mut market = market_with(
        make_token_tradeable_with_owner_conventions_and_freezes,
        dash_to_credits!(1.0),
    );
    market.transfer_to_buyer(5_000).await;
    let (place, order_id) = market.buyer_places_ask(1_000, PRICE, 8).await;
    market.assert_executes(&place, &BlockInfo::default());
    market.freeze_the_buyer().await;

    let fill = market.seller_fills_ask(order_id, 400).await;
    assert_matches!(
        market.process(&fill).execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(error)),
            ..
        }] if *error.identity_id() == market.buyer.id() && *error.token_id() == market.token_id
    );
    assert_eq!(market.escrow(order_id), Some(1_000));
    assert_eq!(market.token_balance(market.buyer.id()), Some(4_000));
    assert_eq!(
        market.token_balance(market.seller.id()),
        Some(BASE_SUPPLY - 5_000)
    );

    let cancel = market.cancel(order_id, true).await;
    market.assert_executes(&cancel, &BlockInfo::default());
    assert_eq!(market.escrow(order_id), None);
    assert_eq!(market.listing(order_id), None);
    assert_eq!(market.token_balance(market.buyer.id()), Some(5_000));
}

/// A buyer whose account is frozen cannot buy a listing unless the token allows transfers to
/// frozen balances: then the fill executes and the tokens land in the frozen balance.
#[tokio::test]
async fn should_refuse_a_frozen_buyers_fill_unless_the_token_allows_transfers_to_frozen_balances() {
    for allowed in [false, true] {
        let mut market = market_with(
            |token_configuration: &mut TokenConfiguration| {
                make_token_tradeable_with_owner_conventions_and_freezes(token_configuration);
                token_configuration.allow_transfer_to_frozen_balance(allowed);
            },
            dash_to_credits!(1.0),
        );
        let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        market.freeze_the_buyer().await;

        let fill = market.fill_ask(order_id, 1, 400, 4_000).await;
        let processing_result = market.process(&fill);
        if allowed {
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            assert_eq!(market.token_balance(market.buyer.id()), Some(400));
            assert_eq!(market.escrow(order_id), Some(600));
        } else {
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::IdentityTokenAccountFrozenError(error)
                    ),
                    ..
                }] if *error.identity_id() == market.buyer.id()
            );
            assert_eq!(market.token_balance(market.buyer.id()), None);
            assert_eq!(market.escrow(order_id), Some(1_000));
        }
    }
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

/// What a listing is worth at the decimals it signs is checked with the transition's structure:
/// two whole tokens at `i64::MAX` credits each are refused by check_tx and in a block before
/// anything is charged, and nothing moves.
#[tokio::test]
async fn should_refuse_a_listing_worth_more_than_an_order_holds() {
    let mut market = market_with(
        |token_configuration: &mut TokenConfiguration| {
            make_token_tradeable(token_configuration);
            token_configuration.conventions_mut().set_decimals(0);
        },
        dash_to_credits!(1.0),
    );
    let seller_credits = market.credit_balance(market.seller.id());
    let (place, order_id) = market.place_ask(2, i64::MAX as u64, 1, None).await;
    assert_matches!(
        market.check_tx(&place).as_slice(),
        [ConsensusError::BasicError(BasicError::InvalidTokenOrderValueError(error))]
            if error.amount() == 2 && error.decimals() == 0
    );
    let processing_result = market.process(&place);

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            error @ ConsensusError::BasicError(BasicError::InvalidTokenOrderValueError(_))
        )] if error.code() == 10465
    );
    assert_eq!(market.credit_balance(market.seller.id()), seller_credits);
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
    assert_eq!(market.listing(order_id), None);
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

/// The batch's balance check counts what a fill pays: a buyer whose credits cover the price but
/// not the fee, and one whose credits do not cover the price, are both refused before anything
/// is charged, and neither the buyer, the listing nor the buyer's nonce changes.
#[tokio::test]
async fn should_not_fill_a_listing_the_buyer_cannot_pay_for() {
    let mut market = market_with(make_token_tradeable, dash_to_credits!(0.0001));
    let buyer_credits = market.credit_balance(market.buyer.id());
    // 10,000,000 credits for the whole listing: the buyer's balance, short of the fee.
    let (place, at_balance) = market.place_ask(1_000, 1_000_000_000_000, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    // 20,000,000 credits: twice the buyer's balance.
    let (place, above_balance) = market.place_ask(1_000, 2_000_000_000_000, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    assert_eq!(buyer_credits, 10_000_000);
    let buyer_nonce = market.contract_nonce(market.buyer.id());

    for order_id in [at_balance, above_balance] {
        let fill = market.fill_ask(order_id, 1, 1_000, u64::MAX >> 1).await;
        let processing_result = market.process(&fill);
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(_))
            )],
            "a fill the buyer cannot pay for is refused unpaid: {:?}",
            processing_result.execution_results()
        );
        assert_eq!(market.credit_balance(market.buyer.id()), buyer_credits);
        assert_eq!(market.contract_nonce(market.buyer.id()), buyer_nonce);
        assert_eq!(market.token_balance(market.buyer.id()), None);
        assert_eq!(market.escrow(order_id), Some(1_000));
        assert_eq!(
            market.listing(order_id).map(|listing| listing.remaining),
            Some(1_000)
        );
    }
}

/// A listing records the decimals its token had when it was placed and every fill prices it at
/// them. The token changes from 8 decimals to 2 between two fills: priced at 2, 100 base units
/// of the listing would cost 1,000,000,000 credits, and the buyer's limit of 1,000 would refuse
/// the fill; priced at the listing's 8, they cost 1,000.
#[tokio::test]
async fn should_keep_pricing_a_listing_at_the_decimals_it_was_placed_at() {
    let mut market = market_with(
        |token_configuration: &mut TokenConfiguration| {
            make_token_tradeable(token_configuration);
            token_configuration.set_conventions_change_rules(ChangeControlRules::V0(
                ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                },
            ));
        },
        dash_to_credits!(1.0),
    );
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    assert_eq!(
        market.listing(order_id).map(|listing| listing.decimals),
        Some(8)
    );
    let seller_credits = market.credit_balance(market.seller.id());

    let fill = market.fill_ask(order_id, 100, 100, 1_000).await;
    market.assert_executes(&fill, &BlockInfo::default());
    assert_eq!(
        market.credit_balance(market.seller.id()),
        seller_credits + 1_000
    );

    market.set_decimals(2).await;
    let seller_credits = market.credit_balance(market.seller.id());

    let fill = market.fill_ask(order_id, 100, 100, 1_000).await;
    market.assert_executes(&fill, &BlockInfo::default());
    assert_eq!(
        market.credit_balance(market.seller.id()),
        seller_credits + 1_000,
        "the fill is priced at the listing's decimals"
    );
    assert_eq!(market.token_balance(market.buyer.id()), Some(200));
    let listing = market.listing(order_id).expect("expected the listing");
    assert_eq!((listing.remaining, listing.decimals), (800, 8));

    // A listing placed after the change records the new decimals.
    let (place, newer) = market.place_ask(10, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());
    assert_eq!(
        market.listing(newer).map(|listing| listing.decimals),
        Some(2)
    );
    market.assert_supply_balanced();
}

/// A listing's expiry is stored as a signed 8-byte value: `i64::MAX` is a valid expiry, and one
/// more is refused by the structure validation, in check_tx and in a block, before anything is
/// charged.
#[tokio::test]
async fn should_refuse_a_listing_expiry_above_i64_max() {
    let mut market = tradeable_market();
    let latest = i64::MAX as u64;

    let (place, order_id) = market.place_ask(1_000, PRICE, 1, Some(latest)).await;
    assert!(market.check_tx(&place).is_empty());
    market.assert_executes(&place, &BlockInfo::default());
    assert_eq!(
        market
            .listing(order_id)
            .and_then(|listing| listing.expires_at),
        Some(latest)
    );

    let seller_credits = market.credit_balance(market.seller.id());
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, Some(latest + 1)).await;
    assert_matches!(
        market.check_tx(&place).as_slice(),
        [ConsensusError::BasicError(BasicError::InvalidTokenOrderExpiryError(error))]
            if error.expires_at() == latest + 1
    );
    let processing_result = market.process(&place);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            error @ ConsensusError::BasicError(BasicError::InvalidTokenOrderExpiryError(_))
        )] if error.code() == 10464
    );
    assert_eq!(market.listing(order_id), None);
    assert_eq!(market.credit_balance(market.seller.id()), seller_credits);
    assert_eq!(
        market.token_balance(market.seller.id()),
        Some(BASE_SUPPLY - 1_000)
    );
}

/// A listing naming a token its contract does not define is refused with the token base
/// rules' error, charged, in check_tx and in a block: a position past the contract's last
/// token, and a contract without tokens (DPNS).
#[tokio::test]
async fn should_refuse_a_listing_of_a_token_the_contract_does_not_define() {
    let mut market = tradeable_market();
    let contract_id = market.contract.id();
    let nonce = market.seller.take_nonce();
    let place = market.place_ask_naming(contract_id, 1, nonce).await;
    assert_matches!(
        market.check_tx(&place).as_slice(),
        [ConsensusError::BasicError(BasicError::InvalidTokenPositionError(error))]
            if error.invalid_token_position() == 1
    );
    let processing_result = market.process(&place);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::InvalidTokenPositionError(_)),
            ..
        }]
    );

    let dpns_id = SystemDataContract::DPNS.id();
    let place = market.place_ask_naming(dpns_id, 0, 1).await;
    assert_matches!(
        market.check_tx(&place).as_slice(),
        [ConsensusError::BasicError(
            BasicError::ContractHasNoTokensError(_)
        )]
    );
    let processing_result = market.process(&place);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::ContractHasNoTokensError(_)),
            ..
        }]
    );
    assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
}

/// A fill naming a token its contract does not define is refused with the token base rules'
/// error before the listing is read, charged, in check_tx and in a block.
#[tokio::test]
async fn should_refuse_a_fill_of_a_token_the_contract_does_not_define() {
    let mut market = tradeable_market();
    let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
    market.assert_executes(&place, &BlockInfo::default());

    let platform_version = PlatformVersion::latest();
    let nonce = market.buyer.take_nonce();
    let fill = BatchTransition::new_token_fill_ask_transition(
        Identifier::from(calculate_token_id(market.contract.id().as_bytes(), 1)),
        market.buyer.id(),
        market.contract.id(),
        1,
        order_id,
        1,
        1_000,
        u64::MAX >> 1,
        &market.buyer.key,
        nonce,
        0,
        &market.buyer.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to build the fill");
    assert_matches!(
        market.check_tx(&fill).as_slice(),
        [ConsensusError::BasicError(
            BasicError::InvalidTokenPositionError(_)
        )]
    );
    assert_matches!(
        market.process(&fill).execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::InvalidTokenPositionError(_)),
            ..
        }]
    );
    assert_eq!(market.escrow(order_id), Some(1_000));
}

/// Offers: a buyer offers credits for an amount of a tradeable token (`PlaceBid`), which moves
/// what the offer is worth from their balance into escrow and records a `bid`; a holder sells
/// into it (`FillBid`), receiving credits out of the escrow; the buyer withdraws what is left
/// (`CancelOrder` with the `bid` side).
mod offers {
    use super::*;
    use dpp::fee::fee_result::FeeResult;

    impl Market {
        /// Executes `transition` and returns the fee its signer was charged.
        fn executes(&mut self, transition: &StateTransition) -> FeeResult {
            let processing_result = self.process(transition);
            match processing_result.execution_results().as_slice() {
                [StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. }] => {
                    fee_result.clone()
                }
                other => panic!("expected the batch to execute: {other:?}"),
            }
        }

        /// An offer by the buyer, and its order id.
        pub(super) async fn place_bid(
            &mut self,
            amount: u64,
            price: Credits,
            min_fill: u64,
            expires_at: Option<u64>,
        ) -> (StateTransition, Identifier) {
            let platform_version = PlatformVersion::latest();
            let decimals = self.token_decimals();
            let nonce = self.buyer.take_nonce();
            let order_id = TokenOrderSide::Bid.order_id(self.token_id, self.buyer.id(), nonce);
            let transition = BatchTransition::new_token_place_bid_transition(
                self.token_id,
                self.buyer.id(),
                self.contract.id(),
                0,
                amount,
                price,
                decimals,
                min_fill,
                expires_at,
                &self.buyer.key,
                nonce,
                0,
                &self.buyer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected to build the offer");
            (transition, order_id)
        }

        /// A sale into an offer, by the seller, or by the buyer into their own offer when
        /// `by_buyer`.
        pub(super) async fn fill_bid(
            &mut self,
            order_id: Identifier,
            min_amount: u64,
            max_amount: u64,
            min_total_received: Credits,
            by_buyer: bool,
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let party = if by_buyer {
                &mut self.buyer
            } else {
                &mut self.seller
            };
            let nonce = party.take_nonce();
            BatchTransition::new_token_fill_bid_transition(
                self.token_id,
                party.id(),
                self.contract.id(),
                0,
                order_id,
                min_amount,
                max_amount,
                min_total_received,
                &party.key,
                nonce,
                0,
                &party.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected to build the sale")
        }

        /// A cancel of an offer, by the buyer, or by the seller when `by_seller`.
        pub(super) async fn cancel_bid(
            &mut self,
            order_id: Identifier,
            by_seller: bool,
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let party = if by_seller {
                &mut self.seller
            } else {
                &mut self.buyer
            };
            let nonce = party.take_nonce();
            BatchTransition::new_token_cancel_order_transition(
                self.token_id,
                party.id(),
                self.contract.id(),
                0,
                order_id,
                TokenOrderSide::Bid,
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

        /// An offer by the buyer for the token at `position` of the contract `contract_id`,
        /// with the buyer's identity contract nonce `nonce` for that contract.
        async fn place_bid_naming(
            &mut self,
            contract_id: Identifier,
            position: u16,
            nonce: u64,
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            BatchTransition::new_token_place_bid_transition(
                Identifier::from(calculate_token_id(contract_id.as_bytes(), position)),
                self.buyer.id(),
                contract_id,
                position,
                1_000,
                PRICE,
                8,
                1,
                None,
                &self.buyer.key,
                nonce,
                0,
                &self.buyer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected to build the offer")
        }

        /// The seller, the contract owner, freezes `identity_id`'s token account.
        async fn freeze_account(&mut self, identity_id: Identifier) {
            let platform_version = PlatformVersion::latest();
            let nonce = self.seller.take_nonce();
            let freeze = BatchTransition::new_token_freeze_transition(
                self.token_id,
                self.seller.id(),
                self.contract.id(),
                0,
                identity_id,
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
            .expect("expected to build the freeze");
            self.executes(&freeze);
        }

        fn credit_escrow(&self, order_id: Identifier) -> Option<Credits> {
            self.platform
                .drive
                .fetch_credit_order_escrow(
                    order_id.to_buffer(),
                    None,
                    &mut vec![],
                    PlatformVersion::latest(),
                )
                .expect("expected to fetch the escrow")
        }

        fn offer(&self, order_id: Identifier) -> Option<TokenOrder> {
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
                    TokenOrderSide::Bid,
                    order_id,
                    &BlockInfo::default().epoch,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the offer");
            document.map(|document| {
                TokenOrder::from_document(&document, TokenOrderSide::Bid)
                    .expect("expected the offer to read as an order")
            })
        }
    }

    /// What `identity_id`'s balance changes by when it signs a transition charged `fee` and
    /// receives `received` credits: the credits less the fee, plus the storage it gets back.
    fn change_for(fee: &FeeResult, identity_id: Identifier, received: Credits) -> i128 {
        let refund = fee
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity_id)
            .unwrap_or_default();
        received as i128 + refund as i128 - fee.storage_fee as i128 - fee.processing_fee as i128
    }

    fn change(before: Credits, after: Credits) -> i128 {
        after as i128 - before as i128
    }

    #[tokio::test]
    async fn should_hold_what_an_offer_is_worth_in_escrow_and_record_the_offer() {
        let mut market = tradeable_market();
        let buyer_credits = market.credit_balance(market.buyer.id());
        let (place, order_id) = market.place_bid(1_000, PRICE, 100, Some(9_000)).await;
        let fee = market.executes(&place);

        // 1,000 base units at 10 credits each.
        assert_eq!(market.credit_escrow(order_id), Some(10_000));
        assert_eq!(
            change(buyer_credits, market.credit_balance(market.buyer.id())),
            change_for(&fee, market.buyer.id(), 0) - 10_000,
            "the buyer pays the escrow and the fee"
        );
        let offer = market.offer(order_id).expect("expected the offer");
        assert_eq!(
            offer,
            TokenOrder {
                id: order_id,
                side: TokenOrderSide::Bid,
                maker_id: market.buyer.id(),
                token_id: market.token_id,
                price: PRICE,
                amount: 1_000,
                remaining: 1_000,
                min_fill: 100,
                expires_at: Some(9_000),
                escrow: 10_000,
                decimals: 8,
            }
        );

        let StateTransitionProofResult::VerifiedDocuments(documents) =
            market.prove_and_verify(&place)
        else {
            panic!("expected the offer's document to be proved");
        };
        let document = documents
            .get(&order_id)
            .and_then(Option::as_ref)
            .expect("expected the proof to hold the offer");
        assert_eq!(
            TokenOrder::from_document(document, TokenOrderSide::Bid).expect("expected an order"),
            offer
        );
    }

    /// An offer's `bid` still proves its placement after a sale into it: the order id is the
    /// transition's alone, and the document holds its terms with less open and the escrow less
    /// what was paid. An offer filled entirely is gone: its `orderPlaced` record proves it then.
    #[tokio::test]
    async fn should_prove_an_offer_after_it_is_partly_filled() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        let sale = market.fill_bid(order_id, 1, 400, 4_000, false).await;
        market.executes(&sale);

        let StateTransitionProofResult::VerifiedDocuments(documents) =
            market.prove_and_verify(&place)
        else {
            panic!("expected the offer's document to be proved");
        };
        let order = TokenOrder::from_document(
            documents
                .get(&order_id)
                .and_then(Option::as_ref)
                .expect("expected the proof to hold the offer"),
            TokenOrderSide::Bid,
        )
        .expect("expected an order");
        assert_eq!(
            (order.amount, order.remaining, order.escrow),
            (1_000, 600, 6_000)
        );

        let sale = market.fill_bid(order_id, 1, 600, 1, false).await;
        market.executes(&sale);
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedTokenActionWithDocument(document)
                if document.properties().get("orderId")
                    == Some(&dpp::platform_value::Value::Identifier(order_id.to_buffer()))
        );
    }

    #[tokio::test]
    async fn should_sell_into_part_of_an_offer_and_then_the_rest() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 100, None).await;
        market.executes(&place);
        let seller_credits = market.credit_balance(market.seller.id());
        let buyer_credits = market.credit_balance(market.buyer.id());

        // 400 base units at 10 credits each, out of the escrow.
        let sale = market.fill_bid(order_id, 1, 400, 4_000, false).await;
        let fee = market.executes(&sale);

        assert_eq!(market.token_balance(market.buyer.id()), Some(400));
        assert_eq!(
            market.token_balance(market.seller.id()),
            Some(BASE_SUPPLY - 400)
        );
        assert_eq!(market.credit_escrow(order_id), Some(6_000));
        let offer = market.offer(order_id).expect("expected the offer");
        assert_eq!((offer.remaining, offer.escrow), (600, 6_000));
        assert_eq!(
            change(seller_credits, market.credit_balance(market.seller.id())),
            change_for(&fee, market.seller.id(), 4_000),
            "the seller receives the price and pays the fee"
        );
        assert_eq!(
            market.credit_balance(market.buyer.id()),
            buyer_credits,
            "the buyer paid with the escrow"
        );
        market.assert_supply_balanced();
        assert_matches!(
            market.prove_and_verify(&sale),
            StateTransitionProofResult::VerifiedTokenBalance(seller_id, balance)
                if seller_id == market.seller.id() && balance == BASE_SUPPLY - 400
        );

        // A sale may offer more than is still wanted: it takes what is left.
        let seller_credits = market.credit_balance(market.seller.id());
        let sale = market.fill_bid(order_id, 1, 10_000, 1, false).await;
        let fee = market.executes(&sale);

        assert_eq!(market.token_balance(market.buyer.id()), Some(1_000));
        assert_eq!(market.credit_escrow(order_id), None);
        assert_eq!(market.offer(order_id), None, "a filled offer is gone");
        assert_eq!(
            change(seller_credits, market.credit_balance(market.seller.id())),
            change_for(&fee, market.seller.id(), 6_000)
        );
        // The buyer gets the offer's storage back, and nothing is left in its escrow.
        assert_eq!(
            change(buyer_credits, market.credit_balance(market.buyer.id())),
            fee.fee_refunds
                .calculate_refunds_amount_for_identity(market.buyer.id())
                .unwrap_or_default() as i128
        );
        market.assert_supply_balanced();
    }

    /// 1.5 credits per base unit: an offer for 7 holds 11 (10.5 rounded up). Sales of 3, 3 and
    /// 1 are paid 4, 5 and 1, the change in the offer's running total rounded down, and the 1
    /// credit the rounding left goes back to the buyer when the offer closes.
    #[tokio::test]
    async fn should_pay_each_sale_the_change_in_the_running_total_and_return_the_rest() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(7, 150_000_000, 1, None).await;
        market.executes(&place);
        assert_eq!(market.credit_escrow(order_id), Some(11));
        let buyer_credits = market.credit_balance(market.buyer.id());

        let mut escrows = vec![];
        for (amount, paid) in [(3, 4), (3, 5)] {
            let seller_credits = market.credit_balance(market.seller.id());
            let sale = market.fill_bid(order_id, amount, amount, paid, false).await;
            let fee = market.executes(&sale);
            assert_eq!(
                change(seller_credits, market.credit_balance(market.seller.id())),
                change_for(&fee, market.seller.id(), paid)
            );
            escrows.push(market.credit_escrow(order_id));
        }
        assert_eq!(escrows, vec![Some(7), Some(2)]);

        let seller_credits = market.credit_balance(market.seller.id());
        let sale = market.fill_bid(order_id, 1, 1, 1, false).await;
        let fee = market.executes(&sale);
        assert_eq!(
            change(seller_credits, market.credit_balance(market.seller.id())),
            change_for(&fee, market.seller.id(), 1)
        );
        assert_eq!(market.credit_escrow(order_id), None);
        assert_eq!(
            change(buyer_credits, market.credit_balance(market.buyer.id())),
            1 + fee
                .fee_refunds
                .calculate_refunds_amount_for_identity(market.buyer.id())
                .unwrap_or_default() as i128,
            "the buyer gets back the credit the rounding left, and the offer's storage"
        );
        assert_eq!(market.token_balance(market.buyer.id()), Some(7));
    }

    #[tokio::test]
    async fn should_return_the_escrow_of_a_cancelled_offer() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        let sale = market.fill_bid(order_id, 1, 250, 2_500, false).await;
        market.executes(&sale);
        let buyer_credits = market.credit_balance(market.buyer.id());

        let cancel = market.cancel_bid(order_id, false).await;
        let fee = market.executes(&cancel);

        assert_eq!(market.credit_escrow(order_id), None);
        assert_eq!(market.offer(order_id), None);
        assert_eq!(
            change(buyer_credits, market.credit_balance(market.buyer.id())),
            change_for(&fee, market.buyer.id(), 7_500),
            "the buyer gets back what the escrow still held"
        );
        assert_eq!(market.token_balance(market.buyer.id()), Some(250));
        assert_matches!(
            market.prove_and_verify(&cancel),
            StateTransitionProofResult::VerifiedDocuments(documents)
                if documents.len() == 1 && documents.get(&order_id) == Some(&None)
        );
    }

    #[tokio::test]
    async fn should_refuse_an_offer_for_a_token_that_is_not_tradeable() {
        let mut market = market_with(|_| {}, dash_to_credits!(1.0));
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        assert_matches!(
            market.check_tx(&place).as_slice(),
            [ConsensusError::StateError(
                StateError::TokenNotTradeableError(_)
            )]
        );
        assert_matches!(
            market.process(&place).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenNotTradeableError(_)),
                ..
            }]
        );
        assert_eq!(market.credit_escrow(order_id), None);
    }

    #[tokio::test]
    async fn should_refuse_an_offer_worth_more_than_an_order_holds() {
        let mut market = market_with(
            |token_configuration: &mut TokenConfiguration| {
                make_token_tradeable(token_configuration);
                token_configuration.conventions_mut().set_decimals(0);
            },
            dash_to_credits!(1.0),
        );
        // Two whole tokens at i64::MAX credits each: refused by the transition's structure,
        // before anything is charged.
        let buyer_credits = market.credit_balance(market.buyer.id());
        let (place, order_id) = market.place_bid(2, i64::MAX as u64, 1, None).await;
        assert_matches!(
            market.check_tx(&place).as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenOrderValueError(error))]
                if error.amount() == 2 && error.decimals() == 0
        );
        assert_matches!(
            market.process(&place).execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                error @ ConsensusError::BasicError(BasicError::InvalidTokenOrderValueError(_))
            )] if error.code() == 10465
        );
        assert_eq!(market.credit_balance(market.buyer.id()), buyer_credits);
        assert_eq!(market.credit_escrow(order_id), None);
    }

    /// The batch's balance check counts an offer's escrow: a buyer whose credits cover the
    /// escrow but not the fee, and one whose credits do not cover the escrow, are both refused
    /// before anything is charged, and neither the buyer's credits nor their nonce changes.
    #[tokio::test]
    async fn should_not_place_an_offer_the_buyer_cannot_pay_for() {
        let mut market = market_with(make_token_tradeable, dash_to_credits!(0.0001));
        let buyer_credits = market.credit_balance(market.buyer.id());
        assert_eq!(buyer_credits, 10_000_000);
        let buyer_nonce = market.contract_nonce(market.buyer.id());

        // 1,000 base units at 10,000 credits each: an escrow of 10,000,000 credits, the buyer's
        // balance, short of the fee; then twice that, short of the escrow itself.
        for price in [1_000_000_000_000, 2_000_000_000_000] {
            let (place, order_id) = market.place_bid(1_000, price, 1, None).await;
            let processing_result = market.process(&place);
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(_))
                )],
                "an offer the buyer cannot pay for is refused unpaid: {:?}",
                processing_result.execution_results()
            );
            assert_eq!(market.credit_balance(market.buyer.id()), buyer_credits);
            assert_eq!(market.contract_nonce(market.buyer.id()), buyer_nonce);
            assert_eq!(market.credit_escrow(order_id), None);
            assert_eq!(market.offer(order_id), None);
        }
    }

    /// An offer records the decimals its token had when it was placed: its escrow was worked
    /// out at them and every sale into it is paid at them. The token changes from 8 decimals to
    /// 2 after a partial sale: paid at 2, 100 base units would take 1,000,000,000 credits out of
    /// an escrow of 9,000; paid at the offer's 8 they take 1,000, and the rest of the offer sells
    /// for exactly what is left in escrow.
    #[tokio::test]
    async fn should_keep_paying_an_offer_at_the_decimals_it_was_placed_at() {
        let mut market = market_with(
            |token_configuration: &mut TokenConfiguration| {
                make_token_tradeable(token_configuration);
                token_configuration.set_conventions_change_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            },
            dash_to_credits!(1.0),
        );
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        assert_eq!(market.credit_escrow(order_id), Some(10_000));

        let seller_credits = market.credit_balance(market.seller.id());
        let sale = market.fill_bid(order_id, 100, 100, 1_000, false).await;
        let fee = market.executes(&sale);
        assert_eq!(
            change(seller_credits, market.credit_balance(market.seller.id())),
            change_for(&fee, market.seller.id(), 1_000)
        );

        market.set_decimals(2).await;

        let seller_credits = market.credit_balance(market.seller.id());
        let sale = market.fill_bid(order_id, 100, 100, 1_000, false).await;
        let fee = market.executes(&sale);
        assert_eq!(
            change(seller_credits, market.credit_balance(market.seller.id())),
            change_for(&fee, market.seller.id(), 1_000),
            "the sale is paid at the offer's decimals"
        );
        let offer = market.offer(order_id).expect("expected the offer");
        assert_eq!(
            (offer.remaining, offer.escrow, offer.decimals),
            (800, 8_000, 8)
        );

        let seller_credits = market.credit_balance(market.seller.id());
        let sale = market.fill_bid(order_id, 800, 800, 8_000, false).await;
        let fee = market.executes(&sale);
        assert_eq!(
            change(seller_credits, market.credit_balance(market.seller.id())),
            change_for(&fee, market.seller.id(), 8_000)
        );
        assert_eq!(market.credit_escrow(order_id), None);
        assert_eq!(market.offer(order_id), None);
        assert_eq!(market.token_balance(market.buyer.id()), Some(1_000));
        market.assert_supply_balanced();

        // An offer placed after the change records the new decimals and holds what it is worth
        // at them: 10 base units at 1,000,000,000 credits per 100.
        let (place, newer) = market.place_bid(10, PRICE, 1, None).await;
        market.executes(&place);
        let newer = market.offer(newer).expect("expected the offer");
        assert_eq!((newer.decimals, newer.escrow), (2, 100_000_000));
    }

    /// An offer's expiry is stored as a signed 8-byte value: `i64::MAX` is valid, and one more
    /// is refused by the structure validation, in check_tx and in a block, before anything is
    /// charged.
    #[tokio::test]
    async fn should_refuse_an_offer_expiry_above_i64_max() {
        let mut market = tradeable_market();
        let latest = i64::MAX as u64;

        let (place, order_id) = market.place_bid(1_000, PRICE, 1, Some(latest)).await;
        assert!(market.check_tx(&place).is_empty());
        market.executes(&place);
        assert_eq!(
            market.offer(order_id).and_then(|offer| offer.expires_at),
            Some(latest)
        );

        let buyer_credits = market.credit_balance(market.buyer.id());
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, Some(latest + 1)).await;
        assert_matches!(
            market.check_tx(&place).as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenOrderExpiryError(error))]
                if error.expires_at() == latest + 1
        );
        assert_matches!(
            market.process(&place).execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                error @ ConsensusError::BasicError(BasicError::InvalidTokenOrderExpiryError(_))
            )] if error.code() == 10464
        );
        assert_eq!(market.offer(order_id), None);
        assert_eq!(market.credit_balance(market.buyer.id()), buyer_credits);
    }

    /// An offer, and a sale into one, naming a token its contract does not define is refused
    /// with the token base rules' error, charged, in check_tx and in a block: a position past
    /// the contract's last token, and a contract without tokens (DPNS).
    #[tokio::test]
    async fn should_refuse_an_offer_or_sale_of_a_token_the_contract_does_not_define() {
        let mut market = tradeable_market();
        let contract_id = market.contract.id();
        let nonce = market.buyer.take_nonce();
        let place = market.place_bid_naming(contract_id, 1, nonce).await;
        assert_matches!(
            market.check_tx(&place).as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenPositionError(error))]
                if error.invalid_token_position() == 1
        );
        assert_matches!(
            market.process(&place).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::InvalidTokenPositionError(_)),
                ..
            }]
        );

        let place = market
            .place_bid_naming(SystemDataContract::DPNS.id(), 0, 1)
            .await;
        assert_matches!(
            market.check_tx(&place).as_slice(),
            [ConsensusError::BasicError(
                BasicError::ContractHasNoTokensError(_)
            )]
        );
        assert_matches!(
            market.process(&place).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::ContractHasNoTokensError(_)),
                ..
            }]
        );

        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        let platform_version = PlatformVersion::latest();
        let nonce = market.seller.take_nonce();
        let sale = BatchTransition::new_token_fill_bid_transition(
            Identifier::from(calculate_token_id(contract_id.as_bytes(), 1)),
            market.seller.id(),
            contract_id,
            1,
            order_id,
            1,
            1_000,
            1,
            &market.seller.key,
            nonce,
            0,
            &market.seller.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the sale");
        assert_matches!(
            market.check_tx(&sale).as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidTokenPositionError(_)
            )]
        );
        assert_matches!(
            market.process(&sale).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::InvalidTokenPositionError(_)),
                ..
            }]
        );
        assert_eq!(market.credit_escrow(order_id), Some(10_000));
        assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
    }

    /// An offer's escrow is what it is worth at the decimals it was signed at: 1,000 base units
    /// at 1,000,000,000 credits per whole token hold 10,000 credits at 8 decimals and
    /// 100,000,000 at 4. The buyer signs at 8; the identity controlling the token's conventions
    /// (the contract owner) changes them to 4 before the offer executes. The offer is refused
    /// with `TokenOrderDecimalsMismatchError`, by check_tx and in a block, and none of the
    /// buyer's credits move into escrow.
    #[tokio::test]
    async fn should_refuse_an_offer_signed_at_other_decimals() {
        let mut market = market_with(
            make_token_tradeable_with_owner_conventions_and_freezes,
            dash_to_credits!(1.0),
        );
        let (place, order_id) = market.place_bid(1_000, 1_000_000_000, 1, None).await;

        market.set_decimals(4).await;

        assert_matches!(
            market.check_tx(&place).as_slice(),
            [ConsensusError::StateError(StateError::TokenOrderDecimalsMismatchError(error))]
                if error.signed_decimals() == 8 && error.token_decimals() == 4
        );
        assert_matches!(
            market.process(&place).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: error @ ConsensusError::StateError(
                    StateError::TokenOrderDecimalsMismatchError(inner)
                ),
                ..
            }] if inner.token_id() == market.token_id && error.code() == 40758
        );
        assert_eq!(market.credit_escrow(order_id), None);
        assert_eq!(market.offer(order_id), None);
    }

    /// The genuine proof of an executed offer does not verify the same request re-signed naming
    /// another position of a two-token contract, or another contract, with the same token id.
    #[tokio::test]
    async fn should_not_prove_an_offer_whose_token_id_its_contract_and_position_do_not_derive() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        let mut two_tokens = market.contract.clone();
        let token = market
            .contract
            .expected_token_configuration(0)
            .expect("expected the token")
            .clone();
        two_tokens.add_token(1, token);

        for claimed in [
            with_base(&place, |base| base.set_token_contract_position(1)),
            with_base(&place, |base| {
                base.set_data_contract_id(Identifier::from([9; 32]))
            }),
        ] {
            assert_matches!(
                market.verify_proof_of_as(&place, &claimed, &two_tokens),
                Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
            );
        }
        assert_matches!(
            market.verify_proof_of_as(&place, &place, &two_tokens),
            Ok(StateTransitionProofResult::VerifiedDocuments(documents))
                if documents.get(&order_id).is_some_and(Option::is_some)
        );
    }

    /// An offer that executed, re-sent with group information and otherwise the same, is
    /// refused rather than proved by its `bid`: a placement is never a group action.
    #[tokio::test]
    async fn should_not_prove_an_offer_carrying_group_information() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);

        assert_matches!(
            market.prove_and_try_verify(&with_group_info(&place)),
            Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
        );
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedDocuments(documents)
                if documents.get(&order_id).is_some_and(Option::is_some)
        );
    }

    /// A sale into an offer moves the seller's tokens to the offer's maker. The seller, who
    /// signs, must not be frozen, whatever the token allows; the maker, who receives, may be
    /// frozen only when the token allows transfers to frozen balances. A refused sale names the
    /// frozen participant and moves nothing.
    #[tokio::test]
    async fn should_apply_each_frozen_account_rule_to_its_side_of_a_sale() {
        for allowed in [false, true] {
            // A frozen maker (the buyer).
            let mut market = market_with(
                |token_configuration: &mut TokenConfiguration| {
                    make_token_tradeable_with_owner_conventions_and_freezes(token_configuration);
                    token_configuration.allow_transfer_to_frozen_balance(allowed);
                },
                dash_to_credits!(1.0),
            );
            let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
            market.executes(&place);
            market.freeze_account(market.buyer.id()).await;
            let sale = market.fill_bid(order_id, 1, 400, 1, false).await;
            let processing_result = market.process(&sale);
            if allowed {
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution { .. }]
                );
                assert_eq!(market.token_balance(market.buyer.id()), Some(400));
                assert_eq!(market.credit_escrow(order_id), Some(6_000));
            } else {
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError {
                        error: ConsensusError::StateError(
                            StateError::IdentityTokenAccountFrozenError(error)
                        ),
                        ..
                    }] if *error.identity_id() == market.buyer.id()
                );
                assert_eq!(market.token_balance(market.buyer.id()), None);
                assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
                assert_eq!(market.credit_escrow(order_id), Some(10_000));
            }

            // A frozen seller, refused either way.
            let mut market = market_with(
                |token_configuration: &mut TokenConfiguration| {
                    make_token_tradeable_with_owner_conventions_and_freezes(token_configuration);
                    token_configuration.allow_transfer_to_frozen_balance(allowed);
                },
                dash_to_credits!(1.0),
            );
            let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
            market.executes(&place);
            market.freeze_account(market.seller.id()).await;
            let sale = market.fill_bid(order_id, 1, 400, 1, false).await;
            assert_matches!(
                market.process(&sale).execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::IdentityTokenAccountFrozenError(error)
                    ),
                    ..
                }] if *error.identity_id() == market.seller.id()
            );
            assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
            assert_eq!(market.token_balance(market.buyer.id()), None);
            assert_eq!(market.credit_escrow(order_id), Some(10_000));
        }
    }

    /// A frozen buyer can still cancel its offer and get back what it holds in escrow.
    #[tokio::test]
    async fn should_let_a_frozen_buyer_cancel_its_offer() {
        let mut market = market_with(
            make_token_tradeable_with_owner_conventions_and_freezes,
            dash_to_credits!(1.0),
        );
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        market.freeze_account(market.buyer.id()).await;
        let buyer_credits = market.credit_balance(market.buyer.id());

        let cancel = market.cancel_bid(order_id, false).await;
        let fee = market.executes(&cancel);
        assert_eq!(market.credit_escrow(order_id), None);
        assert_eq!(market.offer(order_id), None);
        assert_eq!(
            change(buyer_credits, market.credit_balance(market.buyer.id())),
            change_for(&fee, market.buyer.id(), 10_000)
        );
    }

    #[tokio::test]
    async fn should_refuse_selling_into_ones_own_offer() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);

        let sale = market.fill_bid(order_id, 1, 1_000, 1, true).await;
        assert_matches!(
            market.process(&sale).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenOrderSelfFillError(_)),
                ..
            }]
        );
        assert_eq!(market.credit_escrow(order_id), Some(10_000));
    }

    #[tokio::test]
    async fn should_refuse_a_sale_paid_below_the_sellers_minimum() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);

        let sale = market.fill_bid(order_id, 1, 400, 4_001, false).await;
        assert_matches!(
            market.process(&sale).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenOrderPriceLimitError(inner)),
                ..
            }] if inner.total() == 4_000 && inner.limit() == 4_001
        );
        assert_eq!(market.credit_escrow(order_id), Some(10_000));
    }

    #[tokio::test]
    async fn should_refuse_selling_more_than_the_seller_holds() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(BASE_SUPPLY + 1, PRICE, 1, None).await;
        market.executes(&place);

        let sale = market
            .fill_bid(order_id, BASE_SUPPLY + 1, BASE_SUPPLY + 1, 1, false)
            .await;
        assert_matches!(
            market.process(&sale).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
        );
        assert_eq!(market.token_balance(market.seller.id()), Some(BASE_SUPPLY));
    }

    #[tokio::test]
    async fn should_refuse_selling_into_an_expired_offer_but_still_cancel_it() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, Some(5_000)).await;
        market.assert_executes(&place, &at_time(1_000));

        let sale = market.fill_bid(order_id, 1, 1_000, 1, false).await;
        assert_matches!(
            market
                .process_at(&sale, &at_time(5_000))
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenOrderExpiredError(_)),
                ..
            }]
        );

        let cancel = market.cancel_bid(order_id, false).await;
        market.assert_executes(&cancel, &at_time(5_000));
        assert_eq!(market.credit_escrow(order_id), None);
    }

    #[tokio::test]
    async fn should_refuse_cancelling_someone_elses_offer() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);

        let cancel = market.cancel_bid(order_id, true).await;
        assert_matches!(
            market.process(&cancel).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenOrderNotOwnedError(_)),
                ..
            }]
        );
        assert_eq!(market.credit_escrow(order_id), Some(10_000));
    }

    #[tokio::test]
    async fn should_refuse_a_sale_of_a_paused_token_but_still_cancel() {
        let mut market = market_with(make_token_tradeable_and_pausable, dash_to_credits!(1.0));
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.executes(&place);
        market.pause().await;

        let sale = market.fill_bid(order_id, 1, 1_000, 1, false).await;
        assert_matches!(
            market.process(&sale).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenIsPausedError(_)),
                ..
            }]
        );

        let cancel = market.cancel_bid(order_id, false).await;
        market.executes(&cancel);
        assert_eq!(market.credit_escrow(order_id), None);
    }
}

/// The marketplace's events, recorded in the token history contract (v3) for a token that keeps
/// transfer history, as its transfers are.
mod history {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::{
        TokenKeepsHistoryRulesV0Getters, TokenKeepsHistoryRulesV0Setters,
    };
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;
    use drive::query::DriveDocumentQuery;

    impl Market {
        /// Every document of `document_type_name` in the token history contract.
        fn history(&self, document_type_name: &str) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let token_history_contract = self
                .platform
                .drive
                .cache
                .system_data_contracts
                .load_token_history(platform_version)
                .expect("expected the token history contract");
            let document_type = token_history_contract
                .document_type_for_name(document_type_name)
                .expect("expected the token history contract v3 to have the type");
            let query =
                DriveDocumentQuery::all_items_query(&token_history_contract, document_type, None);
            self.platform
                .drive
                .query_documents(
                    query,
                    None,
                    false,
                    None,
                    Some(platform_version.protocol_version),
                )
                .expect("expected to query the token history")
                .documents_owned()
        }
    }

    fn property(document: &Document, name: &str) -> Value {
        document
            .properties()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("expected the history document to have {name}"))
    }

    #[tokio::test]
    async fn should_record_a_listing_its_fill_and_its_cancel() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        let fill = market.fill_ask(order_id, 1, 400, 4_000).await;
        market.assert_executes(&fill, &BlockInfo::default());
        let cancel = market.cancel(order_id, false).await;
        market.assert_executes(&cancel, &BlockInfo::default());

        let placed = market.history("orderPlaced");
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].owner_id(), market.seller.id());
        assert_eq!(
            property(&placed[0], "tokenId"),
            Value::Identifier(market.token_id.to_buffer())
        );
        assert_eq!(
            property(&placed[0], "orderId"),
            Value::Identifier(order_id.to_buffer())
        );
        assert_eq!(
            property(&placed[0], "side").to_integer::<u8>().ok(),
            Some(0)
        );
        assert_eq!(
            property(&placed[0], "amount").to_integer::<u64>().ok(),
            Some(1_000)
        );
        assert_eq!(
            property(&placed[0], "price").to_integer::<u64>().ok(),
            Some(PRICE)
        );
        assert_eq!(
            property(&placed[0], "minFill").to_integer::<u64>().ok(),
            Some(1)
        );
        assert_eq!(
            property(&placed[0], "decimals").to_integer::<u8>().ok(),
            Some(8)
        );
        assert_eq!(placed[0].properties().get("expiresAt"), None);

        let filled = market.history("orderFilled");
        assert_eq!(filled.len(), 1);
        assert_eq!(filled[0].owner_id(), market.buyer.id());
        assert_eq!(
            property(&filled[0], "makerId"),
            Value::Identifier(market.seller.id().to_buffer())
        );
        assert_eq!(
            property(&filled[0], "amount").to_integer::<u64>().ok(),
            Some(400)
        );
        assert_eq!(
            property(&filled[0], "credits").to_integer::<u64>().ok(),
            Some(4_000)
        );
        assert_eq!(
            property(&filled[0], "decimals").to_integer::<u8>().ok(),
            Some(8)
        );

        let cancelled = market.history("orderCancelled");
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].owner_id(), market.seller.id());
        assert_eq!(
            property(&cancelled[0], "orderId"),
            Value::Identifier(order_id.to_buffer())
        );
    }

    #[tokio::test]
    async fn should_record_an_offer_and_a_sale_into_it_on_the_bid_side() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        let sale = market.fill_bid(order_id, 1, 1_000, 1, false).await;
        market.assert_executes(&sale, &BlockInfo::default());

        let placed = market.history("orderPlaced");
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].owner_id(), market.buyer.id());
        assert_eq!(
            property(&placed[0], "side").to_integer::<u8>().ok(),
            Some(1)
        );

        let filled = market.history("orderFilled");
        assert_eq!(filled.len(), 1);
        assert_eq!(filled[0].owner_id(), market.seller.id());
        assert_eq!(
            property(&filled[0], "makerId"),
            Value::Identifier(market.buyer.id().to_buffer())
        );
        assert_eq!(
            property(&filled[0], "credits").to_integer::<u64>().ok(),
            Some(10_000)
        );
    }

    /// A cancelled offer is recorded on the bid side, owned by the buyer who cancelled it; the
    /// cancel releases the offer's credit escrow rather than tokens.
    #[tokio::test]
    async fn should_record_the_cancel_of_an_offer_on_the_bid_side() {
        let mut market = tradeable_market();
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        let cancel = market.cancel_bid(order_id, false).await;
        market.assert_executes(&cancel, &BlockInfo::default());

        let cancelled = market.history("orderCancelled");
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].owner_id(), market.buyer.id());
        assert_eq!(
            property(&cancelled[0], "tokenId"),
            Value::Identifier(market.token_id.to_buffer())
        );
        assert_eq!(
            property(&cancelled[0], "orderId"),
            Value::Identifier(order_id.to_buffer())
        );
        assert_eq!(
            property(&cancelled[0], "side").to_integer::<u8>().ok(),
            Some(1)
        );
    }

    /// A closed order's placement is proved by its `orderPlaced` record, which holds every term
    /// of the placement. A placement with the same signer, nonce, amount and price but another
    /// minimum fill, expiry or signed decimals has the same order id and record id, and does
    /// not verify against that proof, on either side.
    #[tokio::test]
    async fn should_not_prove_a_closed_order_for_other_terms() {
        let mut market = tradeable_market();
        let other_terms = [(11, Some(9_000)), (10, Some(9_001)), (10, None)];

        let nonce = market.seller.nonce;
        let (place, order_id) = market.place_ask(1_000, PRICE, 10, Some(9_000)).await;
        market.assert_executes(&place, &BlockInfo::default());
        let fill = market.fill_ask(order_id, 1, 1_000, 10_000).await;
        market.assert_executes(&fill, &BlockInfo::default());
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedTokenActionWithDocument(_)
        );
        for (min_fill, expires_at) in other_terms {
            let next = market.seller.nonce;
            market.seller.nonce = nonce;
            let (other, other_id) = market.place_ask(1_000, PRICE, min_fill, expires_at).await;
            market.seller.nonce = next;
            assert_eq!(other_id, order_id);
            assert!(
                matches!(
                    market.prove_and_try_verify(&other),
                    Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
                ),
                "a listing with a minimum fill of {min_fill} and expiry {expires_at:?}"
            );
        }

        // The same listing signed at other decimals.
        let platform_version = PlatformVersion::latest();
        let other_decimals = BatchTransition::new_token_place_ask_transition(
            market.token_id,
            market.seller.id(),
            market.contract.id(),
            0,
            1_000,
            PRICE,
            2,
            10,
            Some(9_000),
            &market.seller.key,
            nonce,
            0,
            &market.seller.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the listing");
        assert!(matches!(
            market.prove_and_try_verify(&other_decimals),
            Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
        ));

        let nonce = market.buyer.nonce;
        let (place, order_id) = market.place_bid(1_000, PRICE, 10, Some(9_000)).await;
        market.assert_executes(&place, &BlockInfo::default());
        let sale = market.fill_bid(order_id, 1, 1_000, 1, false).await;
        market.assert_executes(&sale, &BlockInfo::default());
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedTokenActionWithDocument(_)
        );
        for (min_fill, expires_at) in other_terms {
            let next = market.buyer.nonce;
            market.buyer.nonce = nonce;
            let (other, other_id) = market.place_bid(1_000, PRICE, min_fill, expires_at).await;
            market.buyer.nonce = next;
            assert_eq!(other_id, order_id);
            assert!(
                matches!(
                    market.prove_and_try_verify(&other),
                    Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
                ),
                "an offer with a minimum fill of {min_fill} and expiry {expires_at:?}"
            );
        }
    }

    /// A placement is never a group action. A placement carrying group action information,
    /// with every other term of one that executed, is refused rather than proved, while its
    /// order is open and once it has closed (where a group action's evidence could otherwise
    /// stand in for the missing placement record), on either side.
    #[tokio::test]
    async fn should_not_prove_a_placement_carrying_group_information() {
        let mut market = tradeable_market();
        let refused = |market: &Market, transition: &StateTransition| {
            assert_matches!(
                market.prove_and_try_verify(&with_group_info(transition)),
                Err(DriveError::Proof(ProofError::UnexpectedResultProof(_)))
            );
        };

        let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        refused(&market, &place);
        let fill = market.fill_ask(order_id, 1, 1_000, 10_000).await;
        market.assert_executes(&fill, &BlockInfo::default());
        refused(&market, &place);
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedTokenActionWithDocument(_)
        );

        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        refused(&market, &place);
        let sale = market.fill_bid(order_id, 1, 1_000, 1, false).await;
        market.assert_executes(&sale, &BlockInfo::default());
        refused(&market, &place);
    }

    /// The history keeps an order's price scale: a listing placed at 8 decimals and filled after
    /// the token changes to 2 records 8 on its placement and its fill, and its placement is
    /// still proved once it has closed, against the contract at either scale.
    #[tokio::test]
    async fn should_record_the_decimals_an_order_was_placed_at() {
        let mut market = market_with(
            |token_configuration: &mut TokenConfiguration| {
                make_token_tradeable(token_configuration);
                token_configuration.set_conventions_change_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            },
            dash_to_credits!(1.0),
        );
        let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        market.set_decimals(2).await;
        let fill = market.fill_ask(order_id, 1, 1_000, 10_000).await;
        market.assert_executes(&fill, &BlockInfo::default());

        let placed = market.history("orderPlaced");
        assert_eq!(placed.len(), 1);
        assert_eq!(
            property(&placed[0], "decimals").to_integer::<u8>().ok(),
            Some(8)
        );
        let filled = market.history("orderFilled");
        assert_eq!(filled.len(), 1);
        assert_eq!(
            property(&filled[0], "decimals").to_integer::<u8>().ok(),
            Some(8)
        );
        assert_eq!(
            property(&filled[0], "credits").to_integer::<u64>().ok(),
            Some(10_000)
        );
        // A client still holding the contract at 8 decimals, and one that fetched it again at 2,
        // both confirm the closed placement.
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedTokenActionWithDocument(_)
        );
        market.refresh_contract();
        assert_eq!(
            market
                .contract
                .expected_token_configuration(0)
                .expect("expected the token")
                .conventions()
                .decimals(),
            2
        );
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedTokenActionWithDocument(_)
        );
    }

    /// A closed order of a token that keeps no transfer history leaves nothing that proves its
    /// placement: the verification refuses rather than take the order's absence for it. An
    /// order still open is proved by its document, history or not.
    #[tokio::test]
    async fn should_not_prove_a_closed_order_of_a_token_without_history() {
        let mut market = market_with(
            |token_configuration: &mut TokenConfiguration| {
                make_token_tradeable(token_configuration);
                token_configuration
                    .keeps_history_mut()
                    .set_keeps_transfer_history(false);
            },
            dash_to_credits!(1.0),
        );
        let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        assert_matches!(
            market.prove_and_verify(&place),
            StateTransitionProofResult::VerifiedDocuments(documents)
                if documents.get(&order_id).is_some_and(Option::is_some)
        );
        let fill = market.fill_ask(order_id, 1, 1_000, 10_000).await;
        market.assert_executes(&fill, &BlockInfo::default());
        assert_matches!(
            market.prove_and_try_verify(&place),
            Err(DriveError::Proof(ProofError::IncorrectProof(_)))
        );
    }

    #[tokio::test]
    async fn should_record_nothing_for_a_token_that_keeps_no_transfer_history() {
        let mut market = market_with(
            |token_configuration: &mut TokenConfiguration| {
                make_token_tradeable(token_configuration);
                token_configuration
                    .keeps_history_mut()
                    .set_keeps_transfer_history(false);
            },
            dash_to_credits!(1.0),
        );
        assert!(!market
            .contract
            .expected_token_configuration(0)
            .expect("expected the token")
            .keeps_history()
            .keeps_transfer_history());
        let (place, order_id) = market.place_ask(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        let fill = market.fill_ask(order_id, 1, 400, 4_000).await;
        market.assert_executes(&fill, &BlockInfo::default());
        let cancel = market.cancel(order_id, false).await;
        market.assert_executes(&cancel, &BlockInfo::default());

        // The bid side: an offer, a sale into it and the cancel of another.
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        let sale = market.fill_bid(order_id, 1, 1_000, 1, false).await;
        market.assert_executes(&sale, &BlockInfo::default());
        let (place, order_id) = market.place_bid(1_000, PRICE, 1, None).await;
        market.assert_executes(&place, &BlockInfo::default());
        let cancel = market.cancel_bid(order_id, false).await;
        market.assert_executes(&cancel, &BlockInfo::default());

        assert!(market.history("orderPlaced").is_empty());
        assert!(market.history("orderFilled").is_empty());
        assert!(market.history("orderCancelled").is_empty());
    }
}
