//! Run as a drive-abci example at the pre-migration revision documented in README.md.

use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::dashcore_rpc::json::QuorumType;
use drive_abci::config::ChainLockConfig;
use drive_abci::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    SignatureVerificationQuorumSetV0Methods, VerificationQuorum,
};

const PUBLIC_KEYS: [&str; 4] = [
    "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
    "a572cbea904d67468808c8eb50a9450c9721db309128012543902d0ac358a62ae28f75bb8f1c7c42c39a8c5529bf0f4e",
    "95fde78acd5f6886ddaf5d0056610167c513d09c1c0efabbc7cdcc69beea113779c4a81e2d24daafc5387dbf6ac5fe48",
    "b7f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
];

fn quorums(entries: &[(u8, usize, Option<u32>)]) -> Quorums<VerificationQuorum> {
    entries
        .iter()
        .map(|&(hash, key, index)| {
            (
                QuorumHash::from_byte_array([hash; 32]),
                VerificationQuorum {
                    public_key: hex::decode(PUBLIC_KEYS[key])
                        .unwrap()
                        .as_slice()
                        .try_into()
                        .unwrap(),
                    index,
                },
            )
        })
        .collect()
}

fn main() {
    let mut runtime = SignatureVerificationQuorumSetV0::new(&ChainLockConfig {
        quorum_type: QuorumType::Llmq400_60,
        quorum_size: 400,
        quorum_window: 288,
        quorum_active_signers: 4,
        quorum_rotation: false,
    });
    runtime.set_current_quorums(quorums(&[(0x11, 0, None), (0x22, 1, Some(0))]));
    runtime.set_previous_past_quorums(Quorums::default(), 899, 900);
    runtime.set_previous_past_quorums(quorums(&[(0x33, 2, Some(1)), (0x44, 3, None)]), 1000, 1008);
    // The normal runtime conversion writes V2; explicitly select the historical V0 writer.
    let stored = SignatureVerificationQuorumSetForSaving::V0(runtime.into());
    let bytes = bincode::encode_to_vec(
        stored,
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit(),
    )
    .unwrap();
    let destination = std::env::args().nth(1).expect("fixture output path");
    std::fs::write(destination, format!("{}\n", hex::encode(bytes))).unwrap();
}
