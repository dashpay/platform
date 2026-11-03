//! The proof of a random token transfer or mint (protocol version 15): its sender cannot know
//! the amount the block's random beacon drew, so verification takes it from the proved history
//! document once it lies in the range the transition asked for, and compares every other field
//! as for any transition. Each test verifies one genuine proof, built from a drive that executed
//! and settled the transition, against expectations that differ from the transition sent.

use super::*;
use crate::drive::tokens::settle_random_token_amounts::{
    RandomTokenRequest, RandomTokenRequestKind,
};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use dpp::data_contract::config::v0::DataContractConfigV0;
use dpp::data_contract::v1::DataContractV1;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV1;
use dpp::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV1;
use dpp::state_transition::batch_transition::{
    BatchTransition, BatchTransitionV1, TokenMintTransition, TokenTransferTransition,
};
use dpp::tokens::token_event::TokenEvent;
use std::sync::Arc;

const CONTRACT: [u8; 32] = [3; 32];
const OWNER: [u8; 32] = [1; 32];
const RECIPIENT: [u8; 32] = [2; 32];
const OTHER_RECIPIENT: [u8; 32] = [5; 32];
const CONTRACT_OWNER: [u8; 32] = [4; 32];
const NONCE: u64 = 1;
const CREDITS: u64 = 1_000_000;
const BASE_SUPPLY: TokenAmount = 10_000;
const MINIMUM: TokenAmount = 100;
const MAXIMUM: TokenAmount = 1_000;
const BEACON: [u8; 96] = [7; 96];
const NOTE: &str = "for the raffle";

/// Which random transition the fixture executed.
#[derive(Clone, Copy)]
enum Kind {
    Transfer,
    Mint,
}

struct Fixture {
    drive: Drive,
    contract: DataContract,
    token_id: Identifier,
    kind: Kind,
}

impl Fixture {
    /// A drive whose token keeps every kind of history, after a block executed a random `kind`
    /// from `MINIMUM` to `MAXIMUM`, sent by `OWNER` to `RECIPIENT` with `NOTE`, and settled it
    /// from `BEACON`: the history document holds the drawn amount.
    fn new(kind: Kind) -> Self {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut token = TokenConfigurationV0::default_most_restrictive().with_base_supply(0);
        token.keeps_history =
            TokenKeepsHistoryRulesV0::default_for_keeping_all_history(true).into();
        let contract = DataContract::V1(DataContractV1 {
            id: CONTRACT.into(),
            version: 0,
            owner_id: CONTRACT_OWNER.into(),
            document_types: Default::default(),
            config: DataContractConfigV0::default().into(),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: Default::default(),
            tokens: BTreeMap::from([(0, TokenConfiguration::V0(token))]),
            keywords: Vec::new(),
            description: None,
        });
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert the token contract");
        let history_contract =
            load_system_data_contract(SystemDataContract::TokenHistory, platform_version)
                .expect("expected the token history contract");
        drive
            .insert_contract(
                &history_contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert the token history contract");
        let token_id = contract
            .token_id(0)
            .expect("expected a token at position 0");

        // The sender's credit balance rides along in the proof
        let mut identity = Identity::random_identity(1, Some(14), platform_version)
            .expect("expected to create an identity");
        identity.set_id(OWNER.into());
        identity.set_balance(CREDITS);
        drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert the sender");
        drive
            .token_mint(
                token_id.to_buffer(),
                OWNER,
                BASE_SUPPLY,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint the base supply to the sender");

        // What the block's execution of the transition writes: the reservation, and the
        // history document holding the range's minimum
        let (reservation, event, request_kind) = match kind {
            Kind::Transfer => (
                TokenOperationType::TokenReserveRandomTransfer {
                    token_id,
                    sender_id: OWNER.into(),
                    recipient_id: RECIPIENT.into(),
                    identity_contract_nonce: NONCE,
                    maximum: MAXIMUM,
                },
                TokenEvent::Transfer(
                    RECIPIENT.into(),
                    Some(NOTE.to_string()),
                    None,
                    None,
                    MINIMUM,
                ),
                RandomTokenRequestKind::Transfer {
                    sender_id: OWNER.into(),
                    recipient_id: RECIPIENT.into(),
                },
            ),
            Kind::Mint => (
                TokenOperationType::TokenReserveRandomMint {
                    token_id,
                    owner_id: OWNER.into(),
                    recipient_id: RECIPIENT.into(),
                    identity_contract_nonce: NONCE,
                    maximum: MAXIMUM,
                    allow_first_mint: false,
                },
                TokenEvent::Mint(MINIMUM, RECIPIENT.into(), Some(NOTE.to_string())),
                RandomTokenRequestKind::Mint {
                    recipient_id: RECIPIENT.into(),
                },
            ),
        };
        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::TokenOperation(reservation),
                    DriveOperation::TokenOperation(TokenOperationType::TokenHistory {
                        token_id,
                        owner_id: OWNER.into(),
                        nonce: NONCE,
                        event,
                    }),
                ],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected the block to execute the random transition");

        // The settlement draws the amount and rewrites the history document to hold it
        drive
            .settle_random_token_amounts(
                &BEACON,
                &[RandomTokenRequest {
                    token_id,
                    owner_id: OWNER.into(),
                    identity_contract_nonce: NONCE,
                    minimum: MINIMUM,
                    maximum: MAXIMUM,
                    kind: request_kind,
                }],
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected the settlement to run");

        Self {
            drive,
            contract,
            token_id,
            kind,
        }
    }

    /// The random transition of the fixture's kind, as a client expecting it sends it: from
    /// `minimum` to `maximum`, to `recipient`, with `note`.
    fn transition(
        &self,
        minimum: TokenAmount,
        maximum: TokenAmount,
        recipient: [u8; 32],
        note: &str,
    ) -> StateTransition {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: NONCE,
            token_contract_position: 0,
            data_contract_id: CONTRACT.into(),
            token_id: self.token_id,
            using_group_info: None,
        });
        let token_transition = match self.kind {
            Kind::Transfer => {
                TokenTransition::Transfer(TokenTransferTransition::V1(TokenTransferTransitionV1 {
                    base,
                    amount: maximum,
                    recipient_id: recipient.into(),
                    public_note: Some(note.to_string()),
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                    random_minimum_amount: Some(minimum),
                }))
            }
            Kind::Mint => TokenTransition::Mint(TokenMintTransition::V1(TokenMintTransitionV1 {
                base,
                issued_to_identity_id: Some(recipient.into()),
                amount: maximum,
                public_note: Some(note.to_string()),
                random_minimum_amount: Some(minimum),
            })),
        };
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: OWNER.into(),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }))
    }

    /// The transition the fixture executed.
    fn genuine_transition(&self) -> StateTransition {
        self.transition(MINIMUM, MAXIMUM, RECIPIENT, NOTE)
    }

    /// The proof of the genuine transition's execution, from the drive's state.
    fn proof(&self) -> Vec<u8> {
        self.drive
            .prove_state_transition(&self.genuine_transition(), None, PlatformVersion::latest())
            .expect("expected to prove the transition")
            .into_data()
            .expect("expected proof bytes")
    }

    /// Verifies `proof` as the execution of `transition`, with the Merkle verifier a client
    /// runs.
    fn verify(
        &self,
        proof: &[u8],
        transition: &StateTransition,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            proof,
            &|_| Ok(Some(Arc::new(self.contract.clone()))),
            PlatformVersion::latest(),
        )
    }

    /// The amount the proved history document holds, read through the verification of the
    /// genuine transition.
    fn proved_amount(&self, proof: &[u8]) -> TokenAmount {
        let (root_hash, outcome) = self
            .verify(proof, &self.genuine_transition())
            .expect("expected the genuine transition to verify");
        assert_eq!(
            root_hash,
            self.drive
                .grove
                .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                .unwrap()
                .expect("expected the stored root hash")
        );
        match outcome.into_result() {
            VerifiedTokenActionWithDocument(document) => document
                .properties()
                .get_integer("amount")
                .expect("expected the history document to hold an amount"),
            other => panic!("expected the history document, got {other:?}"),
        }
    }
}

fn assert_unexpected_result(
    result: Result<(RootHash, StateTransitionProofOutcome), Error>,
    expectation: &str,
) {
    match result {
        Err(Error::Proof(ProofError::UnexpectedResultProof(_))) => {}
        Err(error) => {
            panic!("expected {expectation} to fail as an unexpected result, got {error:?}")
        }
        Ok(_) => panic!("expected {expectation} to fail, but it verified"),
    }
}

/// The proved amount verifies against a range holding it, its bounds included, and fails
/// against a range above or below it.
fn assert_amount_checked_against_the_range(kind: Kind) {
    let fixture = Fixture::new(kind);
    let proof = fixture.proof();
    let drawn = fixture.proved_amount(&proof);
    assert!(
        (MINIMUM..=MAXIMUM).contains(&drawn),
        "expected the settled amount {drawn} to lie in the range sent"
    );

    for (minimum, maximum) in [(drawn, drawn + 10), (drawn - 10, drawn), (drawn, drawn)] {
        fixture
            .verify(
                &proof,
                &fixture.transition(minimum, maximum, RECIPIENT, NOTE),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "expected the range {minimum} to {maximum} holding the proved amount \
                     {drawn} to verify, got {error:?}"
                )
            });
    }

    assert_unexpected_result(
        fixture.verify(
            &proof,
            &fixture.transition(drawn + 1, drawn + 10, RECIPIENT, NOTE),
        ),
        "the proved amount below the minimum",
    );
    assert_unexpected_result(
        fixture.verify(
            &proof,
            &fixture.transition(drawn - 10, drawn - 1, RECIPIENT, NOTE),
        ),
        "the proved amount above the maximum",
    );
}

/// Only the amount is exempt from the comparison with the expected history document: the
/// genuine range with another recipient or another note fails.
fn assert_other_fields_compared(kind: Kind) {
    let fixture = Fixture::new(kind);
    let proof = fixture.proof();
    fixture
        .verify(&proof, &fixture.genuine_transition())
        .expect("expected the genuine transition to verify");

    assert_unexpected_result(
        fixture.verify(
            &proof,
            &fixture.transition(MINIMUM, MAXIMUM, OTHER_RECIPIENT, NOTE),
        ),
        "another recipient",
    );
    assert_unexpected_result(
        fixture.verify(
            &proof,
            &fixture.transition(MINIMUM, MAXIMUM, RECIPIENT, "another note"),
        ),
        "another note",
    );
}

#[test]
fn should_verify_a_random_transfer_only_against_a_range_holding_the_proved_amount() {
    assert_amount_checked_against_the_range(Kind::Transfer);
}

#[test]
fn should_verify_a_random_mint_only_against_a_range_holding_the_proved_amount() {
    assert_amount_checked_against_the_range(Kind::Mint);
}

#[test]
fn should_still_compare_the_recipient_and_note_of_a_random_transfer() {
    assert_other_fields_compared(Kind::Transfer);
}

#[test]
fn should_still_compare_the_recipient_and_note_of_a_random_mint() {
    assert_other_fields_compared(Kind::Mint);
}
