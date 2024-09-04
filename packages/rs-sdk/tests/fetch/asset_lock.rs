use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::{GetEpochsInfoRequest, GetEpochsInfoResponse};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_sdk::platform::transition::asset_lock::AssetLockProofVerifier;
use dpp::dashcore::consensus::deserialize;
use dpp::dashcore::hash_types::CycleHash;
use dpp::dashcore::hashes::hex::FromHex;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{InstantLock, Transaction};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;
use rs_dapi_client::DapiRequest;

use super::{common::setup_logs, config::Config};

async fn current_platform_state(sdk: &dash_sdk::Sdk) -> (u32, Vec<u8>) {
    let req: GetEpochsInfoRequest = GetEpochsInfoRequestV0 {
        ascending: false,
        count: 1,
        prove: true,
        start_epoch: None,
    }
    .into();

    let resp: GetEpochsInfoResponse = req
        .execute(sdk, Default::default())
        .await
        .expect("get epoch info");
    let core_height = resp.metadata().expect("metadata").core_chain_locked_height;
    let quorum_hash = resp.proof().expect("proof").quorum_hash.clone();
    (core_height, quorum_hash)
}

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_asset_lock_proof() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_asset_lock_proof").await;
    let (core_chain_locked_height, quorum_hash) = current_platform_state(&sdk).await;

    // some semi-correct instant lock
    let cyclehash = CycleHash::from_slice(&quorum_hash).expect("cycle hash");
    let instant_lock = InstantLock {
        cyclehash,
        ..Default::default()
    };

    let out_point = [0u8; 36];

    // some hardcoded tx, just for tests
    let tx_bytes = Vec::from_hex(
        "010000000001000100e1f505000000001976a9140389035a9225b3839e2bbf32d826a1e222031fd888ac00000000"
    ).unwrap();
    let tx: Transaction = deserialize(&tx_bytes).expect("deserialize tx");

    struct TestCase {
        asset_lock_proof: AssetLockProof,
        // expect err that can be retried
        expect_err: bool,
    }
    // instant_lock: InstantLock, transaction: Transaction, output_index: u32
    let test_cases = vec![
        TestCase {
            asset_lock_proof: AssetLockProof::Chain(ChainAssetLockProof::new(
                core_chain_locked_height,
                out_point,
            )),
            expect_err: false,
        },
        TestCase {
            asset_lock_proof: AssetLockProof::Instant(InstantAssetLockProof::new(
                instant_lock,
                tx.clone(),
                0,
            )),
            expect_err: false,
        },
        TestCase {
            asset_lock_proof: AssetLockProof::Instant(InstantAssetLockProof::new(
                InstantLock::default(),
                tx,
                0,
            )),
            expect_err: true,
        },
        TestCase {
            asset_lock_proof: AssetLockProof::Chain(ChainAssetLockProof::new(
                core_chain_locked_height + 100,
                out_point,
            )),
            expect_err: true,
        },
    ];

    for (i, tc) in test_cases.into_iter().enumerate() {
        let result = tc.asset_lock_proof.verify(&sdk).await;
        assert_eq!(
            result.is_err(),
            tc.expect_err,
            "tc {} expeced err = {}, got err = {}: {:?}",
            i,
            tc.expect_err,
            result.is_err(),
            result
        );
    }
}
