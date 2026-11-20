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
    recipient_balance: u64,
}

fn metadata_at(height: u64) -> ResponseMetadata {
    ResponseMetadata {
        height,
        core_chain_locked_height: 1,
        time_ms: 1_700_000_000_000 + height * 3_000,
        protocol_version: PlatformVersion::latest().protocol_version,
        chain_id: "dash-testnet-51".into(),
        ..Default::default()
    }
}

fn signed_proof_at(
    drive: &Drive,
    transition: &StateTransition,
    height: u64,
) -> (Proof, ResponseMetadata) {
    let platform_version = PlatformVersion::latest();
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
    let metadata = metadata_at(height);

    (signed(bytes, &metadata, root), metadata)
}

fn fixture() -> Fixture {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let sender = Identity::random_identity(3, Some(1), platform_version).unwrap();
    let recipient = Identity::random_identity(3, Some(2), platform_version).unwrap();
    for identity in [&sender, &recipient] {
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .unwrap();
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
    let before = signed_proof_at(&drive, &transition, BLOCK_HEIGHT - 1);

    // What the transfer's block does to the two balances
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
    let after = signed_proof_at(&drive, &transition, BLOCK_HEIGHT);

    Fixture {
        request: BroadcastStateTransitionRequest {
            state_transition: transition.serialize_to_bytes().unwrap(),
        },
        before,
        after,
        sender_balance: sender.balance(),
        recipient_balance: recipient.balance(),
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

fn balances(result: &StateTransitionProofResult) -> (u64, u64) {
    let StateTransitionProofResult::VerifiedBalanceTransfer(sender, recipient) = result else {
        panic!("expected the balances of a credit transfer, got {result:?}");
    };
    (sender.balance.unwrap(), recipient.balance.unwrap())
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
        (fixture.sender_balance, fixture.recipient_balance)
    );
    assert_eq!(
        balances(verified.outcome.result()),
        (
            fixture.sender_balance - TRANSFER_AMOUNT,
            fixture.recipient_balance + TRANSFER_AMOUNT
        )
    );
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
            fixture.sender_balance - TRANSFER_AMOUNT,
            fixture.recipient_balance + TRANSFER_AMOUNT
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
