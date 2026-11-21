//! Genuine signed proofs of the state before and after the block of a state transition, run
//! through the verifier a client uses.
// The module is shared between suites, and this one uses only a part of it.
#[allow(dead_code)]
mod signed_response;

use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
    wait_for_state_transition_result_response_v0::Result as WaitResult, StateBeforeBlock, Version,
    WaitForStateTransitionResultResponseV0,
};
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, Proof, ResponseMetadata, WaitForStateTransitionResultResponse,
};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identity;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive_proof_verifier::{
    verify_state_transition_result_around_block, StateBeforeBlockVerification,
    StateTransitionResultAroundBlock,
};
use signed_response::{signed, Provider};

const TRANSFER_AMOUNT: u64 = 500_000;
const BLOCK_HEIGHT: u64 = 10;

/// A credit transfer, the proofs of its keys before and after a block that applies it, each
/// signed for its own block, and the balances the sender and the recipient started with.
struct Fixture {
    request: BroadcastStateTransitionRequest,
    before: (Proof, ResponseMetadata),
    after: (Proof, ResponseMetadata),
    sender_balance: u64,
    /// `None` when the recipient does not exist before the transfer's block
    recipient_balance: Option<u64>,
    recipient_balance_after: u64,
}

fn metadata_at(height: u64, protocol_version: u32) -> ResponseMetadata {
    ResponseMetadata {
        height,
        core_chain_locked_height: 1,
        time_ms: 1_700_000_000_000 + height * 3_000,
        protocol_version,
        chain_id: "dash-testnet-51".into(),
        ..Default::default()
    }
}

fn signed_proof_at(
    drive: &Drive,
    transition: &StateTransition,
    height: u64,
    platform_version: &PlatformVersion,
) -> (Proof, ResponseMetadata) {
    let bytes = drive
        .prove_state_transition(transition, None, platform_version)
        .unwrap()
        .into_data()
        .unwrap();
    let root = drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .unwrap();
    let metadata = metadata_at(height, platform_version.protocol_version);

    (signed(bytes, &metadata, root), metadata)
}

fn fixture() -> Fixture {
    fixture_with(true, PlatformVersion::latest().protocol_version)
}

/// A transfer whose recipient exists before its block or is created in it, with the proof of
/// the state before the block made and signed at `before_protocol_version`.
fn fixture_with(recipient_exists_before: bool, before_protocol_version: u32) -> Fixture {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let add = |identity: &Identity| {
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .unwrap()
    };

    let sender = Identity::random_identity(3, Some(1), platform_version).unwrap();
    let recipient = Identity::random_identity(3, Some(2), platform_version).unwrap();
    add(&sender);
    if recipient_exists_before {
        add(&recipient);
    }

    let transition: StateTransition =
        IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
            identity_id: sender.id(),
            recipient_id: recipient.id(),
            amount: TRANSFER_AMOUNT,
            nonce: 1,
            ..Default::default()
        })
        .into();

    // The state the block before the transfer's left
    let before = signed_proof_at(
        &drive,
        &transition,
        BLOCK_HEIGHT - 1,
        PlatformVersion::get(before_protocol_version).unwrap(),
    );

    // What the transfer's block does to the two balances, creating the recipient first when it
    // did not exist before
    if !recipient_exists_before {
        add(&recipient);
    }
    drive
        .remove_from_identity_balance(
            sender.id().to_buffer(),
            TRANSFER_AMOUNT,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .unwrap();
    drive
        .add_to_identity_balance(
            recipient.id().to_buffer(),
            TRANSFER_AMOUNT,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .unwrap();

    // The state the transfer's block left
    let after = signed_proof_at(&drive, &transition, BLOCK_HEIGHT, platform_version);

    Fixture {
        request: BroadcastStateTransitionRequest {
            state_transition: transition.serialize_to_bytes().unwrap(),
        },
        before,
        after,
        sender_balance: sender.balance(),
        recipient_balance: recipient_exists_before.then(|| recipient.balance()),
        recipient_balance_after: recipient.balance() + TRANSFER_AMOUNT,
    }
}

fn response(
    after: (Proof, ResponseMetadata),
    before: Option<(Proof, ResponseMetadata)>,
) -> WaitForStateTransitionResultResponse {
    WaitForStateTransitionResultResponse {
        version: Some(Version::V0(WaitForStateTransitionResultResponseV0 {
            result: Some(WaitResult::Proof(after.0)),
            metadata: Some(after.1),
            state_before_block: before.map(|(proof, metadata)| StateBeforeBlock {
                proof: Some(proof),
                metadata: Some(metadata),
            }),
        })),
    }
}

fn verify(
    request: BroadcastStateTransitionRequest,
    response: WaitForStateTransitionResultResponse,
) -> Result<StateTransitionResultAroundBlock, Box<drive_proof_verifier::Error>> {
    verify_state_transition_result_around_block(
        request,
        response,
        Network::Testnet,
        PlatformVersion::latest(),
        &Provider,
    )
    .map_err(Box::new)
}

/// The sender's and the recipient's balances a transfer's proof holds, `None` for one that does
/// not exist.
type TransferBalances = (Option<u64>, Option<u64>);

/// The balances a transfer's proof holds.
fn balances(result: &StateTransitionProofResult) -> TransferBalances {
    let StateTransitionProofResult::VerifiedBalanceTransfer(sender, recipient) = result else {
        panic!("expected the balances of a credit transfer, got {result:?}");
    };
    (sender.balance, recipient.balance)
}

/// Verifies the fixture's pair and returns the balances before and after the block.
fn verified_balances(fixture: Fixture) -> (TransferBalances, TransferBalances) {
    let verified = verify(
        fixture.request,
        response(fixture.after, Some(fixture.before)),
    )
    .expect("expected both proofs to verify");
    let StateBeforeBlockVerification::Verified(before) = &verified.state_before_block else {
        panic!(
            "expected the state before the block, got {:?}",
            verified.state_before_block
        );
    };
    (balances(&before.state), balances(verified.outcome.result()))
}

#[test]
fn should_verify_the_state_before_and_after_the_block_of_a_credit_transfer() {
    let fixture = fixture();

    let verified = verify(
        fixture.request,
        response(fixture.after, Some(fixture.before)),
    )
    .expect("expected both proofs to verify");

    let StateBeforeBlockVerification::Verified(before) = &verified.state_before_block else {
        panic!(
            "expected the state before the block, got {:?}",
            verified.state_before_block
        );
    };

    // Each state is of its own block, and the two blocks follow one another
    assert_eq!(before.metadata.height, BLOCK_HEIGHT - 1);
    assert_eq!(verified.metadata.height, BLOCK_HEIGHT);

    // And the two states differ by what the block did
    assert_eq!(
        balances(&before.state),
        (Some(fixture.sender_balance), fixture.recipient_balance)
    );
    assert_eq!(
        balances(verified.outcome.result()),
        (
            Some(fixture.sender_balance - TRANSFER_AMOUNT),
            Some(fixture.recipient_balance_after)
        )
    );
}

#[test]
fn should_verify_a_recipient_created_in_the_block_as_absent_before_it() {
    let fixture = fixture_with(false, PlatformVersion::latest().protocol_version);
    let sender_balance = fixture.sender_balance;
    let recipient_balance_after = fixture.recipient_balance_after;

    let (before, after) = verified_balances(fixture);

    assert_eq!(before, (Some(sender_balance), None));
    assert_eq!(
        after,
        (
            Some(sender_balance - TRANSFER_AMOUNT),
            Some(recipient_balance_after)
        )
    );
}

#[test]
fn should_verify_a_state_before_the_block_signed_at_an_older_protocol_version() {
    // The network upgraded between the two blocks
    let fixture = fixture_with(true, 13);
    let sender_balance = fixture.sender_balance;
    let recipient_balance = fixture.recipient_balance;

    let (before, _) = verified_balances(fixture);

    assert_eq!(before, (Some(sender_balance), recipient_balance));
}

#[test]
fn should_verify_a_response_without_a_state_before_the_block() {
    let fixture = fixture();

    let verified = verify(fixture.request, response(fixture.after, None))
        .expect("expected the response's proof to verify");

    assert!(matches!(
        verified.state_before_block,
        StateBeforeBlockVerification::NotInResponse
    ));
    assert_eq!(
        balances(verified.outcome.result()),
        (
            Some(fixture.sender_balance - TRANSFER_AMOUNT),
            Some(fixture.recipient_balance_after)
        )
    );
}

#[test]
fn should_refuse_a_state_before_the_block_that_its_block_did_not_sign() {
    let fixture = fixture();

    // The proof of the state after the block, passed off as the state the block before left:
    // the signature of the block before is over another root
    let passed_off = (
        Proof {
            grovedb_proof: fixture.after.0.grovedb_proof.clone(),
            ..fixture.before.0.clone()
        },
        fixture.before.1.clone(),
    );

    let error = verify(fixture.request, response(fixture.after, Some(passed_off)))
        .expect_err("expected the state before the block to be refused");

    assert!(
        matches!(
            *error,
            drive_proof_verifier::Error::InvalidSignature { .. }
                | drive_proof_verifier::Error::SignatureVerificationError { .. }
        ),
        "{error:?}"
    );
}

#[test]
fn should_refuse_a_state_before_the_block_signed_for_another_height() {
    let fixture = fixture();

    // The metadata says the block before, the signature is of the block itself
    let mislabelled = (fixture.after.0.clone(), fixture.before.1.clone());

    let error = verify(fixture.request, response(fixture.after, Some(mislabelled)))
        .expect_err("expected the state before the block to be refused");

    assert!(
        matches!(
            *error,
            drive_proof_verifier::Error::InvalidSignature { .. }
                | drive_proof_verifier::Error::SignatureVerificationError { .. }
        ),
        "{error:?}"
    );
}
