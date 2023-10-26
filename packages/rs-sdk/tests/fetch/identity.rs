use dpp::identity::accessors::IdentityGettersV0;
use dpp::{identity::hash::IdentityPublicKeyHashMethodsV0, prelude::Identity};

use drive_proof_verifier::types::{IdentityBalance, IdentityBalanceAndRevision};
use rs_sdk::platform::identity::PublicKeyHash;
use rs_sdk::platform::Fetch;

use crate::common::{setup_logs, Config};

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_read() {
    setup_logs();

    use dpp::identity::accessors::IdentityGettersV0;
    let cfg = Config::new();
    let id: dpp::prelude::Identifier = cfg.settings.existing_identity_id;

    let mut api = cfg.setup_api().await;

    let identity = Identity::fetch(&mut api, id)
        .await
        .expect("fetch identity")
        .expect("found identity");

    assert_eq!(identity.id(), id);
}

/// Given some existing identity public key, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_read_by_key() {
    let cfg = Config::new();
    let id = cfg.settings.existing_identity_id;

    let mut api = cfg.setup_api().await;

    let identity = Identity::fetch(&mut api, id)
        .await
        .expect("fetch identity")
        .expect("found identity");

    let key_hash = identity
        .public_keys()
        .first_key_value()
        .expect("need at least one pubkey")
        .1
        .hash()
        .expect("public key hash");

    let identity2 = Identity::fetch(&mut api, PublicKeyHash(key_hash))
        .await
        .expect("fetch identity by key hash")
        .expect("found identity by key hash");
    assert_eq!(identity2, identity);
}

/// Given some existing identity ID, when I fetch the identity balance, I get some number.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_balance_read() {
    setup_logs();

    let cfg = Config::new();
    let id: dpp::prelude::Identifier = cfg.settings.existing_identity_id;

    let mut api = cfg.setup_api().await;

    let balance = IdentityBalance::fetch(&mut api, id)
        .await
        .expect("fetch identity balance")
        .expect("found identity balance");

    assert_ne!(0, balance);
    tracing::debug!(balance, ?id, "identity balance")
}

/// Given some existing identity ID, when I fetch the identity balance, I get some number.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_balance_revision_read() {
    setup_logs();

    let cfg = Config::new();
    let id: dpp::prelude::Identifier = cfg.settings.existing_identity_id;

    let mut api = cfg.setup_api().await;

    let (balance, revision) = IdentityBalanceAndRevision::fetch(&mut api, id)
        .await
        .expect("fetch identity balance")
        .expect("found identity balance");

    assert_ne!(0, balance);
    tracing::debug!(balance, revision, ?id, "identity balance and revision")
}
