use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::prelude::IdentityPublicKey;
use dpp::{identity::hash::IdentityPublicKeyHashMethodsV0, prelude::Identity};
use drive_proof_verifier::types::{IdentityBalance, IdentityBalanceAndRevision};
use rs_sdk::platform::types::identity::PublicKeyHash;
use rs_sdk::platform::{Fetch, FetchMany};

use super::{common::setup_logs, config::Config};

/// Given some existing identity ID, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_read() {
    setup_logs();

    use dpp::identity::accessors::IdentityGettersV0;
    let cfg = Config::new();
    let id: dpp::prelude::Identifier = cfg.existing_identity_id;

    let sdk = cfg.setup_api("test_identity_read").await;

    let identity = Identity::fetch(&sdk, id)
        .await
        .expect("fetch identity")
        .expect("found identity");

    assert_eq!(identity.id(), id);
}

/// Given some existing identity public key, when I fetch the identity, and I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_read_by_key() {
    let cfg = Config::new();
    let id = cfg.existing_identity_id;

    let sdk = cfg.setup_api("test_identity_read_by_key").await;

    let identity = Identity::fetch(&sdk, id)
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

    let identity2 = Identity::fetch(&sdk, PublicKeyHash(key_hash))
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
    let id: dpp::prelude::Identifier = cfg.existing_identity_id;

    let sdk = cfg.setup_api("test_identity_balance_read").await;

    let balance = IdentityBalance::fetch(&sdk, id)
        .await
        .expect("fetch identity balance")
        .expect("found identity balance");

    tracing::debug!(balance, ?id, "identity balance")
}

/// Given some existing identity ID, when I fetch the identity balance, I get some number.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_balance_revision_read() {
    setup_logs();

    let cfg = Config::new();
    let id: dpp::prelude::Identifier = cfg.existing_identity_id;

    let sdk = cfg.setup_api("test_identity_balance_revision_read").await;

    let (balance, revision) = IdentityBalanceAndRevision::fetch(&sdk, id)
        .await
        .expect("fetch identity balance")
        .expect("found identity balance");

    tracing::debug!(balance, revision, ?id, "identity balance and revision")
}

/// Given some existing identity ID, when I fetch the identity keys, I get some of them indexed by key ID.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_public_keys_all_read() {
    setup_logs();

    let cfg = Config::new();
    let id: dpp::prelude::Identifier = cfg.existing_identity_id;

    let sdk = cfg.setup_api("test_identity_public_keys_all_read").await;

    let public_keys = IdentityPublicKey::fetch_many(&sdk, id)
        .await
        .expect("fetch identity public keys");

    assert!(!public_keys.is_empty());
    tracing::debug!(?public_keys, ?id, "fetched identity public keys");

    // key IDs must match
    for item in public_keys {
        let id = item.0;
        let pubkey = item.1.expect("public key should exist");

        assert_eq!(id, pubkey.id());
    }
}
