//! The proof of a random token transfer or mint (protocol version 15): its sender cannot know
//! the amount the block's random beacon drew, so verification takes it from the proved history
//! document once it lies in the range the transition asked for, and compares every other field
//! as for any transition. The document's id hashes the range (`token_history_document_id`), so
//! the proof binds the range it was drawn from. Each test verifies one genuine proof, built from
//! a drive that executed the transition (and settled it, for a random one), against
//! expectations that differ from the transition sent.

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
use dpp::group::GroupStateTransitionInfo;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_mint_transition::{
    TokenMintTransitionV0, TokenMintTransitionV1,
};
use dpp::state_transition::batch_transition::token_transfer_transition::{
    TokenTransferTransitionV0, TokenTransferTransitionV1,
};
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

/// What the block did with the fixture's transition.
#[derive(Clone, Copy)]
enum Execution {
    /// Executed the random transition and settled it from `BEACON`
    Settled,
    /// Executed the random transition with its history document holding `amount`, and settled
    /// nothing: a history whose amount the proof's range check judges
    HistoryHolding(TokenAmount),
    /// Executed a transition of exactly `MINIMUM` (format version 0) in its place
    Fixed,
}

/// The latest platform version with random token amounts active, which no protocol version
/// is until block execution settles them.
fn random_platform_version() -> PlatformVersion {
    let mut platform_version = PlatformVersion::latest().clone();
    platform_version
        .dpp
        .state_transition_serialization_versions
        .random_token_amount = Some(0);
    platform_version
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
        Self::executing(kind, Execution::Settled)
    }

    /// [`Self::new`], with the block doing `execution`: for [`Execution::Fixed`], a drive after
    /// a block executed a `kind` of exactly `MINIMUM`, sent by `OWNER` to `RECIPIENT` with
    /// `NOTE` and the same nonce ([`Self::fixed_transition`]).
    fn executing(kind: Kind, execution: Execution) -> Self {
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

        if let Execution::Fixed = execution {
            let (operation, event) = match kind {
                Kind::Transfer => (
                    TokenOperationType::TokenTransfer {
                        token_id,
                        sender_id: OWNER.into(),
                        recipient_id: RECIPIENT.into(),
                        amount: MINIMUM,
                    },
                    TokenEvent::Transfer(
                        RECIPIENT.into(),
                        Some(NOTE.to_string()),
                        None,
                        None,
                        MINIMUM,
                    ),
                ),
                Kind::Mint => (
                    TokenOperationType::TokenMint {
                        token_id,
                        identity_balance_holder_id: RECIPIENT.into(),
                        mint_amount: MINIMUM,
                        allow_first_mint: false,
                        allow_saturation: false,
                    },
                    TokenEvent::Mint(MINIMUM, RECIPIENT.into(), Some(NOTE.to_string())),
                ),
            };
            drive
                .apply_drive_operations(
                    vec![
                        DriveOperation::TokenOperation(operation),
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
                )
                .expect("expected the block to execute the fixed transition");
            return Fixture {
                drive,
                contract,
                token_id,
                kind,
            };
        }

        // What the block's execution of the transition writes: the reservation, and the
        // history document holding the range's minimum (or the amount the fixture asks for)
        let history_amount = match execution {
            Execution::HistoryHolding(amount) => amount,
            _ => MINIMUM,
        };
        let (reservation, event, request_kind) = match kind {
            Kind::Transfer => (
                TokenOperationType::TokenReserveRandomTransfer {
                    token_id,
                    sender_id: OWNER.into(),
                    recipient_id: RECIPIENT.into(),
                    identity_contract_nonce: NONCE,
                    minimum: MINIMUM,
                    maximum: MAXIMUM,
                },
                TokenEvent::Transfer(
                    RECIPIENT.into(),
                    Some(NOTE.to_string()),
                    None,
                    None,
                    history_amount,
                ),
                RandomTokenRequestKind::Transfer {
                    recipient_id: RECIPIENT.into(),
                },
            ),
            Kind::Mint => (
                TokenOperationType::TokenReserveRandomMint {
                    token_id,
                    owner_id: OWNER.into(),
                    recipient_id: RECIPIENT.into(),
                    identity_contract_nonce: NONCE,
                    minimum: MINIMUM,
                    maximum: MAXIMUM,
                    allow_first_mint: false,
                },
                TokenEvent::Mint(history_amount, RECIPIENT.into(), Some(NOTE.to_string())),
                RandomTokenRequestKind::Mint {
                    recipient_id: RECIPIENT.into(),
                },
            ),
        };
        drive
            .apply_drive_operations(
                vec![
                    DriveOperation::TokenOperation(reservation),
                    DriveOperation::TokenOperation(TokenOperationType::TokenRandomAmountHistory {
                        token_id,
                        owner_id: OWNER.into(),
                        nonce: NONCE,
                        event,
                        minimum: MINIMUM,
                        maximum: MAXIMUM,
                    }),
                ],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected the block to execute the random transition");

        // The settlement draws the amount and rewrites the history document to hold it
        if let Execution::Settled = execution {
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
        }

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

    /// The transition of the fixture's kind of exactly `MINIMUM` (format version 0), to
    /// `RECIPIENT` with `NOTE`, which [`Self::executing`] executes when not random.
    fn fixed_transition(&self) -> StateTransition {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: NONCE,
            token_contract_position: 0,
            data_contract_id: CONTRACT.into(),
            token_id: self.token_id,
            using_group_info: None,
        });
        let token_transition = match self.kind {
            Kind::Transfer => {
                TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                    base,
                    amount: MINIMUM,
                    recipient_id: RECIPIENT.into(),
                    public_note: Some(NOTE.to_string()),
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                }))
            }
            Kind::Mint => TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                base,
                issued_to_identity_id: Some(RECIPIENT.into()),
                amount: MINIMUM,
                public_note: Some(NOTE.to_string()),
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

    /// The proof of the genuine transition's execution, from the drive's state.
    fn proof(&self) -> Vec<u8> {
        self.proof_of(&self.genuine_transition())
    }

    /// The proof of `transition`'s execution, from the drive's state.
    fn proof_of(&self, transition: &StateTransition) -> Vec<u8> {
        self.drive
            .prove_state_transition(transition, None, PlatformVersion::latest())
            .expect("expected to prove the transition")
            .into_data()
            .expect("expected proof bytes")
    }

    /// Verifies `proof` as the execution of `transition`, with the Merkle verifier a client
    /// runs, at a version with random token amounts active.
    fn verify(
        &self,
        proof: &[u8],
        transition: &StateTransition,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        self.verify_at(proof, transition, &random_platform_version())
    }

    /// [`Self::verify`] at `platform_version`.
    fn verify_at(
        &self,
        proof: &[u8],
        transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            proof,
            &|_| Ok(Some(Arc::new(self.contract.clone()))),
            platform_version,
        )
    }

    /// The genuine random mint, asked for as the proposal of a group action, which a random
    /// mint may not be.
    fn random_group_mint_transition(&self) -> StateTransition {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: NONCE,
            token_contract_position: 0,
            data_contract_id: CONTRACT.into(),
            token_id: self.token_id,
            using_group_info: Some(GroupStateTransitionInfo {
                group_contract_position: 0,
                action_id: [9; 32].into(),
                action_is_proposer: true,
            }),
        });
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: OWNER.into(),
            transitions: vec![BatchedTransition::Token(TokenTransition::Mint(
                TokenMintTransition::V1(TokenMintTransitionV1 {
                    base,
                    issued_to_identity_id: Some(RECIPIENT.into()),
                    amount: MAXIMUM,
                    public_note: Some(NOTE.to_string()),
                    random_minimum_amount: Some(MINIMUM),
                }),
            ))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }))
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

fn assert_invalid_transition(
    result: Result<(RootHash, StateTransitionProofOutcome), Error>,
    expectation: &str,
) {
    match result {
        Err(Error::Proof(ProofError::InvalidTransition(_))) => {}
        Err(error) => {
            panic!("expected {expectation} to be refused as an invalid transition, got {error:?}")
        }
        Ok(_) => panic!("expected {expectation} to be refused, but it verified"),
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

/// The proof verifies against the range the transition was sent with, and against no other:
/// the history document's id hashes the range, so another range, even one holding the proved
/// amount, names another document, which the proof does not hold.
fn assert_bound_to_its_range(kind: Kind) {
    let fixture = Fixture::new(kind);
    let proof = fixture.proof();
    let drawn = fixture.proved_amount(&proof);
    assert!(
        (MINIMUM..=MAXIMUM).contains(&drawn),
        "expected the settled amount {drawn} to lie in the range sent"
    );

    for (minimum, maximum) in [
        (drawn, drawn),
        (MINIMUM, MAXIMUM + 1),
        (MINIMUM - 1, MAXIMUM),
        (drawn + 1, MAXIMUM + 10),
    ] {
        assert!(
            fixture
                .verify(
                    &proof,
                    &fixture.transition(minimum, maximum, RECIPIENT, NOTE),
                )
                .is_err(),
            "expected the range {minimum} to {maximum} not to verify against the proof of \
             {MINIMUM} to {MAXIMUM}"
        );
    }
}

/// The amount the history document of the requested range holds is checked against that
/// range: a history holding an amount just below its minimum or just above its maximum fails,
/// and one holding either end verifies with that amount.
fn assert_amount_checked_against_the_range(kind: Kind) {
    for (amount, inside) in [
        (MINIMUM - 1, false),
        (MINIMUM, true),
        (MAXIMUM, true),
        (MAXIMUM + 1, false),
    ] {
        let fixture = Fixture::executing(kind, Execution::HistoryHolding(amount));
        let proof = fixture.proof();
        if inside {
            assert_eq!(fixture.proved_amount(&proof), amount);
        } else {
            assert_unexpected_result(
                fixture.verify(&proof, &fixture.genuine_transition()),
                "a proved amount outside the requested range",
            );
        }
    }
}

/// The proof of a transition of a fixed amount inside the range, sent with the same nonce,
/// recipient and note, verifies as that transition and not as the random one: its history
/// document's id hashes no range, so the random transition names another document.
fn assert_fixed_amount_not_taken_for_a_random_one(kind: Kind) {
    let fixture = Fixture::executing(kind, Execution::Fixed);
    let proof = fixture.proof_of(&fixture.fixed_transition());
    fixture
        .verify(&proof, &fixture.fixed_transition())
        .expect("expected the fixed transition to verify");
    assert!(
        fixture
            .verify(&proof, &fixture.genuine_transition())
            .is_err(),
        "expected the proof of a fixed amount not to verify as the random transition"
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
fn should_verify_a_random_transfer_only_against_the_range_it_was_sent_with() {
    assert_bound_to_its_range(Kind::Transfer);
}

#[test]
fn should_verify_a_random_mint_only_against_the_range_it_was_sent_with() {
    assert_bound_to_its_range(Kind::Mint);
}

#[test]
fn should_check_the_proved_amount_of_a_random_transfer_against_its_range() {
    assert_amount_checked_against_the_range(Kind::Transfer);
}

#[test]
fn should_check_the_proved_amount_of_a_random_mint_against_its_range() {
    assert_amount_checked_against_the_range(Kind::Mint);
}

#[test]
fn should_refuse_a_random_group_mint_before_reading_the_proof() {
    // The platform refuses a random group mint, so no proof can show one executed: a fixed
    // group mint of the same owner, nonce, recipient and action would otherwise stand for it.
    // The refusal comes before any proof is read, so the proof here is any proof.
    let fixture = Fixture::new(Kind::Mint);
    let proof = fixture.proof();
    assert_invalid_transition(
        fixture.verify(&proof, &fixture.random_group_mint_transition()),
        "a random group mint",
    );
}

#[test]
fn should_refuse_a_random_transfer_or_mint_where_random_amounts_are_inactive() {
    let mut inactive = PlatformVersion::latest().clone();
    inactive
        .dpp
        .state_transition_serialization_versions
        .random_token_amount = None;
    for kind in [Kind::Transfer, Kind::Mint] {
        let fixture = Fixture::new(kind);
        let proof = fixture.proof();
        assert_invalid_transition(
            fixture.verify_at(&proof, &fixture.genuine_transition(), &inactive),
            "a random transition where random amounts are inactive",
        );
    }
}

#[test]
fn should_not_verify_the_proof_of_a_fixed_transfer_as_a_random_one() {
    assert_fixed_amount_not_taken_for_a_random_one(Kind::Transfer);
}

#[test]
fn should_not_verify_the_proof_of_a_fixed_mint_as_a_random_one() {
    assert_fixed_amount_not_taken_for_a_random_one(Kind::Mint);
}

#[test]
fn should_still_compare_the_recipient_and_note_of_a_random_transfer() {
    assert_other_fields_compared(Kind::Transfer);
}

#[test]
fn should_still_compare_the_recipient_and_note_of_a_random_mint() {
    assert_other_fields_compared(Kind::Mint);
}
