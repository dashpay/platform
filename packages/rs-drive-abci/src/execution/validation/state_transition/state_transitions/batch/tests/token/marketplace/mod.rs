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
/// alone, and the document holds its terms with less open. A listing filled entirely is gone,
/// so its document no longer proves it.
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
        market.prove_and_try_verify(&place),
        Err(DriveError::Proof(ProofError::IncorrectProof(_)))
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
