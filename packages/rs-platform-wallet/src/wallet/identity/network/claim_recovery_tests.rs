//! `recover_claimed_identity`: the identity a refused invitation claim named
//! is recorded in this wallet's slot only when it is the one registered with
//! the wallet's master key at the claimed index, and the id is compared
//! before anything is written. The Platform lookup is answered by mock DAPI
//! expectations; the DPNS lookup that follows a load is left unanswered, as a
//! load tolerates.

use std::sync::{Arc, Mutex};

use dash_sdk::platform::types::identity::PublicKeyHash;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;

use super::super::identity_handle::{derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX};
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::platform_wallet::WalletId;
use crate::{PlatformWallet, PlatformWalletManager};

const SEED: [u8; 64] = [9; 64];

#[derive(Default)]
struct Recording {
    stores: Mutex<Vec<PlatformWalletChangeSet>>,
}

impl PlatformWalletPersistence for Recording {
    fn store(
        &self,
        _wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        self.stores.lock().unwrap().push(changeset);
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

impl Recording {
    /// The last stored `(wallet_id, identity_index)` of `id`'s entry.
    fn stored_slot(&self, id: &Identifier) -> Option<(Option<WalletId>, Option<u32>)> {
        self.stores
            .lock()
            .unwrap()
            .iter()
            .filter_map(|cs| cs.identities.as_ref()?.identities.get(id))
            .next_back()
            .map(|entry| (entry.wallet_id, entry.identity_index))
    }
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

fn identity(id: u8) -> Identity {
    let mut identity = Identity::default_versioned(PlatformVersion::latest()).unwrap();
    identity.set_id(Identifier::from([id; 32]));
    identity.add_public_key(
        IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            data: vec![id; KeyType::ECDSA_SECP256K1.default_size()].into(),
            ..Default::default()
        }
        .into(),
    );
    identity.set_revision(1);
    identity
}

/// A wallet from `SEED` on a mock SDK whose node answers the index-0 probe
/// with `registered` (when given).
async fn wallet(registered: Option<Identity>) -> (Arc<PlatformWallet>, Arc<Recording>) {
    struct Events;
    impl EventHandler for Events {}
    impl PlatformEventHandler for Events {}
    let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
    if let Some(registered) = registered {
        sdk.mock()
            .expect_fetch(probe(), Some(registered))
            .await
            .unwrap();
    }
    let persistence = Arc::new(Recording::default());
    let manager =
        PlatformWalletManager::new(Arc::new(sdk), Arc::clone(&persistence), Arc::new(Events));
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &SEED,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .unwrap();
    (wallet, persistence)
}

async fn slot_of(wallet: &PlatformWallet, id: &Identifier) -> Option<u32> {
    let wm = wallet.identity().wallet_manager.read().await;
    wm.get_wallet_info(&wallet.wallet_id())
        .unwrap()
        .identity_manager
        .identity_index(id)
}

/// An identity only observed so far (looked up by name) that holds this
/// wallet's master key at the claimed index moves into the wallet's slot:
/// in memory and in the stored entry a restore reads.
#[tokio::test]
async fn should_move_an_observed_identity_into_the_claimed_slot() {
    let ours = identity(1);
    let (wallet, persistence) = wallet(Some(ours.clone())).await;
    {
        let mut wm = wallet.identity().wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_out_of_wallet_identity(ours.clone(), &wallet.identity().persister)
            .unwrap();
    }
    assert_eq!(slot_of(&wallet, &ours.id()).await, None);

    let recovered = wallet
        .identity()
        .recover_claimed_identity_from_master(0, ours.id(), &master())
        .await
        .unwrap();

    assert!(recovered);
    assert_eq!(slot_of(&wallet, &ours.id()).await, Some(0));
    assert_eq!(
        persistence.stored_slot(&ours.id()),
        Some((Some(wallet.wallet_id()), Some(0)))
    );
}

/// A different identity registered at the claimed index is not the one the
/// refusal named: the answer is `false` and nothing is recorded.
#[tokio::test]
async fn should_record_nothing_when_another_identity_holds_the_slot() {
    let other = identity(2);
    let (wallet, persistence) = wallet(Some(other.clone())).await;

    let recovered = wallet
        .identity()
        .recover_claimed_identity_from_master(0, identity(1).id(), &master())
        .await
        .unwrap();

    assert!(!recovered);
    {
        let wm = wallet.identity().wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet.wallet_id()).unwrap();
        assert!(info.identity_manager.identity(&other.id()).is_none());
    }
    assert_eq!(persistence.stored_slot(&other.id()), None);
}

/// An identity this wallet already holds, at any index, is recovered
/// without a lookup (the mock answers none).
#[tokio::test]
async fn should_recover_an_identity_the_wallet_holds_without_a_lookup() {
    let ours = identity(3);
    let (wallet, _) = wallet(None).await;
    {
        let mut wm = wallet.identity().wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                ours.clone(),
                4,
                wallet.wallet_id(),
                &wallet.identity().persister,
            )
            .unwrap();
    }

    let recovered = wallet
        .identity()
        .recover_claimed_identity_from_master(0, ours.id(), &master())
        .await
        .unwrap();

    assert!(recovered);
    assert_eq!(slot_of(&wallet, &ours.id()).await, Some(4));
}
