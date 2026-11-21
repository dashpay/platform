//! A host-held key source for a masternode withdrawal, against a mocked
//! Platform: which owner identity key it signs for, and where a payout-key
//! withdrawal without a destination pays.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use dash_sdk::platform::Identifier;
use dash_sdk::query_types::IdentityNonceFetcher;
use dash_sdk::SdkBuilder;
use dpp::address_funds::AddressWitness;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use platform_wallet::events::EventHandler;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{
    MasternodeKeySource, MasternodeWithdrawalError, MasternodeWithdrawalKey,
    MasternodeWithdrawalRequest, PlatformEventHandler, PlatformWalletError, PlatformWalletManager,
};

const STALE: [u8; 20] = [0x01; 20];
const CURRENT: [u8; 20] = [0x02; 20];
const OWNER: [u8; 20] = [0x11; 20];

/// A proTxHash whose wire and display orders differ, so the owner identity
/// is found only under the display order.
fn wire_pro_tx_hash() -> [u8; 32] {
    let mut wire = [0u8; 32];
    for (byte, value) in wire.iter_mut().zip(1u8..) {
        *byte = value;
    }
    wire
}

fn owner_identity_id() -> Identifier {
    let mut display = wire_pro_tx_hash();
    display.reverse();
    Identifier::from(display)
}

fn key(id: KeyID, purpose: Purpose, hash: [u8; 20], disabled: bool) -> IdentityPublicKey {
    IdentityPublicKeyV0 {
        id,
        key_type: KeyType::ECDSA_HASH160,
        purpose,
        security_level: SecurityLevel::CRITICAL,
        read_only: true,
        data: BinaryData::new(hash.to_vec()),
        disabled_at: disabled.then_some(1),
        contract_bounds: None,
    }
    .into()
}

/// The owner identity after a payout change: the replaced TRANSFER key is
/// disabled, the current one enabled.
fn owner_identity() -> Identity {
    IdentityV0 {
        id: owner_identity_id(),
        public_keys: [
            key(0, Purpose::TRANSFER, STALE, true),
            key(1, Purpose::OWNER, OWNER, false),
            key(2, Purpose::TRANSFER, CURRENT, false),
        ]
        .into_iter()
        .map(|key| (key.id(), key))
        .collect(),
        balance: 1_000_000_000,
        revision: 0,
    }
    .into()
}

/// Holds the key whose hash is `hash`; records what it is asked to sign and
/// refuses, so nothing is ever broadcast.
#[derive(Debug)]
struct RecordingSigner {
    hash: [u8; 20],
    signed: Mutex<Vec<Vec<u8>>>,
}

impl RecordingSigner {
    fn holding(hash: [u8; 20]) -> Self {
        Self {
            hash,
            signed: Mutex::new(Vec::new()),
        }
    }

    fn signed(&self) -> Vec<Vec<u8>> {
        self.signed.lock().unwrap().clone()
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for RecordingSigner {
    async fn sign(
        &self,
        _key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.signed.lock().unwrap().push(data.to_vec());
        Err(ProtocolError::Generic(
            "test signer records and refuses".to_string(),
        ))
    }

    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::Generic("no witnesses".to_string()))
    }

    fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
        key.data().as_slice() == self.hash.as_slice()
    }
}

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

async fn manager_with_owner_identity() -> PlatformWalletManager<NoPlatformPersistence> {
    let mut sdk = SdkBuilder::new_mock().build().expect("mock sdk");
    sdk.mock()
        .expect_fetch(owner_identity_id(), Some(owner_identity()))
        .await
        .expect("identity expectation")
        .expect_fetch(owner_identity_id(), Some(IdentityNonceFetcher(5)))
        .await
        .expect("nonce expectation");
    PlatformWalletManager::new(
        Arc::new(sdk),
        Arc::new(NoPlatformPersistence),
        Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>,
    )
}

fn payout_key_request() -> MasternodeWithdrawalRequest {
    MasternodeWithdrawalRequest {
        pro_tx_hash: wire_pro_tx_hash(),
        owner_key_hash: None,
        amount_credits: 100_000_000,
        signing_key: MasternodeWithdrawalKey::Transfer,
        destination: None,
    }
}

/// The P2PKH output script paying `hash`.
fn p2pkh_script(hash: &[u8; 20]) -> Vec<u8> {
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(hash);
    script.extend_from_slice(&[0x88, 0xac]);
    script
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// A host payout-key withdrawal without a destination pays the P2PKH address
/// of the enabled TRANSFER key the host signs for, never the disabled one
/// it replaced.
#[tokio::test]
async fn should_pay_a_host_payout_key_withdrawal_to_the_enabled_transfer_key_address() {
    let manager = manager_with_owner_identity().await;
    let signer = RecordingSigner::holding(CURRENT);

    let result = manager
        .masternode_withdraw_with(payout_key_request(), MasternodeKeySource::host(&signer))
        .await;

    assert!(
        matches!(
            result,
            Err(MasternodeWithdrawalError::Wallet(
                PlatformWalletError::InvalidIdentityData(_)
            ))
        ),
        "the refused signature fails the withdrawal before broadcasting: {result:?}"
    );
    let signed = signer.signed();
    assert_eq!(signed.len(), 1, "exactly one transition was signed");
    assert!(
        contains(&signed[0], &p2pkh_script(&CURRENT)),
        "the signed withdrawal pays the enabled TRANSFER key's address"
    );
    assert!(!contains(&signed[0], &p2pkh_script(&STALE)));
}

/// A host key that is only the identity's disabled payout key is refused
/// before anything is signed, as a key mismatch for the TRANSFER purpose.
#[tokio::test]
async fn should_refuse_a_host_key_the_identity_has_disabled_before_signing() {
    let manager = manager_with_owner_identity().await;
    let signer = RecordingSigner::holding(STALE);

    let result = manager
        .masternode_withdraw_with(payout_key_request(), MasternodeKeySource::host(&signer))
        .await;

    match result {
        Err(MasternodeWithdrawalError::Wallet(
            PlatformWalletError::MasternodeWithdrawalKeyMismatch {
                identity_id,
                purpose,
            },
        )) => {
            assert_eq!(identity_id, owner_identity_id());
            assert_eq!(purpose, Purpose::TRANSFER);
        }
        other => panic!("expected MasternodeWithdrawalKeyMismatch, got {other:?}"),
    }
    assert!(signer.signed().is_empty(), "nothing was signed");
}

/// An owner-key withdrawal from a host pays the registered payout address:
/// the signed transition carries no output script of the payout key.
#[tokio::test]
async fn should_sign_a_host_owner_key_withdrawal_without_an_output_script() {
    let manager = manager_with_owner_identity().await;
    let signer = RecordingSigner::holding(OWNER);
    let request = MasternodeWithdrawalRequest {
        signing_key: MasternodeWithdrawalKey::Owner,
        ..payout_key_request()
    };

    let result = manager
        .masternode_withdraw_with(request, MasternodeKeySource::host(&signer))
        .await;

    assert!(
        matches!(
            result,
            Err(MasternodeWithdrawalError::Wallet(
                PlatformWalletError::InvalidIdentityData(_)
            ))
        ),
        "the refused signature fails the withdrawal before broadcasting: {result:?}"
    );
    let signed = signer.signed();
    assert_eq!(signed.len(), 1, "exactly one transition was signed");
    assert!(!contains(&signed[0], &p2pkh_script(&CURRENT)));
    assert!(!contains(&signed[0], &p2pkh_script(&OWNER)));
}
