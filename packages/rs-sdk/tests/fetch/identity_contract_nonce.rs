use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::prelude::IdentityPublicKey;
use dpp::{identity::hash::IdentityPublicKeyHashMethodsV0, prelude::Identity};
use drive_proof_verifier::types::{
    IdentityBalance, IdentityBalanceAndRevision, IdentityContractNonceFetcher,
};
use rs_sdk::platform::types::identity::PublicKeyHash;
use rs_sdk::platform::{Fetch, FetchMany};

use super::{common::setup_logs, config::Config};

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_contract_nonce_read() {
    setup_logs();

    let cfg = Config::new();
    let identity_id: dpp::prelude::Identifier = cfg.existing_identity_id;
    let contract_id: dpp::prelude::Identifier = cfg.existing_data_contract_id;

    let sdk = cfg.setup_api().await;

    let identity_contract_nonce =
        IdentityContractNonceFetcher::fetch(&sdk, (identity_id, contract_id))
            .await
            .expect("fetch identity contract nonce")
            .expect("found identity contract nonce")
            .0;

    assert_eq!(identity_contract_nonce, 1);
}
