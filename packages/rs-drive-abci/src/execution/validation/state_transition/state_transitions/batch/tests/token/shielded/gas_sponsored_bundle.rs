//! The shielded compute fee of a batch whose gas the contract owner pays.
//!
//! A batch that insists on the contract owner paying its gas funds only its principal from the
//! signer's balance, so the signer is never held to the compute fee of the Orchard bundles the
//! batch carries. That leaves the sponsor as the only one who can be charged for the
//! verification, and the sponsor is read while the batch is transformed, before any
//! sub-transition is validated. A sponsor who cannot cover the compute fee is refused there, so
//! the bundles are never verified for a batch nobody can be charged for. A sponsor who merely
//! prefers to pay is not asked, because the signer pays in their place.

use super::document_shielded_token_payment_tests as cards;
use super::token_shielded_pool_tests::{
    assert_tokens_conserved, build_spend_bundle, insert_token_pool_anchor, nullifier_is_spent,
    platform_with_latest_version, pool_balance, process, spendable_note,
};
use super::*;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
use dpp::data_contract::document_type::accessors::{
    DocumentTypeV0MutGetters, DocumentTypeV1Setters,
};
use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::identity::Identity;
use dpp::prelude::IdentityNonce;
use dpp::shielded::{
    compute_shielded_verification_fee, document_token_payment_extra_sighash_data_v0,
};
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_amount_on_contract_token::{
    DocumentActionTokenCost, DocumentActionTokenEffect,
};
use dpp::tokens::token_payment_info::v1::{TokenPaymentInfoV1, TokenShieldedPayment};
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use drive::util::test_helpers::setup_contract;
use platform_version::version::PlatformVersion;

/// The Orchard actions of the spend bundle the setup builds: one spend of a pool note and its
/// change output, padded to the bundle type's minimum. Every test asserts it against the bundle
/// that was actually built, so a change in the builder cannot silently reprice the compute fee
/// a test sets a balance from.
const BUNDLE_ACTIONS: usize = 2;

/// The compute fee such a bundle will be charged.
fn bundle_compute_fee() -> Credits {
    compute_shielded_verification_fee(BUNDLE_ACTIONS, PlatformVersion::latest())
        .expect("shielded compute fee")
}

/// The card game contract with a burned creation cost in its own pooled token 0, whose card
/// creation offers the contract owner for the gas. That offer accepts both an insistence and a
/// preference, so it is the one offer every test here needs.
fn sponsoring_card_game_contract(
    platform: &TempPlatform<MockCoreRPCLike>,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> (DataContract, Identifier) {
    let offered = GasFeesPaidBy::ContractOwner;
    let data_contract_id = DataContract::generate_data_contract_id_v0(owner_id, 1);
    let contract = setup_contract(
        &platform.drive,
        "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json",
        Some(data_contract_id.to_buffer()),
        Some(owner_id.to_buffer()),
        Some(|data_contract: &mut DataContract| {
            data_contract.set_created_at_epoch(Some(0));
            data_contract.set_created_at(Some(0));
            data_contract.set_created_at_block_height(Some(0));
            let configuration = data_contract
                .tokens_mut()
                .and_then(|tokens| tokens.get_mut(&0))
                .expect("token 0");
            configuration.set_has_shielded_pool(true);
            let document_type = data_contract
                .document_types_mut()
                .get_mut("card")
                .expect("card document type");
            document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                contract_id: None,
                token_contract_position: 0,
                token_amount: cards::CARD_COST,
                effect: DocumentActionTokenEffect::BurnToken,
                gas_fees_paid_by: offered,
                optional: false,
            }));
            // The validator reads the contract back from Drive, where the document type is
            // rebuilt from the stored schema.
            let offered_int: u8 = offered.into();
            document_type
                .schema_mut()
                .get_mut("tokenCost")
                .expect("token cost")
                .expect("the card game charges for a creation")
                .get_mut("create")
                .expect("creation token cost")
                .expect("the card game charges for a creation")
                .set_value("gasFeesPaidBy", offered_int.into())
                .expect("who pays the gas of a creation");
        }),
        None,
        Some(platform_version),
    );
    (
        contract,
        calculate_token_id(data_contract_id.as_bytes(), 0).into(),
    )
}

/// The card's token cost paid out of the pool by `payment`, asking `requested` for the gas.
fn sponsored_payment_info(
    payment: TokenShieldedPayment,
    requested: GasFeesPaidBy,
) -> TokenPaymentInfo {
    TokenPaymentInfo::V1(TokenPaymentInfoV1 {
        payment_token_contract_id: None,
        token_contract_position: 0,
        minimum_token_cost: None,
        maximum_token_cost: Some(cards::CARD_COST),
        gas_fees_paid_by: requested,
        shielded_payment: Box::new(payment),
    })
}

/// A sponsored card creation whose bundle cannot verify, on its own chain.
struct UnverifiableSponsoredCard {
    platform: TempPlatform<MockCoreRPCLike>,
    contract_owner: Identity,
    buyer_id: Identifier,
    contract_id: Identifier,
    token_id: Identifier,
    transition: StateTransition,
    payment: TokenShieldedPayment,
}

impl UnverifiableSponsoredCard {
    /// The buyer's nonce on the card contract, as stored: the counter with the recent-nonce
    /// bitmap above it.
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.platform
            .drive
            .fetch_identity_contract_nonce(
                self.buyer_id.to_buffer(),
                self.contract_id.to_buffer(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("identity contract nonce")
            .expect("the shield set the buyer's nonce on this contract")
    }
}

/// A chain with a funded pool and a card creation paying its cost from it, asking `requested`
/// for the gas of a contract owner holding `sponsor_balance`. The buyer is funded well enough to
/// pay for the whole batch themselves, so nothing here turns on the signer's balance.
///
/// Every check before the Halo 2 verification passes — the anchor is recorded, the nullifiers are
/// unspent, the pool holds the cost, and the payment's amount equals it — and only the proof
/// fails, because its sighash is bound to another id in the document's place. A refusal naming
/// anything but the proof therefore happened before the verification ran.
async fn unverifiable_sponsored_card(
    sponsor_balance: Credits,
    requested: GasFeesPaidBy,
    seed: u64,
) -> UnverifiableSponsoredCard {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(seed);

    let (contract_owner, _, _) = setup_identity(&mut platform, 971, sponsor_balance);
    let (buyer, signer, key) = setup_identity(&mut platform, 247, dash_to_credits!(0.5));
    let (contract, token_id) =
        sponsoring_card_game_contract(&platform, contract_owner.id(), platform_version);
    cards::fund_and_shield(
        &mut platform,
        &contract,
        token_id,
        &buyer,
        &key,
        &signer,
        seed,
        platform_version,
    )
    .await;

    let card_document_type = contract
        .document_type_for_name("card")
        .expect("card document type");
    let (document, entropy) = cards::random_card(
        &mut rng,
        card_document_type,
        buyer.id(),
        2,
        platform_version,
    );

    let (note, anchor, merkle_path) = spendable_note(cards::SHIELDED, 9);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    // The buyer's id stands where the document's id belongs, so the bundle is proven for a
    // sighash no validator will compute for it.
    let extra = document_token_payment_extra_sighash_data_v0(
        &token_id.to_buffer(),
        &buyer.id().to_buffer(),
        &contract.id().to_buffer(),
        &buyer.id().to_buffer(),
        cards::CARD_COST,
    );
    let (bundle, _) = build_spend_bundle(
        note,
        merkle_path,
        anchor,
        cards::CARD_COST,
        &extra,
        seed + 1,
    );
    let payment = cards::shielded_payment(bundle, cards::CARD_COST);
    assert_eq!(
        payment.actions.len(),
        BUNDLE_ACTIONS,
        "the sponsor's balance is set from this many actions"
    );

    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        card_document_type,
        entropy.0,
        &key,
        2,
        0,
        Some(sponsored_payment_info(payment.clone(), requested)),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("document create transition");

    UnverifiableSponsoredCard {
        platform,
        contract_owner,
        buyer_id: buyer.id(),
        contract_id: contract.id(),
        token_id,
        transition,
        payment,
    }
}

/// A sponsor one credit short of the compute fee is refused while the batch is transformed, so
/// the bundle is never verified: a batch nobody can be charged for cannot buy proof work. The
/// refusal names the sponsor's shortfall rather than the proof, which is how the test tells the
/// two orders apart.
#[tokio::test]
async fn should_refuse_a_sponsor_who_cannot_cover_the_compute_fee_before_verifying_the_bundle() {
    let compute_fee = bundle_compute_fee();
    let sponsor_balance = compute_fee - 1;
    let card =
        unverifiable_sponsored_card(sponsor_balance, GasFeesPaidBy::ContractOwner, 9701).await;

    let result = process(&card.platform, &card.transition);
    // A batch insisting on a sponsor who cannot pay charges nobody, so the refusal is unpaid,
    // exactly as it is when fee validation finds the same shortfall after the verification.
    let [StateTransitionExecutionResult::UnpaidConsensusError(error)] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected one unpaid refusal, got {:?}",
            result.execution_results()
        );
    };
    // The required balance is the compute fee alone, which is what separates this refusal from
    // the one fee validation raises after the verification: that one asks for the whole gas.
    assert_matches!(
        error,
        ConsensusError::StateError(StateError::GasSponsorInsufficientBalanceError(error))
            if error.sponsor_id() == &card.contract_owner.id()
                && error.balance() == sponsor_balance
                && error.required_balance() == compute_fee,
        "the sponsor's shortfall, not the proof: the bundle must not be verified for a batch \
         nobody can be charged for"
    );

    // Nothing left the pool and the notes are still unspent.
    assert_eq!(pool_balance(&card.platform, card.token_id), cards::SHIELDED);
    assert!(!nullifier_is_spent(
        &card.platform,
        card.token_id,
        &card.payment.actions[0].nullifier
    ));
    assert_tokens_conserved(&card.platform);
    // Nothing was charged, so the nonce the batch used stays available: the shield that funded
    // the pool used the first one, and nothing has moved it since.
    assert_eq!(
        card.identity_contract_nonce() & IDENTITY_NONCE_VALUE_FILTER,
        1
    );
}

/// The same batch with a sponsor who can pay still has its bundle verified: the compute fee
/// refusal must not stand in for the proof check.
#[tokio::test]
async fn should_verify_the_bundle_of_a_sponsor_who_can_cover_the_compute_fee() {
    let card =
        unverifiable_sponsored_card(dash_to_credits!(0.1), GasFeesPaidBy::ContractOwner, 9703)
            .await;

    let result = process(&card.platform, &card.transition);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
            ..
        }],
        "a funded sponsor's batch reaches the verification, which the bundle fails"
    );

    assert_eq!(pool_balance(&card.platform, card.token_id), cards::SHIELDED);
    assert_tokens_conserved(&card.platform);
}

/// A batch that only prefers the contract owner keeps its fallback: the signer pays when the
/// contract owner's balance falls short, so the compute fee is not held against a sponsor who
/// never promised to cover it, and the bundle is verified as it would be for an unsponsored
/// batch.
#[tokio::test]
async fn should_verify_the_bundle_of_a_preferred_sponsor_who_cannot_cover_the_compute_fee() {
    let sponsor_balance = bundle_compute_fee() - 1;
    let card =
        unverifiable_sponsored_card(sponsor_balance, GasFeesPaidBy::PreferContractOwner, 9705)
            .await;

    let result = process(&card.platform, &card.transition);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
            ..
        }],
        "a preference is not an insistence: the signer pays, so the batch reaches the verification"
    );

    assert_eq!(pool_balance(&card.platform, card.token_id), cards::SHIELDED);
    assert_tokens_conserved(&card.platform);
}
