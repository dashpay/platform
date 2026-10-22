//! Network ordering and the fetch fold, end to end: a load by index or a
//! refresh answered by a node behind the block an update landed in, and a
//! fetch that lands between an update's snapshot and its post-broadcast
//! apply.
//!
//! What is real and what is modelled:
//! - The fetches are real: `load_identity_by_index_from_master` (the FFI
//!   `platform_wallet_load_identity_at_index` path DashConnect calls before
//!   every update) and `refresh_identity`, answered by mock DAPI expectations.
//!   A lagging node is modelled by answering with the identity as it was one
//!   block before the update landed. The mock reports every proof at height
//!   0, which stands for "below the height the cache's balance came from".
//! - The update's broadcast is not run: the mock SDK cannot answer
//!   `broadcast_and_wait`. What runs instead is the update's post-broadcast
//!   local apply, `apply_landed_update`.
//! - The key-limits update's broadcast is not run either; its post-broadcast
//!   apply (`ManagedIdentity::replace_key`) is called as is.
//! - The balance a client read after the update (`refresh_identity_balance`)
//!   is recorded with `persist_refreshed_balance`, the call that path makes,
//!   at height 100 instead of the mock's flat 0.

use std::sync::Arc;

use dash_sdk::platform::transition::update_identity_key_limits::raised_key_limits;
use dash_sdk::platform::types::identity::PublicKeyHash;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;

use super::super::identity_handle::{derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX};
use super::update_revision;
use super::IdentityWallet;
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::persister::NoPlatformPersistence;
use crate::wallet::platform_wallet::WalletId;
use crate::{BlockTime, PlatformWallet, PlatformWalletManager};

const SEED: [u8; 64] = [7; 64];
/// Balance before the update, and after it paid its fee.
const BALANCE_BEFORE: u64 = 10_000_000;
const BALANCE_AFTER: u64 = 9_000_000;
/// The block the update landed in; a lagging node answers from the one below.
const LANDED_AT: u64 = 100;

fn key(id: u32, level: SecurityLevel) -> IdentityPublicKey {
    IdentityPublicKeyV0 {
        id,
        purpose: Purpose::AUTHENTICATION,
        security_level: level,
        key_type: KeyType::ECDSA_SECP256K1,
        data: vec![id as u8 + 1; KeyType::ECDSA_SECP256K1.default_size()].into(),
        ..Default::default()
    }
    .into()
}

/// The identity on Platform at revision 1: MASTER 0, HIGH 1, CRITICAL 2.
fn identity_at_revision_1() -> Identity {
    let mut identity = Identity::default_versioned(PlatformVersion::latest()).unwrap();
    identity.add_public_key(key(0, SecurityLevel::MASTER));
    identity.add_public_key(key(1, SecurityLevel::HIGH));
    identity.add_public_key(key(2, SecurityLevel::CRITICAL));
    identity.set_revision(1);
    identity.set_balance(BALANCE_BEFORE);
    identity
}

/// The update every scenario lands: add HIGH key 3, disable key 1.
fn update_payload() -> (Vec<IdentityPublicKey>, Vec<u32>) {
    (vec![key(3, SecurityLevel::HIGH)], vec![1])
}

/// The identity on Platform once the update landed at revision 2.
fn identity_at_revision_2() -> Identity {
    let mut identity = identity_at_revision_1();
    let mut key_1 = identity.public_keys()[&1].clone();
    key_1.set_disabled_at(1_700_000_000_000);
    identity.add_public_key(key_1);
    identity.add_public_key(key(3, SecurityLevel::HIGH));
    identity.set_revision(2);
    identity.set_balance(BALANCE_AFTER);
    identity
}

fn master() -> ExtendedPrivKey {
    ExtendedPrivKey::new_master(Network::Testnet, &SEED).unwrap()
}

/// The unique-hash probe a load by index 0 sends.
fn probe() -> PublicKeyHash {
    PublicKeyHash(
        derive_identity_auth_key_hash_from_master(&master(), Network::Testnet, 0, MASTER_KEY_INDEX)
            .unwrap(),
    )
}

/// A wallet from `SEED` holding the identity at revision 1 at index 0, as a
/// registration or an earlier load leaves it.
async fn wallet_on(sdk: dash_sdk::Sdk) -> Arc<PlatformWallet> {
    wallet_storing_to(sdk, Arc::new(NoPlatformPersistence)).await
}

/// [`wallet_on`], storing to `persister`.
async fn wallet_storing_to<P: PlatformWalletPersistence + 'static>(
    sdk: dash_sdk::Sdk,
    persister: Arc<P>,
) -> Arc<PlatformWallet> {
    struct Events;
    impl EventHandler for Events {}
    impl PlatformEventHandler for Events {}
    let manager = PlatformWalletManager::new(Arc::new(sdk), persister, Arc::new(Events));
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &SEED,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .unwrap();
    let mut wm = manager.wallet_manager.write().await;
    wm.get_wallet_info_mut(&wallet.wallet_id())
        .unwrap()
        .identity_manager
        .add_identity(
            identity_at_revision_1(),
            0,
            wallet.wallet_id(),
            &wallet.identity().persister,
        )
        .unwrap();
    drop(wm);
    wallet
}

async fn cached(wallet: &IdentityWallet) -> Identity {
    let wm = wallet.wallet_manager.read().await;
    wm.get_wallet_info(&wallet.wallet_id)
        .unwrap()
        .identity_manager
        .identity(&Identifier::default())
        .unwrap()
        .identity
        .clone()
}

/// What an identity update reads from the cache when it starts: the
/// revision its transition carries (cache + 1, the same in both trees).
async fn update_snapshot(wallet: &IdentityWallet) -> u64 {
    cached(wallet).await.revision() + 1
}

/// The post-broadcast local apply of an update Drive accepted at
/// `transition_revision`.
async fn land_update(
    wallet: &IdentityWallet,
    transition_revision: u64,
    added: Vec<IdentityPublicKey>,
    disabled: Vec<u32>,
) {
    wallet
        .apply_landed_update(
            &Identifier::default(),
            Some(0),
            transition_revision,
            added,
            disabled,
        )
        .await
        .unwrap();
}

/// A client read the balance after the update from an up-to-date node.
async fn balance_read_after_update(wallet: &IdentityWallet) {
    let mut wm = wallet.wallet_manager.write().await;
    let managed = wm
        .get_wallet_info_mut(&wallet.wallet_id)
        .unwrap()
        .identity_manager
        .managed_identity_mut(&Identifier::default())
        .unwrap();
    managed
        .persist_refreshed_balance(
            BALANCE_AFTER,
            BlockTime::new(LANDED_AT, 0, 1_700_000_000_000),
            &wallet.persister,
        )
        .unwrap();
}

#[derive(Debug)]
struct Seen {
    revision: u64,
    key_1_disabled: bool,
    has_key_3: bool,
    balance: u64,
}

fn seen(identity: &Identity) -> Seen {
    Seen {
        revision: identity.revision(),
        key_1_disabled: identity.public_keys()[&1].is_disabled(),
        has_key_3: identity.public_keys().contains_key(&3),
        balance: identity.balance(),
    }
}

#[derive(Clone, Copy, Debug)]
enum Fetch {
    /// `load_identity_by_index_from_master(0)`: DashConnect before an
    /// update, pull-to-refresh in Storage Explorer.
    Load,
    /// `refresh_identity`: the post-trade refresh of the DPNS marketplace.
    Refresh,
}

/// A mock SDK whose one node answers `fetch` with `answer`.
async fn sdk_answering(fetch: Fetch, answer: Identity) -> dash_sdk::Sdk {
    let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
    match fetch {
        Fetch::Load => {
            sdk.mock()
                .expect_fetch(probe(), Some(answer))
                .await
                .unwrap();
        }
        Fetch::Refresh => {
            sdk.mock()
                .expect_fetch(Identifier::default(), Some(answer))
                .await
                .unwrap();
        }
    }
    sdk
}

async fn run_fetch(wallet: &IdentityWallet, fetch: Fetch) {
    match fetch {
        Fetch::Load => {
            wallet
                .load_identity_by_index_from_master(0, &master())
                .await
                .unwrap()
                .unwrap();
        }
        Fetch::Refresh => {
            wallet
                .refresh_identity(&Identifier::default())
                .await
                .unwrap();
        }
    }
}

/// A load or refresh answered by a node one block behind an update (the
/// identity at revision 1, after the update landed at 2) keeps the cache at
/// the update: its revision, the disable, the added key and the balance the
/// update left, so the next update carries the revision Drive takes.
#[tokio::test]
async fn should_keep_a_landed_update_when_a_lagging_fetch_answers_older() {
    for fetch in [Fetch::Load, Fetch::Refresh] {
        let wallet = wallet_on(sdk_answering(fetch, identity_at_revision_1()).await).await;
        let wallet = wallet.identity();

        let transition_revision = update_snapshot(wallet).await;
        let (added, disabled) = update_payload();
        land_update(wallet, transition_revision, added, disabled).await;
        balance_read_after_update(wallet).await;
        let after_update = seen(&cached(wallet).await);
        assert_eq!(after_update.revision, 2, "{fetch:?}");
        assert!(after_update.key_1_disabled && after_update.has_key_3);
        assert_eq!(after_update.balance, BALANCE_AFTER);

        run_fetch(wallet, fetch).await;
        let after_fetch = seen(&cached(wallet).await);
        assert_eq!(after_fetch.revision, 2, "{fetch:?}: cache revision kept");
        assert!(after_fetch.key_1_disabled, "{fetch:?}: disabled key stays");
        assert!(after_fetch.has_key_3, "{fetch:?}: added key stays");
        assert_eq!(
            after_fetch.balance, BALANCE_AFTER,
            "{fetch:?}: balance from the newer proof stays"
        );
        // The next update carries cache + 1, which Drive takes: chain + 1.
        assert_eq!(after_fetch.revision + 1, 3, "{fetch:?}");
    }
}

/// A key-limits update raises key 3's budget 1_000 -> 5_000 and expiry
/// T1 -> T2 (no revision is claimed); a load or refresh answered by a node
/// one block behind, with the old limits at the same revision, keeps the
/// raised ones, so the next top-up is computed from what the chain holds.
#[tokio::test]
async fn should_keep_raised_key_limits_when_a_lagging_fetch_answers_older() {
    const T1: u64 = 1_800_000_000_000;
    const T2: u64 = 1_900_000_000_000;
    let limited =
        |budget, expires| key(3, SecurityLevel::HIGH).with_limits(Some(budget), Some(expires));
    let mut before = identity_at_revision_1();
    before.add_public_key(limited(1_000, T1));

    for fetch in [Fetch::Load, Fetch::Refresh] {
        let wallet = wallet_on(sdk_answering(fetch, before.clone()).await).await;
        let wallet = wallet.identity();
        {
            // The key as registered: the cache took it with the update that added it.
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&Identifier::default())
                .unwrap()
                .identity = before.clone();
        }
        {
            // Post-broadcast apply of `update_identity_key_limits_with_external_signer`.
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&Identifier::default())
                .unwrap()
                .replace_key(limited(5_000, T2), &wallet.persister)
                .unwrap();
        }

        run_fetch(wallet, fetch).await;
        let held = cached(wallet).await;
        let key_3 = held.public_keys()[&3].clone();
        assert_eq!(key_3.total_budget(), Some(5_000), "{fetch:?}: budget kept");
        assert_eq!(key_3.expires_at(), Some(T2), "{fetch:?}: expiry kept");
        // The next top-up of 500 is computed from the cached key: above the
        // 5_000 the chain holds, so consensus takes it as a raise.
        let next_top_up = raised_key_limits(&held, 3, Some(500), None).unwrap();
        assert_eq!(next_top_up.0, Some(5_500), "{fetch:?}");
    }
}

/// The update read revision 1 from the cache (transition at 2); while it
/// waits for its result, a load or refresh is answered by an up-to-date
/// node, which already holds the update (revision 2); then the
/// post-broadcast apply runs. The cache stays at the chain's revision, so
/// the next update is not refused, and keeps the on-chain `disabled_at`
/// rather than the local clock's.
#[tokio::test]
async fn should_not_put_the_cache_past_the_chain_when_a_fetch_lands_before_the_apply() {
    for fetch in [Fetch::Load, Fetch::Refresh] {
        let wallet = wallet_on(sdk_answering(fetch, identity_at_revision_2()).await).await;
        let wallet = wallet.identity();

        let transition_revision = update_snapshot(wallet).await;
        assert_eq!(transition_revision, 2);
        // ... nonce fetch, broadcast, the block commits at revision 2 ...
        run_fetch(wallet, fetch).await;
        assert_eq!(
            cached(wallet).await.revision(),
            2,
            "{fetch:?}: fetch took the landed update"
        );
        let (added, disabled) = update_payload();
        land_update(wallet, transition_revision, added, disabled).await;

        let held = cached(wallet).await;
        assert_eq!(held.revision(), 2, "{fetch:?}: cache at the chain");
        assert_eq!(
            held.public_keys()[&1].disabled_at(),
            identity_at_revision_2().public_keys()[&1].disabled_at(),
            "{fetch:?}: on-chain disable time kept"
        );
        assert!(held.public_keys().contains_key(&3), "{fetch:?}");
    }
}

/// The revision of the identity snapshot each store carried, in order.
#[derive(Default)]
struct RevisionLog(std::sync::Mutex<Vec<u64>>);

impl PlatformWalletPersistence for RevisionLog {
    fn store(
        &self,
        _wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if let Some(entry) = changeset
            .identities
            .and_then(|ids| ids.identities.get(&Identifier::default()).cloned())
        {
            self.0.lock().unwrap().push(entry.revision);
        }
        Ok(())
    }
    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

/// A base build hit the A3 race and persisted its result: a fetch took the
/// update at the chain's revision 2, then the post-broadcast bump stored
/// `cached + 1` = 3 over the chain's keys (and stamped the disable it
/// carried with the local clock). The app is then updated, and the restart
/// restores that state.
///
/// A load or refresh answered at the chain's revision 2 takes the cache back
/// to it, so the next update carries 3 and a caller pinning 3 passes the
/// revision check; the snapshot stored last carries 2. After an update that
/// only added keys the first answer does it. After one that also disabled a
/// key, the first answer replaces the local stamp with Platform's time and
/// the next one takes the revision.
#[tokio::test]
async fn should_take_back_a_revision_a_base_build_put_past_the_chain() {
    // The update added key 3 only.
    let mut added = identity_at_revision_1();
    added.add_public_key(key(3, SecurityLevel::HIGH));
    added.set_revision(2);
    added.set_balance(BALANCE_AFTER);
    // The scenarios' update: add key 3, disable key 1.
    let added_and_disabled = identity_at_revision_2();

    for (chain, answers_to_heal) in [(added, 1), (added_and_disabled, 2)] {
        for fetch in [Fetch::Load, Fetch::Refresh] {
            let log = Arc::new(RevisionLog::default());
            let wallet =
                wallet_storing_to(sdk_answering(fetch, chain.clone()).await, log.clone()).await;
            let wallet = wallet.identity();
            let mut restored = chain.clone();
            restored.set_revision(3);
            if let Some(key_1) = restored.public_keys().get(&1).cloned() {
                if key_1.is_disabled() {
                    let mut stamped = key_1;
                    stamped.set_disabled_at(1_800_000_000_000);
                    restored.add_public_key(stamped);
                }
            }
            {
                let mut wm = wallet.wallet_manager.write().await;
                wm.get_wallet_info_mut(&wallet.wallet_id)
                    .unwrap()
                    .identity_manager
                    .managed_identity_mut(&Identifier::default())
                    .unwrap()
                    .identity = restored;
            }
            let id = Identifier::default();
            assert!(
                update_revision(&id, cached(wallet).await.revision(), Some(3)).is_err(),
                "{fetch:?}: the restored state refuses a pinned 3"
            );

            for answer in 1..=answers_to_heal {
                run_fetch(wallet, fetch).await;
                let held = cached(wallet).await;
                if answer < answers_to_heal {
                    assert_eq!(held.revision(), 3, "{fetch:?}: stamps differ");
                    assert_eq!(held.public_keys(), chain.public_keys(), "{fetch:?}");
                }
            }
            let held = cached(wallet).await;
            assert_eq!(held.revision(), 2, "{fetch:?}: cache back at the chain");
            assert_eq!(held.public_keys(), chain.public_keys(), "{fetch:?}");
            assert_eq!(update_revision(&id, held.revision(), Some(3)).unwrap(), 3);
            assert_eq!(
                log.0.lock().unwrap().last(),
                Some(&2),
                "{fetch:?}: the store carries the chain's revision"
            );
        }
    }
}
