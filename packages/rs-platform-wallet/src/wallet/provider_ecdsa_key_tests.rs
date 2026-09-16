//! End-to-end coverage for the secp256k1 arm of
//! [`PlatformWallet::derive_provider_key_at_index`].
//!
//! The sibling unit tests in `provider_key_at_index.rs` pin the derivation
//! *paths* by re-deriving them from a bare key-wallet — useful, but they never
//! call the entry point the FFI calls, so `include_private`, the address / WIF
//! outputs, and the seed-vs-xpub cross-check went uncovered. That gap has
//! already cost once: the first cut of this arm used a wrapper that refuses
//! watch-only accounts, and every path test passed while the iOS app failed on
//! device, because those tests build seed-bearing wallets and never reach the
//! gate.
//!
//! These drive the real method.
//!
//! `derive_provider_key_at_index` is synchronous and takes the wallet-manager
//! lock with `blocking_read`, so it panics if called on an async executor
//! thread. The tests therefore run on a multi-thread runtime and call it
//! through `block_in_place` — which is also how a caller must treat it.

use std::sync::Arc;

use tokio::task::block_in_place;

use dashcore::Network;
use key_wallet::account::StandardAccountType;

use crate::test_support::{funded_wallet_manager, NoopTestPersister};
use crate::wallet::platform_wallet::PlatformWallet;
use crate::wallet::provider_key_at_index::ProviderKeyKind;

/// A `PlatformWallet` over a fresh, seed-bearing test wallet.
async fn platform_wallet() -> PlatformWallet {
    let (wallet_manager, wallet_id, balance, _signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&wallet_manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    PlatformWallet::new(
        sdk,
        wallet_id,
        wallet_manager,
        balance,
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(NoopTestPersister) as Arc<dyn crate::changeset::PlatformWalletPersistence>,
        Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
    )
}

/// The network the test wallet was created on, so address expectations are not
/// hardcoded to one chain.
async fn wallet_network(wallet: &PlatformWallet) -> Network {
    let manager = wallet.wallet_manager().read().await;
    manager
        .get_wallet(&wallet.wallet_id())
        .expect("wallet present")
        .network
}

/// A public listing must carry the address but no private material — the
/// secp256k1 public side comes off the account xpub and needs no seed at all.
#[tokio::test(flavor = "multi_thread")]
async fn public_listing_has_address_and_no_private_material() {
    let wallet = platform_wallet().await;

    for kind in [ProviderKeyKind::Owner, ProviderKeyKind::Voting] {
        let derived = block_in_place(|| wallet.derive_provider_key_at_index(kind, 0, None, false))
            .expect("public listing");

        assert!(
            derived.private_key.is_none(),
            "{kind:?}: no private scalar was requested"
        );
        assert!(
            derived.private_key_wif.is_none(),
            "{kind:?}: no WIF without a private reveal"
        );
        assert!(
            derived.address.is_some(),
            "{kind:?}: secp256k1 keys have an on-chain address"
        );
        assert!(
            derived.legacy_public_key_bytes.is_none() && derived.node_id.is_none(),
            "{kind:?}: BLS legacy form and platform node id belong to other curves"
        );
        assert_eq!(derived.index, 0);
    }
}

/// A private reveal must return a scalar and a WIF that agree with each other
/// and with the public key / address the same call reports.
///
/// This is the invariant a wrong derivation path breaks silently: every field
/// is individually well-formed, and only their agreement with the account's
/// real key exposes the mismatch.
#[tokio::test(flavor = "multi_thread")]
async fn private_reveal_is_internally_consistent() {
    use dashcore::PrivateKey;

    let wallet = platform_wallet().await;
    let network = wallet_network(&wallet).await;
    let secp = dashcore::key::Secp256k1::new();

    for kind in [ProviderKeyKind::Owner, ProviderKeyKind::Voting] {
        for index in [0u32, 1, 19] {
            let derived =
                block_in_place(|| wallet.derive_provider_key_at_index(kind, index, None, true))
                    .expect("private reveal");

            let scalar = derived.private_key.as_ref().expect("scalar requested");
            let wif = derived.private_key_wif.as_ref().expect("wif requested");
            let address = derived.address.as_ref().expect("address");

            // WIF and raw scalar must be the same key.
            let from_wif = PrivateKey::from_wif(wif).expect("valid WIF");
            assert_eq!(
                from_wif.inner.secret_bytes().to_vec(),
                **scalar,
                "{kind:?}#{index}: WIF and raw scalar disagree"
            );

            // The reported public key must be this private key's.
            assert_eq!(
                from_wif.public_key(&secp).to_bytes(),
                derived.public_key_bytes,
                "{kind:?}#{index}: public key does not belong to the returned private key"
            );

            // ...and the reported address must be that public key's P2PKH on
            // this wallet's own network, not a hardcoded chain.
            let expected = dashcore::Address::p2pkh(&from_wif.public_key(&secp), network);
            assert_eq!(
                address,
                &expected.to_string(),
                "{kind:?}#{index}: address is not the returned key's P2PKH"
            );
        }
    }
}

/// Distinct indexes must produce distinct keys — a derivation that ignored the
/// index would otherwise pass every consistency check above.
#[tokio::test(flavor = "multi_thread")]
async fn indexes_produce_distinct_keys() {
    let wallet = platform_wallet().await;

    let keys: Vec<Vec<u8>> = (0u32..5)
        .map(|index| {
            block_in_place(|| {
                wallet.derive_provider_key_at_index(ProviderKeyKind::Voting, index, None, false)
            })
            .expect("public listing")
            .public_key_bytes
        })
        .collect();

    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len(), "indexes collided");
}

/// Owner and voting keys live on different DIP-3 branches, so the same index
/// must not yield the same key for both.
#[tokio::test(flavor = "multi_thread")]
async fn owner_and_voting_do_not_share_keys() {
    let wallet = platform_wallet().await;

    let owner = block_in_place(|| {
        wallet.derive_provider_key_at_index(ProviderKeyKind::Owner, 0, None, false)
    })
    .expect("owner");
    let voting = block_in_place(|| {
        wallet.derive_provider_key_at_index(ProviderKeyKind::Voting, 0, None, false)
    })
    .expect("voting");

    assert_ne!(
        owner.public_key_bytes, voting.public_key_bytes,
        "owner and voting keys must come from different account paths"
    );
}

/// A supplied seed that does not belong to this wallet must be refused by the
/// cross-check rather than returning a (public, private) pair that disagrees.
///
/// Without this guard the caller signs for an address nobody expects, which has
/// no local symptom at all — on the voting family it surfaces only as Platform
/// rejecting the vote as having no voter identity.
#[tokio::test(flavor = "multi_thread")]
async fn foreign_seed_is_rejected_by_the_cross_check() {
    let wallet = platform_wallet().await;

    // A valid seed, just not this wallet's.
    let foreign_seed = [7u8; 64];
    let result = block_in_place(|| {
        wallet.derive_provider_key_at_index(ProviderKeyKind::Voting, 0, Some(&foreign_seed), true)
    });

    // `expect_err` would require `Debug` on `ProviderDerivedKey`, which holds
    // private key material — deriving it to satisfy a test would be a leak
    // waiting to happen in a log line.
    match result {
        Ok(_) => panic!("a foreign seed must not produce a key"),
        Err(err) => assert!(
            format!("{err:?}").contains("inconsistent"),
            "expected the cross-check to reject it, got: {err:?}"
        ),
    }
}

/// Supplying this wallet's own seed explicitly — the external-signable path,
/// where the app resolves the mnemonic from the Keychain — must produce the
/// same key as letting the wallet use its resident seed.
///
/// This is the shape that failed on device: the first cut went through a
/// wrapper gated on `is_watch_only`, which refuses exactly this caller.
#[tokio::test(flavor = "multi_thread")]
async fn explicitly_supplied_seed_matches_the_resident_one() {
    let wallet = platform_wallet().await;

    let resident = block_in_place(|| {
        wallet.derive_provider_key_at_index(ProviderKeyKind::Voting, 3, None, true)
    })
    .expect("resident-seed derivation");

    let seed = {
        let manager = wallet.wallet_manager().read().await;
        manager
            .get_wallet(&wallet.wallet_id())
            .expect("wallet present")
            .wallet_seed_bytes()
            .expect("test wallet is seed-bearing")
    };

    let supplied = block_in_place(|| {
        wallet.derive_provider_key_at_index(ProviderKeyKind::Voting, 3, Some(&seed), true)
    })
    .expect("supplied-seed derivation");

    assert_eq!(resident.public_key_bytes, supplied.public_key_bytes);
    assert_eq!(resident.address, supplied.address);
    assert_eq!(
        resident.private_key.as_deref(),
        supplied.private_key.as_deref(),
        "the same seed must derive the same scalar whichever way it is supplied"
    );
}
