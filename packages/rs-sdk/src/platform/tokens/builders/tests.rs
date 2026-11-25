//! A signer who joins someone else's group action must not repeat the proposal's note: every
//! group-capable builder leaves it off, so the signed transition passes structure validation.

use super::burn::TokenBurnTransitionBuilder;
use super::config_update::TokenConfigUpdateTransitionBuilder;
use super::destroy::TokenDestroyFrozenFundsTransitionBuilder;
use super::emergency_action::TokenEmergencyActionTransitionBuilder;
use super::freeze::TokenFreezeTransitionBuilder;
use super::mint::TokenMintTransitionBuilder;
use super::public_note_for_signer;
use super::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;
use super::unfreeze::TokenUnfreezeTransitionBuilder;
use crate::sdk::put_test_support::CountingSigner;
use crate::sdk::SdkBuilder;
use crate::Sdk;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::DataContract;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use drive_proof_verifier::types::IdentityContractNonceFetcher;
use std::sync::Arc;

const NOTE: &str = "proposer's note";

fn signer_id() -> Identifier {
    Identifier::new([3; 32])
}

fn target_id() -> Identifier {
    Identifier::new([4; 32])
}

/// A non-default position, so a builder that reset the group info would be caught.
fn co_sign_info() -> GroupStateTransitionInfo {
    GroupStateTransitionInfo {
        group_contract_position: 2,
        action_id: Identifier::new([7; 32]),
        action_is_proposer: false,
    }
}

fn other_signer() -> GroupStateTransitionInfoStatus {
    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(co_sign_info())
}

fn contract() -> Arc<DataContract> {
    Arc::new(
        load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("contract"),
    )
}

struct Fixture {
    sdk: Sdk,
    contract: Arc<DataContract>,
    key: IdentityPublicKey,
    signer: CountingSigner,
}

impl Fixture {
    async fn new() -> Self {
        let version = PlatformVersion::latest();
        let mut sdk = SdkBuilder::new_mock()
            .with_version(version)
            .build()
            .expect("mock sdk");
        let contract = contract();
        sdk.mock()
            .expect_fetch(
                (signer_id(), contract.id()),
                Some(IdentityContractNonceFetcher(0)),
            )
            .await
            .expect("nonce expectation");
        let key =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(1, Some(1), version)
                .expect("key")
                .0;
        Self {
            sdk,
            contract,
            key,
            signer: CountingSigner::default(),
        }
    }
}

fn batch(state_transition: &StateTransition) -> &BatchTransition {
    match state_transition {
        StateTransition::Batch(batch) => batch,
        other => panic!("expected a batch transition, got {other:?}"),
    }
}

/// The signature still counts toward the proposal: same position, action and signer role.
fn assert_targets_the_proposal(state_transition: &StateTransition) {
    let BatchTransition::V1(v1) = batch(state_transition) else {
        panic!("token transitions are built as batch v1");
    };
    let [BatchedTransition::Token(token)] = v1.transitions.as_slice() else {
        panic!("expected a single token transition");
    };
    assert_eq!(token.base().using_group_info(), Some(co_sign_info()));
}

fn assert_passes_structure_validation(state_transition: &StateTransition) {
    let result = batch(state_transition)
        .validate_base_structure(PlatformVersion::latest())
        .expect("structure validation runs");
    assert!(
        !result.errors.iter().any(|error| matches!(
            error,
            ConsensusError::BasicError(BasicError::TokenNoteOnlyAllowedWhenProposerError(_))
        )),
        "a co-sign must not carry the proposal's note: {:?}",
        result.errors
    );
    assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
}

#[tokio::test]
async fn should_leave_the_proposal_note_off_a_mint_co_sign() {
    let f = Fixture::new().await;
    let st = TokenMintTransitionBuilder::new(f.contract.clone(), 0, signer_id(), 777)
        .with_public_note(NOTE.to_string())
        .with_using_group_info(other_signer())
        .sign(&f.sdk, &f.key, &f.signer, PlatformVersion::latest())
        .await
        .expect("sign");

    let BatchTransition::V1(v1) = batch(&st) else {
        panic!("token transitions are built as batch v1");
    };
    let Some(BatchedTransition::Token(TokenTransition::Mint(mint))) = v1.transitions.first() else {
        panic!("expected a single mint transition");
    };
    assert_eq!(mint.public_note(), None);
    assert_targets_the_proposal(&st);
    assert_passes_structure_validation(&st);
}

#[tokio::test]
async fn should_leave_the_proposal_note_off_every_group_co_sign() {
    let version = PlatformVersion::latest();
    let c = contract();
    let note = || NOTE.to_string();
    let cases = vec![
        (
            "burn",
            CoSign::Burn(
                TokenBurnTransitionBuilder::new(c.clone(), 0, signer_id(), 5)
                    .with_public_note(note())
                    .with_using_group_info(other_signer()),
            ),
        ),
        (
            "freeze",
            CoSign::Freeze(
                TokenFreezeTransitionBuilder::new(c.clone(), 0, signer_id(), target_id())
                    .with_public_note(note())
                    .with_using_group_info(other_signer()),
            ),
        ),
        (
            "unfreeze",
            CoSign::Unfreeze(
                TokenUnfreezeTransitionBuilder::new(c.clone(), 0, signer_id(), target_id())
                    .with_public_note(note())
                    .with_using_group_info(other_signer()),
            ),
        ),
        (
            "destroy frozen funds",
            CoSign::Destroy(
                TokenDestroyFrozenFundsTransitionBuilder::new(
                    c.clone(),
                    0,
                    signer_id(),
                    target_id(),
                )
                .with_public_note(note())
                .with_using_group_info(other_signer()),
            ),
        ),
        (
            "pause",
            CoSign::Emergency(
                TokenEmergencyActionTransitionBuilder::pause(c.clone(), 0, signer_id())
                    .with_public_note(note())
                    .with_using_group_info(other_signer()),
            ),
        ),
        (
            "set price",
            CoSign::SetPrice(
                TokenChangeDirectPurchasePriceTransitionBuilder::new(c.clone(), 0, signer_id())
                    .with_single_price(10)
                    .with_public_note(note())
                    .with_using_group_info(other_signer()),
            ),
        ),
        (
            "config update",
            CoSign::ConfigUpdate(
                TokenConfigUpdateTransitionBuilder::new(
                    c.clone(),
                    0,
                    signer_id(),
                    TokenConfigurationChangeItem::MaxSupply(Some(1_000)),
                )
                .with_public_note(note())
                .with_using_group_info(other_signer()),
            ),
        ),
    ];

    for (name, co_sign) in cases {
        // A fresh SDK per case: each signature fetches the signer's contract nonce once.
        let f = Fixture::new().await;
        let st = co_sign
            .sign(&f.sdk, &f.key, &f.signer, version)
            .await
            .unwrap_or_else(|e| panic!("{name}: sign failed: {e}"));
        assert_targets_the_proposal(&st);
        let result = batch(&st)
            .validate_base_structure(version)
            .expect("structure validation runs");
        assert!(result.is_valid(), "{name}: {:?}", result.errors);
    }
}

enum CoSign {
    Burn(TokenBurnTransitionBuilder),
    Freeze(TokenFreezeTransitionBuilder),
    Unfreeze(TokenUnfreezeTransitionBuilder),
    Destroy(TokenDestroyFrozenFundsTransitionBuilder),
    Emergency(TokenEmergencyActionTransitionBuilder),
    SetPrice(TokenChangeDirectPurchasePriceTransitionBuilder),
    ConfigUpdate(TokenConfigUpdateTransitionBuilder),
}

impl CoSign {
    async fn sign(
        self,
        sdk: &Sdk,
        key: &IdentityPublicKey,
        signer: &CountingSigner,
        version: &PlatformVersion,
    ) -> Result<StateTransition, crate::Error> {
        match self {
            CoSign::Burn(b) => b.sign(sdk, key, signer, version).await,
            CoSign::Freeze(b) => b.sign(sdk, key, signer, version).await,
            CoSign::Unfreeze(b) => b.sign(sdk, key, signer, version).await,
            CoSign::Destroy(b) => b.sign(sdk, key, signer, version).await,
            CoSign::Emergency(b) => b.sign(sdk, key, signer, version).await,
            CoSign::SetPrice(b) => b.sign(sdk, key, signer, version).await,
            CoSign::ConfigUpdate(b) => b.sign(sdk, key, signer, version).await,
        }
    }
}

#[test]
fn should_keep_the_note_for_a_proposer_or_an_ungrouped_action() {
    let note = Some(NOTE.to_string());
    assert_eq!(public_note_for_signer(note.clone(), None), note);
    assert_eq!(
        public_note_for_signer(
            note.clone(),
            Some(&GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
        ),
        note
    );
    // A proposer may also state its action explicitly; the note stays its own.
    assert_eq!(
        public_note_for_signer(
            note.clone(),
            Some(
                &GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id: Identifier::new([7; 32]),
                        action_is_proposer: true,
                    },
                ),
            ),
        ),
        note
    );
    assert_eq!(public_note_for_signer(note, Some(&other_signer())), None);
}
