use dpp::prelude::Identifier;
use drive_proof_verifier::types::IdentityContractNonceFetcher;
use rs_sdk::platform::Fetch;

use super::{common::setup_logs, config::Config};

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_contract_no_nonce_read() {
    setup_logs();

    let cfg = Config::new();
    let identity_id: dpp::prelude::Identifier = cfg.existing_identity_id;
    // We are putting a contract id that does not exist, hence we will never get a nonce
    let contract_id: dpp::prelude::Identifier = Identifier::from_bytes(&[5u8; 32]).unwrap();

    let sdk = cfg.setup_api("test_identity_contract_no_nonce_read").await;

    let identity_contract_nonce =
        IdentityContractNonceFetcher::fetch(&sdk, (identity_id, contract_id))
            .await
            .expect("fetch identity contract nonce");

    assert!(identity_contract_nonce.is_none());
}

// Todo: add this test when we have a mock wallet
// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_identity_contract_nonce_read() {
//     setup_logs();
//
//     let cfg = Config::new();
//     let identity_id: dpp::prelude::Identifier = cfg.existing_identity_id;
//     let contract_id: dpp::prelude::Identifier = cfg.existing_data_contract_id;
//
//     let sdk = cfg.setup_api().await;
//
//     let identity_contract_nonce =
//         IdentityContractNonceFetcher::fetch(&sdk, (identity_id, contract_id))
//             .await
//             .expect("fetch identity contract nonce")
//             .expect("found identity contract nonce")
//             .0;
//
//     assert_eq!(identity_contract_nonce, 1);
// }
