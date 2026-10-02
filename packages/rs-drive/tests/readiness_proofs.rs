//! Readiness proofs verified from outside the crate, through the public paths only.

#![cfg(all(feature = "server", feature = "verify"))]

use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive::verify::voting::VerifiedReadinessRound;

#[test]
fn should_verify_an_absent_readiness_round_into_its_public_result_type() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract_id = [7u8; 32];

    let proof = drive
        .prove_readiness_round(contract_id, None, platform_version)
        .expect("expected a proof");
    let (root_hash, round): (_, Option<VerifiedReadinessRound>) =
        Drive::verify_readiness_round(&proof, contract_id, false, platform_version)
            .expect("expected the proof to verify");

    assert_eq!(
        root_hash,
        drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash")
    );
    assert_eq!(round, None);
}
