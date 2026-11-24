//! A masternode withdrawal refused by its own key source, before anything
//! is signed, must leave the owner identity's nonce untouched: the SDK's
//! nonce cache never gives back a bumped nonce, and Platform refuses a nonce
//! more than 24 past the identity's, so every such refusal would bring the
//! node's later withdrawals closer to a nonce Platform rejects.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use dash_sdk::platform::Identifier;
use dash_sdk::query_types::IdentityNonceFetcher;
use dash_sdk::SdkBuilder;
use dashcore::hashes::hash160;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{Mnemonic, Network};
use platform_wallet::events::EventHandler;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{
    MasterKeyProvider, MasternodeKeySource, MasternodeWithdrawalError, MasternodeWithdrawalKey,
    MasternodeWithdrawalRequest, PlatformEventHandler, PlatformWalletManager,
};

const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                        abandon abandon about";
const PRO_TX_HASH: [u8; 32] = [0x31; 32];
const PLATFORM_NONCE: u64 = 5;

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

#[derive(Debug, thiserror::Error)]
#[error("the keychain is locked")]
struct NoMasterKey;

/// A keychain the user declined to unlock.
struct LockedKeychain;

#[async_trait]
impl MasterKeyProvider for LockedKeychain {
    type Error = NoMasterKey;

    async fn master_key(&self, _wallet_id: &WalletId) -> Result<ExtendedPrivKey, NoMasterKey> {
        Err(NoMasterKey)
    }
}

fn owner_identity_id() -> Identifier {
    let mut display = PRO_TX_HASH;
    display.reverse();
    Identifier::from(display)
}

/// Receive key `index` of the test wallet's BIP-44 account 0: its path and
/// pubkey hash.
fn receive_key(index: u32) -> (DerivationPath, [u8; 20]) {
    let seed = Mnemonic::from_phrase(MNEMONIC).unwrap().to_seed("");
    let master = ExtendedPrivKey::new_master(Network::Testnet, seed.as_ref()).unwrap();
    let path = DerivationPath::from_str(&format!("m/44'/1'/0'/0/{index}")).unwrap();
    let public = master.derive_priv(&path).unwrap().private_key.public_key();
    (
        path,
        hash160::Hash::hash(&public.to_compressed()).to_byte_array(),
    )
}

fn p2pkh_script(hash: &[u8; 20]) -> Vec<u8> {
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(hash);
    script.extend_from_slice(&[0x88, 0xac]);
    script
}

fn owner_identity(payout_hash: [u8; 20]) -> Identity {
    let key: IdentityPublicKey = IdentityPublicKeyV0 {
        id: 0,
        key_type: KeyType::ECDSA_HASH160,
        purpose: Purpose::TRANSFER,
        security_level: SecurityLevel::CRITICAL,
        read_only: true,
        data: BinaryData::new(payout_hash.to_vec()),
        disabled_at: None,
        contract_bounds: None,
    }
    .into();
    IdentityV0 {
        id: owner_identity_id(),
        public_keys: [(key.id(), key)].into_iter().collect(),
        balance: 1_000_000_000,
        revision: 0,
    }
    .into()
}

/// The user declines the keychain prompt for a wallet without resident
/// keys: the identity nonce the withdrawal would have signed with stays
/// unused.
#[test]
fn should_take_no_identity_nonce_when_the_master_key_provider_fails() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (payout_path, payout_hash) = receive_key(400);

    let mut sdk = SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .build()
        .unwrap();
    runtime.block_on(async {
        sdk.mock()
            .expect_fetch(owner_identity_id(), Some(owner_identity(payout_hash)))
            .await
            .unwrap()
            .expect_fetch(
                owner_identity_id(),
                Some(IdentityNonceFetcher(PLATFORM_NONCE)),
            )
            .await
            .unwrap();
    });
    // The manager spawns its tasks on the runtime it is built in.
    let manager = {
        let _runtime = runtime.enter();
        PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoPlatformPersistence),
            Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>,
        )
    };
    let wallet = runtime
        .block_on(manager.create_wallet_from_mnemonic(
            MNEMONIC,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
            Some(0),
        ))
        .unwrap();
    let keys = wallet
        .masternode_withdrawal_keys(
            None,
            Some(&p2pkh_script(&payout_hash)),
            None,
            Some(&payout_path),
        )
        .unwrap();
    let request = MasternodeWithdrawalRequest {
        pro_tx_hash: PRO_TX_HASH,
        owner_key_hash: None,
        amount_credits: 100_000_000,
        signing_key: MasternodeWithdrawalKey::Transfer,
        destination: None,
    };

    let result = runtime.block_on(manager.masternode_withdraw_with(
        request,
        MasternodeKeySource::Wallet {
            wallet: &wallet,
            keys: &keys,
            master_key: &LockedKeychain,
        },
    ));

    assert!(
        matches!(
            result,
            Err(MasternodeWithdrawalError::KeySource(NoMasterKey))
        ),
        "{result:?}"
    );
    let next = runtime
        .block_on(
            manager
                .sdk()
                .get_identity_nonce(owner_identity_id(), false, None),
        )
        .unwrap();
    assert_eq!(
        next, PLATFORM_NONCE,
        "a withdrawal stopped before signing must not consume an identity nonce"
    );
}
