//! The keyset entry points of the hooked shielded spends.
//!
//! A caller that holds only an account's `OrchardKeySet` (never the seed)
//! spends through [`PlatformWallet::shielded_transfer_to_with_keyset`],
//! [`PlatformWallet::shielded_withdraw_to_with_keyset`],
//! [`PlatformWallet::shielded_unshield_to_with_keyset`],
//! [`PlatformWallet::shielded_identity_top_up_from_pool_with_keyset`] and
//! [`PlatformWallet::shielded_identity_create_from_pool_with_keyset`]. Each
//! drives the real path: a bound wallet, a spendable note in a commitment
//! tree, the recorded anchor from the (mock) SDK, a real Halo 2 proof, the
//! coordinator's dispatch hook, then the broadcast.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dash_sdk::platform::shielded::try_decrypt_note_with_memo;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::state_transition::identity_create_from_shielded_pool_transition::{
    derive_identity_id_from_actions, IdentityCreateFromShieldedPoolTransition,
};
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::StateTransition;
use dpp::withdrawal::Pooling;
use dpp::ProtocolError;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::FutureExt;
use grovedb_commitment_tree::{ExtractedNoteCommitment, Note, NoteValue, RandomSeed, Rho, Scope};

use super::dispatch::{
    DispatchRefused, DispatchedNote, ShieldedDispatch, ShieldedDispatchKind,
    ShieldedDispatchPayload,
};
use super::keys::OrchardKeySet;
use super::store::ShieldedStore;
use super::viewing_key_bind_tests::{coordinator_at, platform_wallet_on, CapturingPersistence};
use super::{CachedOrchardProver, NetworkShieldedCoordinator, ShieldedNote, SubwalletId};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};

const SEED: [u8; 64] = [0x42; 64];
const OTHER_SEED: [u8; 64] = [0x43; 64];
const ACCOUNT: u32 = 0;
const NOTE_VALUE: u64 = 1_000_000_000_000;
const AMOUNT: u64 = 100_000_000;
/// A text memo (kind 1, little-endian, then a zero-padded UTF-8 payload).
/// It is non-zero, so a path that drops or zeroes the caller's memo is told
/// apart from one that carries it.
const MEMO: [u8; 36] = {
    let text = b"keyset spend memo";
    let mut memo = [0u8; 36];
    memo[0] = 1;
    let mut i = 0;
    while i < text.len() {
        memo[4 + i] = text[i];
        i += 1;
    }
    memo
};
const CORE_FEE_PER_BYTE: u32 = 1;
/// The identity a top-up credits. It need not exist for the spend to be
/// built and handed to the hook.
const TOP_UP_IDENTITY: [u8; 32] = [0x7A; 32];
/// 0.1 DASH in credits, a member of the latest exit-denomination set.
const DENOMINATION: u64 = 10_000_000_000;
/// Not 0, so a registration at the default slot is told apart.
const IDENTITY_INDEX: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Transfer,
    Withdraw,
    Unshield,
    TopUp,
    Create,
}

const KINDS: [Kind; 5] = [
    Kind::Transfer,
    Kind::Withdraw,
    Kind::Unshield,
    Kind::TopUp,
    Kind::Create,
];

/// A proof-of-possession signer with a fixed signature: the builder fills
/// the signatures without verifying them.
#[derive(Debug)]
struct FixedSigner;

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for FixedSigner {
    async fn sign(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::new(vec![0x5A; 65]))
    }

    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::ShieldedBuildError(
            "no address witnesses in these tests".to_string(),
        ))
    }

    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

/// The new identity's one master key, in both forms the create takes.
fn identity_keys() -> Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)> {
    let public = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![0x02; 33]),
        disabled_at: None,
    });
    let in_creation = IdentityPublicKeyInCreation::from(&public);
    vec![(public, in_creation)]
}

fn platform_recipient() -> PlatformAddress {
    PlatformAddress::P2pkh([0x11; 20])
}

fn platform_recipient_bech32m() -> String {
    platform_recipient().to_bech32m_string(dash_sdk::Sdk::new_mock().network)
}

/// Where a failed identity creation credits the denomination.
fn creation_failure_address() -> PlatformAddress {
    PlatformAddress::P2pkh([0x22; 20])
}

/// What the hook saw at one `dispatch`.
#[derive(Clone, Debug)]
struct Seen {
    payload: ShieldedDispatchPayload,
    /// Whether the spend's recovery record, with the transition's bytes, was
    /// on disk while the hook ran.
    durable_record: bool,
}

struct TestHook {
    db_path: PathBuf,
    accept: bool,
    seen: Mutex<Vec<Seen>>,
    /// The store's directory, removed when the coordinator that holds the
    /// hook is dropped.
    _dir: tempfile::TempDir,
}

impl TestHook {
    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl ShieldedDispatch for TestHook {
    async fn dispatch(&self, payload: ShieldedDispatchPayload) -> Result<(), DispatchRefused> {
        let id = SubwalletId::new(payload.wallet_id, payload.account_index);
        let durable_record =
            durable_record(&self.db_path, id, &payload.activity_id, &payload.st_bytes);
        self.seen.lock().unwrap().push(Seen {
            payload,
            durable_record,
        });
        if self.accept {
            Ok(())
        } else {
            Err(DispatchRefused::new("refused by the test hook"))
        }
    }

    async fn admit(
        &self,
        _wallet_id: WalletId,
        _account_index: u32,
        _activity_id: [u8; 32],
        _st_hash: [u8; 32],
    ) -> Result<(), DispatchRefused> {
        Err(DispatchRefused::new("no rebroadcast in these tests"))
    }
}

/// Whether `id`'s recovery record for `activity_id` is on disk holding
/// `st_bytes`, read through a connection of its own.
fn durable_record(path: &Path, id: SubwalletId, activity_id: &[u8; 32], st_bytes: &[u8]) -> bool {
    let conn = rusqlite::Connection::open(path).expect("raw connection");
    conn.query_row(
        "SELECT COUNT(*) FROM shielded_pending_spends \
         WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3 \
         AND st_bytes = ?4",
        rusqlite::params![
            id.wallet_id.as_slice(),
            id.account_index,
            activity_id.as_slice(),
            st_bytes
        ],
        |row| row.get::<_, i64>(0),
    )
    .expect("count the recovery record")
        == 1
}

fn pending_spend_rows(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("raw connection");
    conn.query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
        row.get(0)
    })
    .expect("count pending spends")
}

/// Counts every row a connection inserts, updates or deletes in the
/// database at `path` from [`Self::install`] on, through triggers on each of
/// the tables it has then.
struct WriteCounter {
    path: PathBuf,
}

impl WriteCounter {
    fn install(path: &Path) -> Self {
        let conn = rusqlite::Connection::open(path).expect("raw connection");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS test_write_count (n INTEGER NOT NULL); \
             DELETE FROM test_write_count; \
             INSERT INTO test_write_count (n) VALUES (0);",
        )
        .expect("write count table");
        let tables: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' \
                 AND name NOT LIKE 'sqlite_%' AND name != 'test_write_count'",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<_, _>>()
            })
            .expect("list tables");
        assert!(!tables.is_empty(), "the store's tables exist");
        for table in &tables {
            for op in ["INSERT", "UPDATE", "DELETE"] {
                conn.execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS \"test_write_count_{table}_{op}\" \
                     AFTER {op} ON \"{table}\" \
                     BEGIN UPDATE test_write_count SET n = n + 1; END;"
                ))
                .expect("write count trigger");
            }
        }
        Self {
            path: path.to_path_buf(),
        }
    }

    fn count(&self) -> i64 {
        let conn = rusqlite::Connection::open(&self.path).expect("raw connection");
        conn.query_row("SELECT n FROM test_write_count", [], |row| row.get(0))
            .expect("read the write count")
    }
}

/// Run `spend` while holding `wallet`'s shield lock and a shared guard on
/// `coordinator`'s store, counting writes to its database. Every change to
/// the store, the in-memory note reservation included, needs its write
/// lock, so a call that would change it can't finish on its first poll; it
/// must finish there, with no row written. A call that takes the shield
/// lock before its refusal can't finish there either. The poll runs outside
/// the task's cooperative budget, so only a lock that is held makes it wait.
async fn without_writes<F>(
    wallet: &PlatformWallet,
    coordinator: &NetworkShieldedCoordinator,
    spend: F,
    context: &str,
) -> Result<(), PlatformWalletError>
where
    F: Future<Output = Result<(), PlatformWalletError>>,
{
    without_writes_holding(wallet, coordinator, spend, context, true).await
}

/// [`without_writes`], holding `wallet`'s shield lock only if `shield`.
async fn without_writes_holding<F>(
    wallet: &PlatformWallet,
    coordinator: &NetworkShieldedCoordinator,
    spend: F,
    context: &str,
    shield: bool,
) -> Result<(), PlatformWalletError>
where
    F: Future<Output = Result<(), PlatformWalletError>>,
{
    let writes = WriteCounter::install(coordinator.db_path());
    let result = {
        let _shield = if shield {
            Some(wallet.shield_guard.lock().await)
        } else {
            None
        };
        let _shared = coordinator.store().read().await;
        tokio::task::unconstrained(spend).now_or_never()
    };
    let result = result.unwrap_or_else(|| {
        panic!("{context}: the call waited for the shield lock or the store's write lock")
    });
    assert_eq!(
        writes.count(),
        0,
        "{context}: no row written to the store's database during the call"
    );
    result
}

fn keyset(seed: &[u8], account: u32) -> OrchardKeySet {
    OrchardKeySet::from_seed(seed, dash_sdk::Sdk::new_mock().network, account)
        .expect("ZIP-32 derivation from a fixed seed")
}

/// One real, spendable Orchard note of `keys`' default address, as the
/// wallet stores it after a scan.
fn owned_note(keys: &OrchardKeySet) -> ShieldedNote {
    owned_note_at(keys, NOTE_VALUE, 0)
}

/// [`owned_note`] worth `value` at tree `position`, which also tells its
/// rho apart from the other notes'.
fn owned_note_at(keys: &OrchardKeySet, value: u64, position: u64) -> ShieldedNote {
    let rho = (1u16..=u16::MAX)
        .find_map(|n| {
            let mut b = [0u8; 32];
            b[0..2].copy_from_slice(&n.to_le_bytes());
            b[4] = u8::try_from(position).expect("a small position");
            Rho::from_bytes(&b).into_option()
        })
        .expect("a canonical rho exists");
    let rseed = (1u16..=u16::MAX)
        .find_map(|m| {
            let mut b = [0u8; 32];
            b[2..4].copy_from_slice(&m.to_le_bytes());
            RandomSeed::from_bytes(b, &rho).into_option()
        })
        .expect("a canonical rseed exists");
    let note = Note::from_parts(keys.default_address, NoteValue::from_raw(value), rho, rseed)
        .into_option()
        .expect("valid note parts");
    let mut note_data = Vec::with_capacity(115);
    note_data.extend_from_slice(&note.recipient().to_raw_address_bytes());
    note_data.extend_from_slice(&note.value().inner().to_le_bytes());
    note_data.extend_from_slice(&note.rho().to_bytes());
    note_data.extend_from_slice(note.rseed().as_bytes());
    ShieldedNote {
        position,
        cmx: ExtractedNoteCommitment::from(note.commitment()).to_bytes(),
        nullifier: note.nullifier(&keys.full_viewing_key).to_bytes(),
        block_height: 1,
        is_spent: false,
        value,
        note_data,
    }
}

/// A coordinator with `hook` set whose tree holds `note` at a block
/// boundary. Returns the tree's anchor.
async fn hooked_coordinator(
    tag: &str,
    note: &ShieldedNote,
    hook_accepts: bool,
) -> (Arc<NetworkShieldedCoordinator>, Arc<TestHook>, [u8; 32]) {
    hooked_coordinator_holding(tag, std::slice::from_ref(note), hook_accepts).await
}

/// [`hooked_coordinator`] whose tree holds `notes`, in position order.
async fn hooked_coordinator_holding(
    tag: &str,
    notes: &[ShieldedNote],
    hook_accepts: bool,
) -> (Arc<NetworkShieldedCoordinator>, Arc<TestHook>, [u8; 32]) {
    let dir = tempfile::Builder::new()
        .prefix(&format!("keyset_spend_{tag}_"))
        .tempdir()
        .expect("temporary shielded store");
    let coordinator = coordinator_at(dir.path());
    let anchor = {
        let mut store = coordinator.store().write().await;
        for note in notes {
            store.append_commitment(&note.cmx, true).unwrap();
        }
        store.checkpoint_tree(1).unwrap();
        store.tree_anchor().unwrap()
    };
    let hook = Arc::new(TestHook {
        db_path: coordinator.db_path().to_path_buf(),
        accept: hook_accepts,
        seen: Mutex::new(Vec::new()),
        _dir: dir,
    });
    coordinator
        .set_shielded_dispatch(Arc::clone(&hook) as Arc<dyn ShieldedDispatch>)
        .expect("set the hook");
    (coordinator, hook, anchor)
}

/// A wallet bound to `SEED`'s `ACCOUNT` on `coordinator`, whose SDK reports
/// `anchor` as recorded, holding `note` in each of `stores`.
async fn bound_wallet(
    coordinator: &Arc<NetworkShieldedCoordinator>,
    anchor: [u8; 32],
    note: &ShieldedNote,
    stores: &[&Arc<NetworkShieldedCoordinator>],
) -> (PlatformWallet, Arc<CapturingPersistence>) {
    bound_wallet_holding(
        coordinator,
        anchor,
        std::slice::from_ref(note),
        stores,
        None,
    )
    .await
}

/// [`bound_wallet`] holding `notes`, whose SDK also serves `identity` when
/// it is fetched by its id.
async fn bound_wallet_holding(
    coordinator: &Arc<NetworkShieldedCoordinator>,
    anchor: [u8; 32],
    notes: &[ShieldedNote],
    stores: &[&Arc<NetworkShieldedCoordinator>],
    identity: Option<dpp::identity::Identity>,
) -> (PlatformWallet, Arc<CapturingPersistence>) {
    use dash_sdk::query_types::{NoParamQuery, ShieldedAnchors};
    use dpp::identity::accessors::IdentityGettersV0;
    // Pinned, so the anchor response can't move the protocol version between
    // the fee the spend reserves and the fee its builder applies.
    let mut sdk = dash_sdk::SdkBuilder::new_mock()
        .with_version(dpp::version::PlatformVersion::latest())
        .build()
        .expect("mock sdk");
    sdk.mock()
        .expect_fetch(NoParamQuery {}, Some(ShieldedAnchors(vec![anchor])))
        .await
        .expect("anchor expectation");
    if let Some(identity) = identity {
        sdk.mock()
            .expect_fetch(identity.id(), Some(identity))
            .await
            .expect("identity expectation");
    }
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_on(Arc::clone(&persister), Arc::new(sdk)).await;
    wallet
        .bind_shielded(&SEED, &[ACCOUNT], coordinator)
        .await
        .expect("seed bind");
    let id = SubwalletId::new(wallet.wallet_id(), ACCOUNT);
    for holder in stores {
        let mut store = holder.store().write().await;
        for note in notes {
            store.save_note(id, note).expect("save the note");
        }
    }
    (wallet, persister)
}

fn recipient() -> [u8; 43] {
    keyset(&OTHER_SEED, 0)
        .default_address
        .to_raw_address_bytes()
}

fn core_address() -> String {
    let key: dashcore::PublicKey =
        "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
            .parse()
            .expect("a valid public key");
    dashcore::Address::p2pkh(&key, dash_sdk::Sdk::new_mock().network).to_string()
}

async fn spend_with_keyset(
    wallet: &PlatformWallet,
    coordinator: &Arc<NetworkShieldedCoordinator>,
    keys: &OrchardKeySet,
    account: u32,
    kind: Kind,
) -> Result<(), PlatformWalletError> {
    let prover = CachedOrchardProver::new();
    match kind {
        Kind::Transfer => {
            wallet
                .shielded_transfer_to_with_keyset(
                    coordinator,
                    keys,
                    account,
                    &recipient(),
                    AMOUNT,
                    MEMO,
                    &prover,
                )
                .await
        }
        Kind::Withdraw => {
            wallet
                .shielded_withdraw_to_with_keyset(
                    coordinator,
                    keys,
                    account,
                    &core_address(),
                    AMOUNT,
                    CORE_FEE_PER_BYTE,
                    &prover,
                )
                .await
        }
        Kind::Unshield => {
            wallet
                .shielded_unshield_to_with_keyset(
                    coordinator,
                    keys,
                    account,
                    &platform_recipient_bech32m(),
                    AMOUNT,
                    &prover,
                )
                .await
        }
        Kind::TopUp => wallet
            .shielded_identity_top_up_from_pool_with_keyset(
                coordinator,
                keys,
                account,
                &Identifier::new(TOP_UP_IDENTITY),
                AMOUNT,
                &prover,
            )
            .await
            .map(drop),
        Kind::Create => wallet
            .shielded_identity_create_from_pool_with_keyset(
                coordinator,
                keys,
                account,
                IDENTITY_INDEX,
                identity_keys(),
                DENOMINATION,
                creation_failure_address(),
                &FixedSigner,
                &prover,
            )
            .await
            .map(drop),
    }
}

async fn spend_with_seed(
    wallet: &PlatformWallet,
    coordinator: &Arc<NetworkShieldedCoordinator>,
    seed: &[u8],
    kind: Kind,
) -> Result<(), PlatformWalletError> {
    let prover = CachedOrchardProver::new();
    match kind {
        Kind::Transfer => {
            wallet
                .shielded_transfer_to(
                    coordinator,
                    seed,
                    ACCOUNT,
                    &recipient(),
                    AMOUNT,
                    MEMO,
                    &prover,
                )
                .await
        }
        Kind::Withdraw => {
            wallet
                .shielded_withdraw_to(
                    coordinator,
                    seed,
                    ACCOUNT,
                    &core_address(),
                    AMOUNT,
                    CORE_FEE_PER_BYTE,
                    &prover,
                )
                .await
        }
        Kind::Unshield => {
            wallet
                .shielded_unshield_to(
                    coordinator,
                    seed,
                    ACCOUNT,
                    &platform_recipient_bech32m(),
                    AMOUNT,
                    &prover,
                )
                .await
        }
        Kind::TopUp => wallet
            .shielded_identity_top_up_from_pool(
                coordinator,
                seed,
                ACCOUNT,
                &Identifier::new(TOP_UP_IDENTITY),
                AMOUNT,
                &prover,
            )
            .await
            .map(drop),
        Kind::Create => wallet
            .shielded_identity_create_from_pool(
                coordinator,
                seed,
                ACCOUNT,
                IDENTITY_INDEX,
                identity_keys(),
                DENOMINATION,
                creation_failure_address(),
                &FixedSigner,
                &prover,
            )
            .await
            .map(drop),
    }
}

/// Nothing of a spend reached the store, the persister or the hook.
async fn assert_nothing_written(
    wallet: &PlatformWallet,
    coordinator: &Arc<NetworkShieldedCoordinator>,
    hook: &TestHook,
    persister: &CapturingPersistence,
    stored_before: usize,
    context: &str,
) {
    let id = SubwalletId::new(wallet.wallet_id(), ACCOUNT);
    {
        let store = coordinator.store().read().await;
        assert!(
            store.pending_redrives(id).unwrap().is_empty(),
            "{context}: no redrive row"
        );
        assert_eq!(
            store.spendable_balance(id).unwrap(),
            NOTE_VALUE,
            "{context}: no note reserved"
        );
    }
    assert_eq!(
        pending_spend_rows(coordinator.db_path()),
        0,
        "{context}: no pending spend row on disk"
    );
    assert!(hook.seen().is_empty(), "{context}: dispatch never called");
    assert_eq!(
        persister.stored_count(),
        stored_before,
        "{context}: no activity handed to the persister"
    );
}

/// Keysets that aren't `SEED`'s `ACCOUNT` spend authority: another seed's or
/// another account's, whole, and the account's own with one field replaced
/// by one its full viewing key doesn't determine.
fn mismatched_keysets() -> Vec<(&'static str, OrchardKeySet)> {
    let own = keyset(&SEED, ACCOUNT);
    let other = keyset(&OTHER_SEED, ACCOUNT);

    let mut foreign_spend_key = keyset(&OTHER_SEED, ACCOUNT);
    foreign_spend_key.full_viewing_key = own.full_viewing_key.clone();
    foreign_spend_key.incoming_viewing_key = own.incoming_viewing_key.clone();
    foreign_spend_key.outgoing_viewing_key = own.outgoing_viewing_key.clone();
    foreign_spend_key.default_address = own.default_address;

    let mut foreign_address = keyset(&SEED, ACCOUNT);
    foreign_address.default_address = other.default_address;

    let mut second_address = keyset(&SEED, ACCOUNT);
    second_address.default_address = second_address.address_at(1);

    let mut foreign_ivk = keyset(&SEED, ACCOUNT);
    foreign_ivk.incoming_viewing_key = other.incoming_viewing_key.clone();

    let mut foreign_ovk = keyset(&SEED, ACCOUNT);
    foreign_ovk.outgoing_viewing_key = other.outgoing_viewing_key.clone();

    let mut internal_ivk = keyset(&SEED, ACCOUNT);
    internal_ivk.incoming_viewing_key = own.full_viewing_key.to_ivk(Scope::Internal);

    let mut internal_ovk = keyset(&SEED, ACCOUNT);
    internal_ovk.outgoing_viewing_key = own.full_viewing_key.to_ovk(Scope::Internal);

    let mut internal_address = keyset(&SEED, ACCOUNT);
    internal_address.default_address = own.full_viewing_key.address_at(0u32, Scope::Internal);

    vec![
        ("another seed", keyset(&OTHER_SEED, ACCOUNT)),
        ("another account", keyset(&SEED, ACCOUNT + 1)),
        (
            "the bound FVK over another seed's spend key",
            foreign_spend_key,
        ),
        ("another seed's default address", foreign_address),
        ("the bound account's address at index 1", second_address),
        ("another seed's IVK", foreign_ivk),
        ("another seed's OVK", foreign_ovk),
        ("the bound FVK's internal-scope IVK", internal_ivk),
        ("the bound FVK's internal-scope OVK", internal_ovk),
        (
            "the bound FVK's internal-scope address at index 0",
            internal_address,
        ),
    ]
}

/// A keyset that isn't the account's spend authority, whole or in any one
/// field the spend reads, is refused with the typed mismatch on every kind,
/// before anything is reserved, recorded, stored or handed to the hook.
#[tokio::test]
async fn a_mismatched_keyset_is_refused_and_writes_nothing() {
    for kind in KINDS {
        for (what, wrong) in mismatched_keysets() {
            let own = keyset(&SEED, ACCOUNT);
            let note = owned_note(&own);
            let (coordinator, hook, anchor) =
                hooked_coordinator(&format!("mismatch_{kind:?}"), &note, true).await;
            let (wallet, persister) =
                bound_wallet(&coordinator, anchor, &note, &[&coordinator]).await;
            let stored_before = persister.stored_count();

            let result = without_writes(
                &wallet,
                &coordinator,
                spend_with_keyset(&wallet, &coordinator, &wrong, ACCOUNT, kind),
                &format!("{kind:?} {what}"),
            )
            .await;

            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::ShieldedKeysetMismatch { account: ACCOUNT })
                ),
                "{kind:?} {what}: refused as a keyset mismatch, got {result:?}"
            );
            assert_nothing_written(
                &wallet,
                &coordinator,
                &hook,
                &persister,
                stored_before,
                &format!("{kind:?} {what}"),
            )
            .await;
        }
    }
}

/// A top-up waiting for the shield lock behind a bind that re-binds its
/// account away is refused as an unbound account once it holds the lock,
/// on both entry points, with nothing written.
#[tokio::test]
async fn a_top_up_rechecks_the_keyset_once_it_holds_the_shield_lock() {
    for by_seed in [false, true] {
        let context = if by_seed { "seed" } else { "keyset" };
        let own = keyset(&SEED, ACCOUNT);
        let note = owned_note(&own);
        let (coordinator, hook, anchor) =
            hooked_coordinator(&format!("top_up_rebind_{context}"), &note, true).await;
        let (wallet, _persister) = bound_wallet(&coordinator, anchor, &note, &[&coordinator]).await;

        let shield = wallet.shield_guard.lock().await;
        let mut rebind = std::pin::pin!(tokio::task::unconstrained(wallet.bind_shielded(
            &SEED,
            &[ACCOUNT + 1],
            &coordinator
        )));
        assert!(
            rebind.as_mut().now_or_never().is_none(),
            "{context}: the re-bind waits for the shield lock"
        );
        let top_up = async {
            if by_seed {
                spend_with_seed(&wallet, &coordinator, &SEED, Kind::TopUp).await
            } else {
                spend_with_keyset(&wallet, &coordinator, &own, ACCOUNT, Kind::TopUp).await
            }
        };
        let mut top_up = std::pin::pin!(tokio::task::unconstrained(top_up));
        assert!(
            top_up.as_mut().now_or_never().is_none(),
            "{context}: the top-up passes its first check and waits for the shield lock"
        );
        drop(shield);
        rebind.await.expect("the re-bind");

        let result = top_up.await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ShieldedKeyDerivation(ref message))
                    if message == "shielded account 0 not bound"
            ),
            "{context}: refused as unbound once the shield lock is held, got {result:?}"
        );
        assert!(hook.seen().is_empty(), "{context}: dispatch never called");
        assert_eq!(
            pending_spend_rows(coordinator.db_path()),
            0,
            "{context}: no pending spend row on disk"
        );
    }
}

/// The keyset check runs before the request is parsed: a mismatched
/// keyset with an unparsable recipient, or an identity create with no key,
/// is still refused as a mismatch.
#[tokio::test]
async fn a_mismatched_keyset_is_refused_before_the_recipient_is_parsed() {
    let own = keyset(&SEED, ACCOUNT);
    let note = owned_note(&own);
    let (coordinator, hook, anchor) = hooked_coordinator("mismatch_parse", &note, true).await;
    let (wallet, persister) = bound_wallet(&coordinator, anchor, &note, &[&coordinator]).await;
    let stored_before = persister.stored_count();
    let other = keyset(&OTHER_SEED, ACCOUNT);
    let prover = CachedOrchardProver::new();

    let transfer = without_writes(
        &wallet,
        &coordinator,
        wallet.shielded_transfer_to_with_keyset(
            &coordinator,
            &other,
            ACCOUNT,
            &[0xFF; 43],
            AMOUNT,
            MEMO,
            &prover,
        ),
        "unparsable recipient, transfer",
    )
    .await;
    let withdraw = without_writes(
        &wallet,
        &coordinator,
        wallet.shielded_withdraw_to_with_keyset(
            &coordinator,
            &other,
            ACCOUNT,
            "not a core address",
            AMOUNT,
            CORE_FEE_PER_BYTE,
            &prover,
        ),
        "unparsable recipient, withdraw",
    )
    .await;
    let unshield = without_writes(
        &wallet,
        &coordinator,
        wallet.shielded_unshield_to_with_keyset(
            &coordinator,
            &other,
            ACCOUNT,
            "not a platform address",
            AMOUNT,
            &prover,
        ),
        "unparsable recipient, unshield",
    )
    .await;
    let create = without_writes(
        &wallet,
        &coordinator,
        wallet
            .shielded_identity_create_from_pool_with_keyset(
                &coordinator,
                &other,
                ACCOUNT,
                IDENTITY_INDEX,
                Vec::new(),
                DENOMINATION,
                creation_failure_address(),
                &FixedSigner,
                &prover,
            )
            .map(|result| result.map(drop)),
        "no identity key, create",
    )
    .await;

    for result in [transfer, withdraw, unshield, create] {
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ShieldedKeysetMismatch { account: ACCOUNT })
            ),
            "refused as a keyset mismatch, got {result:?}"
        );
    }
    assert_nothing_written(
        &wallet,
        &coordinator,
        &hook,
        &persister,
        stored_before,
        "unparsable recipient",
    )
    .await;

    // Control: with the matching keyset, the same requests fail to parse.
    let transfer = wallet
        .shielded_transfer_to_with_keyset(
            &coordinator,
            &own,
            ACCOUNT,
            &[0xFF; 43],
            AMOUNT,
            MEMO,
            &prover,
        )
        .await;
    let withdraw = wallet
        .shielded_withdraw_to_with_keyset(
            &coordinator,
            &own,
            ACCOUNT,
            "not a core address",
            AMOUNT,
            CORE_FEE_PER_BYTE,
            &prover,
        )
        .await;
    let unshield = wallet
        .shielded_unshield_to_with_keyset(
            &coordinator,
            &own,
            ACCOUNT,
            "not a platform address",
            AMOUNT,
            &prover,
        )
        .await;
    let create = wallet
        .shielded_identity_create_from_pool_with_keyset(
            &coordinator,
            &own,
            ACCOUNT,
            IDENTITY_INDEX,
            Vec::new(),
            DENOMINATION,
            creation_failure_address(),
            &FixedSigner,
            &prover,
        )
        .await
        .map(drop);
    for result in [transfer, withdraw, unshield, create] {
        assert!(
            matches!(result, Err(PlatformWalletError::ShieldedBuildError(_))),
            "an unparsable request fails to parse, got {result:?}"
        );
    }
}

/// A wrong seed on the seed wrapper keeps its own error, not the keyset
/// path's mismatch, and writes nothing. A seed top-up derives its keyset
/// once it holds the shield lock, as before the keyset path existed, so its
/// case runs without that lock held.
#[tokio::test]
async fn a_wrong_seed_keeps_its_error_on_the_seed_path() {
    for kind in KINDS {
        let own = keyset(&SEED, ACCOUNT);
        let note = owned_note(&own);
        let (coordinator, hook, anchor) =
            hooked_coordinator(&format!("wrong_seed_{kind:?}"), &note, true).await;
        let (wallet, persister) = bound_wallet(&coordinator, anchor, &note, &[&coordinator]).await;
        let stored_before = persister.stored_count();

        let context = format!("{kind:?} wrong seed");
        let result = without_writes_holding(
            &wallet,
            &coordinator,
            spend_with_seed(&wallet, &coordinator, &OTHER_SEED, kind),
            &context,
            kind != Kind::TopUp,
        )
        .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ShieldedKeyDerivation(ref message))
                    if message == "seed does not derive the bound viewing key for shielded account 0"
            ),
            "{kind:?}: the seed path's own error, got {result:?}"
        );
        assert_nothing_written(
            &wallet,
            &coordinator,
            &hook,
            &persister,
            stored_before,
            &context,
        )
        .await;
    }
}

/// An unbound account, or a wallet with no shielded sub-wallet, is refused
/// on the keyset path with the seed path's errors.
#[tokio::test]
async fn an_unbound_account_is_refused_on_the_keyset_path() {
    for kind in KINDS {
        let own = keyset(&SEED, ACCOUNT);
        let note = owned_note(&own);
        let (coordinator, hook, anchor) =
            hooked_coordinator(&format!("unbound_{kind:?}"), &note, true).await;
        let (wallet, persister) = bound_wallet(&coordinator, anchor, &note, &[&coordinator]).await;
        let stored_before = persister.stored_count();

        let result = without_writes(
            &wallet,
            &coordinator,
            spend_with_keyset(&wallet, &coordinator, &keyset(&SEED, 5), 5, kind),
            &format!("{kind:?} unbound account"),
        )
        .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ShieldedKeyDerivation(ref message))
                    if message == "shielded account 5 not bound"
            ),
            "{kind:?}: unbound account refused, got {result:?}"
        );
        assert_nothing_written(
            &wallet,
            &coordinator,
            &hook,
            &persister,
            stored_before,
            &format!("{kind:?} unbound account"),
        )
        .await;

        let unbound_persister = Arc::new(CapturingPersistence::default());
        let unbound = platform_wallet_on(
            Arc::clone(&unbound_persister),
            Arc::new(dash_sdk::Sdk::new_mock()),
        )
        .await;
        let unbound_stored_before = unbound_persister.stored_count();
        let result = without_writes(
            &unbound,
            &coordinator,
            spend_with_keyset(&unbound, &coordinator, &own, ACCOUNT, kind),
            &format!("{kind:?} unbound wallet"),
        )
        .await;
        assert!(
            matches!(result, Err(PlatformWalletError::ShieldedNotBound)),
            "{kind:?}: unbound wallet refused, got {result:?}"
        );
        assert_nothing_written(
            &wallet,
            &coordinator,
            &hook,
            &unbound_persister,
            unbound_stored_before,
            &format!("{kind:?} unbound wallet, the bound wallet's note untouched"),
        )
        .await;
        assert_eq!(
            persister.stored_count(),
            stored_before,
            "{kind:?} unbound wallet: nothing handed to the bound wallet's persister"
        );
    }
}

/// `seen`'s artifact is `kind`'s spend of exactly `note` by `wallet`'s
/// `ACCOUNT` against `anchor`, recorded and not yet broadcast.
fn assert_artifact(
    seen: &Seen,
    kind: Kind,
    wallet: &PlatformWallet,
    note: &ShieldedNote,
    anchor: [u8; 32],
) {
    let payload = &seen.payload;
    let expected_kind = match kind {
        Kind::Transfer => ShieldedDispatchKind::Transfer,
        Kind::Withdraw => ShieldedDispatchKind::Withdraw,
        Kind::Unshield => ShieldedDispatchKind::Unshield,
        Kind::TopUp => ShieldedDispatchKind::IdentityTopUp {
            identity_id: TOP_UP_IDENTITY,
        },
        Kind::Create => ShieldedDispatchKind::IdentityCreate {
            identity_id: created_identity_id(&payload.st_bytes),
        },
    };
    assert_eq!(payload.kind, expected_kind, "{kind:?}: kind");
    assert_eq!(payload.wallet_id, wallet.wallet_id(), "{kind:?}: wallet");
    assert_eq!(payload.account_index, ACCOUNT, "{kind:?}: account");
    assert_eq!(payload.anchor, anchor, "{kind:?}: anchor");
    assert_eq!(
        payload.selected_notes,
        vec![DispatchedNote {
            nullifier: note.nullifier,
            cmx: note.cmx,
            value: note.value,
            position: note.position,
        }],
        "{kind:?}: selected notes"
    );
    assert!(
        payload.published_nullifiers.contains(&note.nullifier),
        "{kind:?}: the note's nullifier is published"
    );
    assert_eq!(
        payload.st_hash,
        dpp::util::hash::hash_single(&payload.st_bytes),
        "{kind:?}: st_hash keys the bytes"
    );
    assert!(
        seen.durable_record,
        "{kind:?}: the recovery record is durable while the hook runs"
    );
}

/// A matching keyset reaches the hook on every kind: `dispatch` sees the
/// spend while its recovery record is durable, and only once it returns
/// `Ok` is the transition sent; the unconfirmed send keeps the record.
#[tokio::test]
async fn a_matching_keyset_reaches_the_hook_before_the_send() {
    for kind in KINDS {
        let own = keyset(&SEED, ACCOUNT);
        let note = owned_note(&own);
        let (coordinator, hook, anchor) =
            hooked_coordinator(&format!("match_{kind:?}"), &note, true).await;
        let (wallet, _persister) = bound_wallet(&coordinator, anchor, &note, &[&coordinator]).await;

        // Controls for `without_writes`: a spend that gets past the check
        // waits for the store's write lock, and its rows are counted.
        {
            let _shared = coordinator.store().read().await;
            assert!(
                tokio::task::unconstrained(spend_with_keyset(
                    &wallet,
                    &coordinator,
                    &own,
                    ACCOUNT,
                    kind
                ))
                .now_or_never()
                .is_none(),
                "{kind:?}: a matching keyset's spend waits for the store's write lock"
            );
        }
        let writes = WriteCounter::install(coordinator.db_path());

        let result = spend_with_keyset(&wallet, &coordinator, &own, ACCOUNT, kind).await;

        assert!(writes.count() > 0, "{kind:?}: the spend's rows are counted");
        let seen = hook.seen();
        assert_eq!(seen.len(), 1, "{kind:?}: one dispatch");
        assert_artifact(&seen[0], kind, &wallet, &note, anchor);
        let id = SubwalletId::new(wallet.wallet_id(), ACCOUNT);
        assert!(
            durable_record(
                coordinator.db_path(),
                id,
                &seen[0].payload.activity_id,
                &seen[0].payload.st_bytes
            ),
            "{kind:?}: the unconfirmed send keeps its recovery record"
        );
        // The mock SDK has no broadcast expectation, so the send itself is
        // what fails, and it fails as an unconfirmed send.
        let unconfirmed = match &result {
            Err(PlatformWalletError::ShieldedSpendUnconfirmed { reason, .. }) => {
                kind != Kind::Create && reason.contains("mock expectation not found")
            }
            Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
                identity_id,
                reason,
            }) => {
                kind == Kind::Create
                    && identity_id.to_buffer() == created_identity_id(&seen[0].payload.st_bytes)
                    && reason.contains("mock expectation not found")
            }
            _ => false,
        };
        assert!(
            unconfirmed,
            "{kind:?}: the transition was sent after dispatch, got {result:?}"
        );
    }
}

/// The deterministic fields of a transition of `kind`: everything but the
/// proof, the binding signature and the actions, whose order and contents
/// the builder randomizes.
fn deterministic_fields(kind: Kind, st_bytes: &[u8]) -> String {
    let st = StateTransition::deserialize_from_bytes_trusted(st_bytes).expect("decode transition");
    match (kind, st) {
        (Kind::Transfer, StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(v0))) => {
            format!(
                "transfer actions {} value_balance {} anchor {:?}",
                v0.actions.len(),
                v0.value_balance,
                v0.anchor,
            )
        }
        (
            Kind::Withdraw,
            StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(v0)),
        ) => format!(
            "withdraw actions {} amount {} anchor {:?} core_fee_per_byte {} pooling {:?} \
             output_script {:?}",
            v0.actions.len(),
            v0.unshielding_amount,
            v0.anchor,
            v0.core_fee_per_byte,
            v0.pooling,
            v0.output_script,
        ),
        (Kind::Unshield, StateTransition::Unshield(UnshieldTransition::V0(v0))) => format!(
            "unshield actions {} amount {} anchor {:?} output_address {:?}",
            v0.actions.len(),
            v0.unshielding_amount,
            v0.anchor,
            v0.output_address,
        ),
        (
            Kind::TopUp,
            StateTransition::IdentityTopUpFromShieldedPool(
                IdentityTopUpFromShieldedPoolTransition::V0(v0),
            ),
        ) => format!(
            "top up actions {} amount {} anchor {:?} identity {:?}",
            v0.actions.len(),
            v0.top_up_amount,
            v0.anchor,
            v0.identity_id,
        ),
        (
            Kind::Create,
            StateTransition::IdentityCreateFromShieldedPool(
                IdentityCreateFromShieldedPoolTransition::V0(v0),
            ),
        ) => format!(
            "create actions {} denomination {} anchor {:?} failure_address {:?} keys {:?}",
            v0.actions.len(),
            v0.denomination,
            v0.anchor,
            v0.send_to_address_on_creation_failure,
            v0.public_keys,
        ),
        (kind, st) => panic!("{kind:?}: unexpected transition {st:?}"),
    }
}

/// The id of the identity `st_bytes`, an identity create, creates: the one
/// it carries, which must be the one its published nullifiers derive.
fn created_identity_id(st_bytes: &[u8]) -> [u8; 32] {
    let st = StateTransition::deserialize_from_bytes_trusted(st_bytes).expect("decode transition");
    let StateTransition::IdentityCreateFromShieldedPool(
        IdentityCreateFromShieldedPoolTransition::V0(v0),
    ) = st
    else {
        panic!("not an identity create from the pool: {st:?}");
    };
    assert_eq!(
        v0.identity_id,
        derive_identity_id_from_actions(&v0.actions),
        "the created id is the one the published nullifiers derive"
    );
    v0.identity_id.to_buffer()
}

/// The actions of a transition of `kind`, and the credits it takes out of
/// the shielded pool: the fee of a transfer, the whole amount leaving the
/// pool otherwise.
fn actions_and_outflow(kind: Kind, st_bytes: &[u8]) -> (Vec<dpp::shielded::SerializedAction>, u64) {
    let st = StateTransition::deserialize_from_bytes_trusted(st_bytes).expect("decode transition");
    match (kind, st) {
        (Kind::Transfer, StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(v0))) => {
            (v0.actions, v0.value_balance)
        }
        (
            Kind::Withdraw,
            StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(v0)),
        ) => (v0.actions, v0.unshielding_amount),
        (Kind::Unshield, StateTransition::Unshield(UnshieldTransition::V0(v0))) => {
            (v0.actions, v0.unshielding_amount)
        }
        (
            Kind::TopUp,
            StateTransition::IdentityTopUpFromShieldedPool(
                IdentityTopUpFromShieldedPoolTransition::V0(v0),
            ),
        ) => (v0.actions, v0.top_up_amount),
        (
            Kind::Create,
            StateTransition::IdentityCreateFromShieldedPool(
                IdentityCreateFromShieldedPoolTransition::V0(v0),
            ),
        ) => (v0.actions, v0.denomination),
        (kind, st) => panic!("{kind:?}: unexpected transition {st:?}"),
    }
}

/// The output notes of `actions` that decrypt under `viewer`'s incoming
/// viewing key, as (value, raw recipient address, memo).
fn decrypted_outputs(
    actions: &[dpp::shielded::SerializedAction],
    viewer: &OrchardKeySet,
) -> Vec<(u64, [u8; 43], [u8; 36])> {
    let ivk = viewer.prepared_ivk();
    actions
        .iter()
        .filter_map(|action| {
            let wire = ShieldedEncryptedNote {
                cmx: action.cmx.to_vec(),
                nullifier: action.nullifier.to_vec(),
                cv_net: action.cv_net.to_vec(),
                encrypted_note: action.encrypted_note.clone(),
            };
            try_decrypt_note_with_memo(&ivk, &wire).map(|(note, address, memo)| {
                (note.value().inner(), address.to_raw_address_bytes(), memo)
            })
        })
        .collect()
}

/// `st_bytes` pays exactly what was asked: a transfer's one output to the
/// recipient carries `AMOUNT` and `MEMO`; a withdrawal, an unshield and a
/// top-up take exactly `AMOUNT` plus their fee for the action count out of
/// the pool, to the requested Core address, Platform address or identity;
/// an identity create takes exactly `DENOMINATION` out, with the requested
/// keys and failure address. The one output the bound account can decrypt
/// is the change, at its default address, worth the note less the payment
/// and everything taken out of the pool.
fn assert_outputs(kind: Kind, st_bytes: &[u8], path: &str) {
    let own = keyset(&SEED, ACCOUNT);
    let (actions, outflow) = actions_and_outflow(kind, st_bytes);
    let to_recipient = decrypted_outputs(&actions, &keyset(&OTHER_SEED, 0));
    let to_self = decrypted_outputs(&actions, &own);
    let (paid, taken) = match kind {
        Kind::Transfer => {
            assert!(outflow > 0, "{kind:?} {path}: a fee leaves the pool");
            assert_eq!(
                to_recipient,
                vec![(AMOUNT, recipient(), MEMO)],
                "{kind:?} {path}: the recipient's output"
            );
            (AMOUNT, outflow)
        }
        Kind::Withdraw => {
            assert!(
                to_recipient.is_empty(),
                "{kind:?} {path}: no shielded output to anyone else"
            );
            assert_withdrawal_request(st_bytes, path);
            let fee = dpp::shielded::compute_shielded_withdrawal_fee(
                actions.len(),
                dpp::version::PlatformVersion::latest(),
            )
            .expect("withdrawal fee");
            assert_eq!(
                outflow,
                AMOUNT + fee,
                "{kind:?} {path}: exactly the amount and the withdrawal fee leave the pool"
            );
            (0, AMOUNT + fee)
        }
        Kind::Unshield | Kind::TopUp | Kind::Create => {
            assert!(
                to_recipient.is_empty(),
                "{kind:?} {path}: no shielded output to anyone else"
            );
            let taken = assert_pool_exit_request(kind, st_bytes, actions.len(), path);
            assert_eq!(
                outflow, taken,
                "{kind:?} {path}: exactly the requested exit leaves the pool"
            );
            (0, taken)
        }
    };
    assert_eq!(to_self.len(), 1, "{kind:?} {path}: one change output");
    let (change, address, _memo) = to_self[0];
    assert_eq!(
        address,
        own.default_address.to_raw_address_bytes(),
        "{kind:?} {path}: change to the bound account's default address"
    );
    assert_eq!(
        change,
        NOTE_VALUE - paid - taken,
        "{kind:?} {path}: change is the rest of the note"
    );
}

/// `st_bytes`, an unshield, a top-up or an identity create, goes to the
/// requested Platform address, identity or key set. Returns what it must
/// take out of the pool: `AMOUNT` plus the fee for `action_count` actions,
/// or the requested denomination.
fn assert_pool_exit_request(kind: Kind, st_bytes: &[u8], action_count: usize, path: &str) -> u64 {
    let version = dpp::version::PlatformVersion::latest();
    let st = StateTransition::deserialize_from_bytes_trusted(st_bytes).expect("decode transition");
    match (kind, st) {
        (Kind::Unshield, StateTransition::Unshield(UnshieldTransition::V0(v0))) => {
            assert_eq!(
                v0.output_address,
                platform_recipient(),
                "Unshield {path}: pays the requested Platform address"
            );
            AMOUNT
                + dpp::shielded::compute_shielded_unshield_fee(action_count, version)
                    .expect("unshield fee")
        }
        (
            Kind::TopUp,
            StateTransition::IdentityTopUpFromShieldedPool(
                IdentityTopUpFromShieldedPoolTransition::V0(v0),
            ),
        ) => {
            assert_eq!(
                v0.identity_id,
                Identifier::new(TOP_UP_IDENTITY),
                "TopUp {path}: credits the requested identity"
            );
            AMOUNT
                + dpp::shielded::compute_shielded_identity_top_up_fee(action_count, version)
                    .expect("top-up fee")
        }
        (
            Kind::Create,
            StateTransition::IdentityCreateFromShieldedPool(
                IdentityCreateFromShieldedPoolTransition::V0(v0),
            ),
        ) => {
            assert_eq!(
                v0.send_to_address_on_creation_failure,
                creation_failure_address(),
                "Create {path}: the requested failure address"
            );
            let requested: Vec<IdentityPublicKeyInCreation> =
                identity_keys().into_iter().map(|(_, key)| key).collect();
            let mut carried = v0.public_keys.clone();
            for key in &mut carried {
                key.set_signature(BinaryData::default());
            }
            assert_eq!(carried, requested, "Create {path}: the requested keys");
            assert_eq!(
                v0.denomination, DENOMINATION,
                "Create {path}: the requested denomination"
            );
            DENOMINATION
        }
        (kind, st) => panic!("{kind:?} {path}: unexpected transition {st:?}"),
    }
}

/// `st_bytes`, a withdrawal, pays the requested Core address at the
/// requested fee rate, with the only pooling the entry point requests.
fn assert_withdrawal_request(st_bytes: &[u8], path: &str) {
    let st = StateTransition::deserialize_from_bytes_trusted(st_bytes).expect("decode transition");
    let StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(v0)) = st else {
        panic!("{path}: not a shielded withdrawal: {st:?}");
    };
    let requested = core_address()
        .parse::<dashcore::Address<dashcore::address::NetworkUnchecked>>()
        .expect("the requested core address parses")
        .require_network(dash_sdk::Sdk::new_mock().network)
        .expect("the requested core address is on the mock network");
    assert_eq!(
        v0.output_script,
        CoreScript::from_bytes(requested.script_pubkey().to_bytes()),
        "Withdraw {path}: pays the requested Core address"
    );
    assert_eq!(
        v0.core_fee_per_byte, CORE_FEE_PER_BYTE,
        "Withdraw {path}: the requested Core fee rate"
    );
    assert_eq!(
        v0.pooling,
        Pooling::Never,
        "Withdraw {path}: the requested pooling"
    );
}

/// For the same wallet, account, note, anchor and arguments, the seed
/// wrapper and the keyset path hand the hook the same artifact. The proof,
/// binding signature and output notes are randomized per build, so
/// `st_bytes`, `st_hash`, `activity_id`, the padding nullifier and so a
/// created identity's id differ; every other payload field and every
/// deterministic transition field match, and the outputs of both decrypt to
/// the same payment and change.
#[tokio::test]
async fn the_seed_and_keyset_paths_hand_the_hook_the_same_artifact() {
    for kind in KINDS {
        let own = keyset(&SEED, ACCOUNT);
        let note = owned_note(&own);
        let (by_seed, seed_hook, anchor) =
            hooked_coordinator(&format!("same_seed_{kind:?}"), &note, false).await;
        let (by_keyset, keyset_hook, keyset_anchor) =
            hooked_coordinator(&format!("same_keyset_{kind:?}"), &note, false).await;
        assert_eq!(anchor, keyset_anchor, "the same tree");
        let (wallet, _persister) =
            bound_wallet(&by_seed, anchor, &note, &[&by_seed, &by_keyset]).await;

        let seed_result = spend_with_seed(&wallet, &by_seed, &SEED, kind).await;
        let keyset_result = spend_with_keyset(&wallet, &by_keyset, &own, ACCOUNT, kind).await;

        for (path, result) in [("seed", &seed_result), ("keyset", &keyset_result)] {
            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::ShieldedDispatchRefused { .. })
                ),
                "{kind:?} {path}: refused by the hook, got {result:?}"
            );
        }
        let (seed_seen, keyset_seen) = (seed_hook.seen(), keyset_hook.seen());
        assert_eq!(seed_seen.len(), 1, "{kind:?}: seed path dispatched once");
        assert_eq!(
            keyset_seen.len(),
            1,
            "{kind:?}: keyset path dispatched once"
        );
        assert_artifact(&seed_seen[0], kind, &wallet, &note, anchor);
        assert_artifact(&keyset_seen[0], kind, &wallet, &note, anchor);

        let (seed, keyset) = (&seed_seen[0].payload, &keyset_seen[0].payload);
        // A create's id is derived from its published nullifiers, padding
        // included, so it differs per build; each is pinned to its own
        // nullifiers by `assert_artifact`.
        if kind != Kind::Create {
            assert_eq!(seed.kind, keyset.kind, "{kind:?}: kind");
        }
        assert_eq!(seed.wallet_id, keyset.wallet_id, "{kind:?}: wallet");
        assert_eq!(
            seed.account_index, keyset.account_index,
            "{kind:?}: account"
        );
        assert_eq!(seed.anchor, keyset.anchor, "{kind:?}: anchor");
        assert_eq!(
            seed.selected_notes, keyset.selected_notes,
            "{kind:?}: selected notes"
        );
        assert_eq!(
            seed.published_nullifiers.len(),
            keyset.published_nullifiers.len(),
            "{kind:?}: action count"
        );
        assert_eq!(
            deterministic_fields(kind, &seed.st_bytes),
            deterministic_fields(kind, &keyset.st_bytes),
            "{kind:?}: deterministic transition fields"
        );
        assert_outputs(kind, &seed.st_bytes, "seed");
        assert_outputs(kind, &keyset.st_bytes, "keyset");
    }
}

/// A create on the keyset path that Platform confirms registers the new
/// identity locally, at the requested slot. The mock can't answer the
/// broadcast or its result wait, so the confirmation is the create's own
/// fallback: the identity is found by the id its nullifiers derive. Two
/// notes that each fall short of the denomination are both spent, so the
/// bundle has no padding action and that id is known before the call.
#[tokio::test]
async fn a_confirmed_keyset_create_registers_the_identity_at_its_slot() {
    a_confirmed_create_registers_the_identity_at_its_slot(false).await;
}

/// [`a_confirmed_keyset_create_registers_the_identity_at_its_slot`] on the
/// seed wrapper, which registers the identity itself.
#[tokio::test]
async fn a_confirmed_seed_create_registers_the_identity_at_its_slot() {
    a_confirmed_create_registers_the_identity_at_its_slot(true).await;
}

async fn a_confirmed_create_registers_the_identity_at_its_slot(by_seed: bool) {
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::state_transition::identity_create_from_shielded_pool_transition::identity_id_from_nullifiers;

    let own = keyset(&SEED, ACCOUNT);
    let notes = [
        owned_note_at(&own, DENOMINATION * 3 / 5, 0),
        owned_note_at(&own, DENOMINATION * 3 / 5, 1),
    ];
    let expected_id = identity_id_from_nullifiers(&[notes[0].nullifier, notes[1].nullifier]);
    let created = dpp::identity::Identity::new_with_id_and_keys(
        expected_id,
        identity_keys()
            .into_iter()
            .map(|(key, _)| (key.id(), key))
            .collect(),
        dpp::version::PlatformVersion::latest(),
    )
    .expect("the created identity");
    let (coordinator, hook, anchor) =
        hooked_coordinator_holding(&format!("create_registers_{by_seed}"), &notes, true).await;
    let (wallet, _persister) =
        bound_wallet_holding(&coordinator, anchor, &notes, &[&coordinator], Some(created)).await;
    let wallet_id = wallet.wallet_id();
    {
        let manager = wallet.wallet_manager().read().await;
        let info = manager.get_wallet_info(&wallet_id).expect("wallet info");
        assert!(
            info.identity_manager
                .wallet_identity_ids(&wallet_id)
                .is_empty(),
            "no identity before the create"
        );
    }

    let prover = CachedOrchardProver::new();
    let identity_id = if by_seed {
        wallet
            .shielded_identity_create_from_pool(
                &coordinator,
                &SEED,
                ACCOUNT,
                IDENTITY_INDEX,
                identity_keys(),
                DENOMINATION,
                creation_failure_address(),
                &FixedSigner,
                &prover,
            )
            .await
    } else {
        wallet
            .shielded_identity_create_from_pool_with_keyset(
                &coordinator,
                &own,
                ACCOUNT,
                IDENTITY_INDEX,
                identity_keys(),
                DENOMINATION,
                creation_failure_address(),
                &FixedSigner,
                &prover,
            )
            .await
    }
    .expect("the create is confirmed");

    assert_eq!(identity_id, expected_id, "the id the nullifiers derive");
    let seen = hook.seen();
    assert_eq!(seen.len(), 1, "one dispatch");
    assert_eq!(
        seen[0].payload.kind,
        ShieldedDispatchKind::IdentityCreate {
            identity_id: expected_id.to_buffer(),
        },
        "the dispatched create is the confirmed one"
    );
    let manager = wallet.wallet_manager().read().await;
    let identities = &manager
        .get_wallet_info(&wallet_id)
        .expect("wallet info")
        .identity_manager;
    assert_eq!(
        identities.wallet_identity_ids(&wallet_id),
        vec![identity_id],
        "the created identity is the wallet's one identity"
    );
    assert_eq!(
        identities.identity_index(&identity_id),
        Some(IDENTITY_INDEX),
        "registered at the requested slot"
    );
}
