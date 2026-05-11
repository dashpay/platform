//! Bank identity — destination of identity-credit sweeps.
//!
//! Identity-to-identity credit transfers (the only sweep path the
//! `CreditTransfer` state transition supports) need an existing
//! identity to receive funds. Tests share a single bank identity so
//! swept credits accumulate in one place rather than leaking on
//! every run.
//!
//! Bootstrap policy:
//! - If `PLATFORM_WALLET_E2E_BANK_IDENTITY_ID` is set, parse it and
//!   trust the operator — no on-chain check at init time.
//! - Otherwise read `<workdir>/bank_identity.json`. If present,
//!   reuse the persisted id.
//! - Otherwise register a fresh identity from the bank's primary
//!   receive address, persist its id to the workdir slot, and
//!   reuse it on subsequent runs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::types::identity::PublicKeyHash;
use dash_sdk::platform::Fetch;
use dash_sdk::Sdk;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::PlatformWalletManager;
use serde::{Deserialize, Serialize};

use super::bank::BankWallet;
use super::signer::{derive_identity_key, SeedBackedIdentitySigner};
use super::wait::wait_for_identity_balance;
use super::{FrameworkError, FrameworkResult};

/// DIP-9 identity index reserved for the bank identity. Tests use
/// 0..N for their own identities; pinning the bank to a high index
/// keeps the two namespaces from colliding when a sweep run also
/// registers a fresh test identity at index 0.
pub const BANK_IDENTITY_INDEX: u32 = 0xBA77;

/// Funding the bootstrap registration consumes from the bank's
/// primary receive address. Sized well above the chain-time
/// registration fee so the new identity carries useful balance for
/// swept credits to land on top of.
pub const BANK_IDENTITY_BOOTSTRAP_FUNDING: Credits = 80_000_000;

/// Post-registration on-chain visibility timeout for the bootstrap
/// path. Generous because bootstrap only happens once per bank.
const BOOTSTRAP_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(60);

/// Persisted bank-identity record at `<workdir>/bank_identity.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedBankIdentity {
    /// Hex-encoded 32-byte identity id.
    identity_id_hex: String,
    /// Hex-encoded `wallet_id` (32 bytes) the identity was derived
    /// from. Cross-check on load — a different bank mnemonic on the
    /// same workdir is an operator error and surfaces as a clear
    /// mismatch instead of a silent wrong-bank sweep.
    wallet_id_hex: String,
    /// DIP-9 identity index used at registration. Pinned to
    /// [`BANK_IDENTITY_INDEX`] today; serialised so future bumps
    /// land cleanly without breaking older slots.
    identity_index: u32,
}

/// Bank identity handle — id plus a pre-built signer for its
/// auth keys.
#[derive(Clone)]
pub struct BankIdentity {
    /// On-chain identity id.
    pub id: Identifier,
    /// `Signer<IdentityPublicKey>` over the bank's seed at
    /// [`BANK_IDENTITY_INDEX`]. Wrapped in `Arc` so multiple sweep
    /// drivers can hold it without re-deriving the key cache.
    pub signer: Arc<SeedBackedIdentitySigner>,
    /// DIP-9 identity index recorded at registration / load.
    pub identity_index: u32,
}

impl std::fmt::Debug for BankIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankIdentity")
            .field("id", &self.id)
            .field("identity_index", &self.identity_index)
            .finish_non_exhaustive()
    }
}

/// Resolve the bank identity, registering it on first run if needed.
///
/// Resolution order:
/// 1. `bank_identity_env` — operator-supplied hex id (already parsed
///    out of the env var by [`super::config::Config`]).
/// 2. `<workdir>/bank_identity.json` — first-run record produced by
///    a prior process on this slot.
/// 3. Auto-register from the bank's primary receive address, persist
///    the resulting id, return it.
pub async fn resolve_bank_identity(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    workdir: &Path,
    bank_identity_env: Option<&str>,
    network: Network,
) -> FrameworkResult<BankIdentity> {
    // Build the signer up front — it's cheap and used by every
    // resolution branch below for downstream sweeps regardless of
    // how the id is sourced.
    let signer = Arc::new(SeedBackedIdentitySigner::new(
        bank.seed_bytes(),
        network,
        BANK_IDENTITY_INDEX,
    )?);

    if let Some(raw) = bank_identity_env {
        let id = parse_identifier_hex(raw).map_err(|err| {
            FrameworkError::Bank(format!(
                "PLATFORM_WALLET_E2E_BANK_IDENTITY_ID = {raw:?} is not a 32-byte hex id: {err}"
            ))
        })?;
        tracing::info!(
            target: "platform_wallet::e2e::bank_identity",
            identity_id = %hex::encode(id),
            "loaded bank identity from env"
        );
        return Ok(BankIdentity {
            id,
            signer,
            identity_index: BANK_IDENTITY_INDEX,
        });
    }

    let path = workdir.join("bank_identity.json");
    let bank_wallet_id_hex = hex::encode(bank.platform_wallet().wallet_id());

    if let Some(persisted) = read_persisted(&path)? {
        if persisted.wallet_id_hex != bank_wallet_id_hex {
            return Err(FrameworkError::Bank(format!(
                "bank_identity.json wallet_id {} does not match active bank wallet id {}; \
                 either point PLATFORM_WALLET_E2E_BANK_IDENTITY_ID at the right id or \
                 remove the stale persistence file",
                persisted.wallet_id_hex, bank_wallet_id_hex
            )));
        }
        let id = parse_identifier_hex(&persisted.identity_id_hex).map_err(|err| {
            FrameworkError::Bank(format!(
                "bank_identity.json identity_id_hex {:?} is not a 32-byte hex id: {err}",
                persisted.identity_id_hex
            ))
        })?;
        tracing::info!(
            target: "platform_wallet::e2e::bank_identity",
            identity_id = %hex::encode(id),
            path = %path.display(),
            "loaded bank identity from workdir slot"
        );
        return Ok(BankIdentity {
            id,
            signer,
            identity_index: persisted.identity_index,
        });
    }

    // Bootstrap path — derive the deterministic master auth key first
    // so we can decide between two cases without re-running derivation:
    //   (a) the on-chain identity already exists (workdir was wiped
    //       between runs but Drive still holds the prior registration)
    //       — fetch by master-key public-key hash and reuse the id;
    //   (b) genuinely fresh — register from the bank's primary receive
    //       address.
    // Without (a) the second run after a wipe panics inside Drive with
    // `a unique key with that hash already exists` and cascades into
    // `tx already exists in cache` failures across the whole suite
    // (QA-100).
    let bank_seed = bank.seed_bytes();
    let master_key = derive_identity_key(
        bank_seed,
        network,
        BANK_IDENTITY_INDEX,
        0,
        Purpose::AUTHENTICATION,
        SecurityLevel::MASTER,
    )?;
    let high_key = derive_identity_key(
        bank_seed,
        network,
        BANK_IDENTITY_INDEX,
        1,
        Purpose::AUTHENTICATION,
        SecurityLevel::HIGH,
    )?;

    let id = if let Some(existing_id) =
        try_recover_on_chain(bank.platform_wallet().sdk(), &master_key).await?
    {
        tracing::info!(
            target: "platform_wallet::e2e::bank_identity",
            identity_id = %hex::encode(existing_id),
            path = %path.display(),
            "bank identity recovered from on-chain state (workdir was wiped, identity already registered)"
        );
        existing_id
    } else {
        let id = bootstrap_register(manager, bank, network, &master_key, &high_key).await?;
        tracing::info!(
            target: "platform_wallet::e2e::bank_identity",
            identity_id = %hex::encode(id),
            path = %path.display(),
            "registered bank identity and persisted to workdir slot"
        );
        id
    };

    write_persisted(
        &path,
        &PersistedBankIdentity {
            identity_id_hex: hex::encode(id),
            wallet_id_hex: bank_wallet_id_hex,
            identity_index: BANK_IDENTITY_INDEX,
        },
    )?;

    Ok(BankIdentity {
        id,
        signer,
        identity_index: BANK_IDENTITY_INDEX,
    })
}

/// Try to recover the bank identity by looking it up on chain via the
/// deterministic master auth key's public-key hash.
///
/// Returns `Ok(Some(id))` when Drive already has an identity owning
/// that unique key (the workdir-wipe-after-prior-run case), `Ok(None)`
/// when the network confirms no such identity exists. Network errors
/// surface as [`FrameworkError::Bank`] — we cannot safely fall through
/// to a fresh registration because the collision-on-register would
/// then panic the whole suite (QA-100).
async fn try_recover_on_chain(
    sdk: &Sdk,
    master_key: &IdentityPublicKey,
) -> FrameworkResult<Option<Identifier>> {
    let pkh = master_key.public_key_hash().map_err(|err| {
        FrameworkError::Bank(format!(
            "computing public-key hash for bank-identity recovery: {err}"
        ))
    })?;
    match Identity::fetch(sdk, PublicKeyHash(pkh)).await {
        Ok(Some(identity)) => Ok(Some(identity.id())),
        Ok(None) => Ok(None),
        Err(err) => Err(FrameworkError::Bank(format!(
            "looking up bank identity by public-key hash {} for recovery: {err}",
            hex::encode(pkh)
        ))),
    }
}

/// Register a fresh bank identity from the bank's primary receive
/// address. Caller is responsible for persistence and for having
/// already verified that the on-chain identity does not yet exist
/// for `master_key`'s public-key hash (see [`try_recover_on_chain`]).
async fn bootstrap_register(
    _manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    network: Network,
    master_key: &IdentityPublicKey,
    high_key: &IdentityPublicKey,
) -> FrameworkResult<Identifier> {
    let bank_wallet = bank.platform_wallet();
    let seed = bank.seed_bytes();
    let funding_address = *bank.primary_receive_address();

    // Refuse to bootstrap when the bank's primary address can't
    // cover the bootstrap funding plus a reasonable fee — the
    // registration would fail downstream with a less actionable
    // error.
    let balances = bank_wallet
        .platform()
        .addresses_with_balances()
        .await
        .into_iter()
        .collect::<BTreeMap<PlatformAddress, Credits>>();
    let primary_balance = balances.get(&funding_address).copied().unwrap_or(0);
    if primary_balance < BANK_IDENTITY_BOOTSTRAP_FUNDING {
        return Err(FrameworkError::Bank(format!(
            "bank primary address {} balance {} below bootstrap funding {}; top up before re-running",
            funding_address.to_bech32m_string(network),
            primary_balance,
            BANK_IDENTITY_BOOTSTRAP_FUNDING,
        )));
    }

    let identity_signer = SeedBackedIdentitySigner::new(seed, network, BANK_IDENTITY_INDEX)?;

    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    let mut public_keys: BTreeMap<KeyID, IdentityPublicKey> = BTreeMap::new();
    public_keys.insert(master_key.id(), master_key.clone());
    public_keys.insert(high_key.id(), high_key.clone());
    let placeholder = Identity::V0(IdentityV0 {
        id: Identifier::default(),
        public_keys,
        balance: 0,
        revision: 0,
    });

    let inputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((funding_address, BANK_IDENTITY_BOOTSTRAP_FUNDING)).collect();

    let registered = bank_wallet
        .identity()
        .register_from_addresses(
            &placeholder,
            inputs,
            None,
            BANK_IDENTITY_INDEX,
            &identity_signer,
            bank.address_signer(),
            None,
        )
        .await
        .map_err(|err| FrameworkError::Bank(format!("bank-identity bootstrap: {err}")))?;

    // Wait for the new identity to settle on chain so subsequent
    // sweeps can transfer credits to it without racing visibility.
    wait_for_identity_balance(
        bank_wallet.sdk(),
        registered.id(),
        BANK_IDENTITY_BOOTSTRAP_FUNDING / 2,
        BOOTSTRAP_VISIBILITY_TIMEOUT,
    )
    .await?;

    Ok(registered.id())
}

fn parse_identifier_hex(raw: &str) -> Result<Identifier, String> {
    let trimmed = raw.trim();
    let bytes = hex::decode(trimmed).map_err(|err| err.to_string())?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected 32 bytes, got {}", v.len()))?;
    Ok(Identifier::from(arr))
}

fn read_persisted(path: &Path) -> FrameworkResult<Option<PersistedBankIdentity>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let parsed: PersistedBankIdentity = serde_json::from_slice(&bytes).map_err(|err| {
                FrameworkError::Bank(format!(
                    "parsing bank_identity.json at {}: {err}",
                    path.display()
                ))
            })?;
            Ok(Some(parsed))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(FrameworkError::Bank(format!(
            "reading bank_identity.json at {}: {err}",
            path.display()
        ))),
    }
}

fn write_persisted(path: &Path, record: &PersistedBankIdentity) -> FrameworkResult<()> {
    use std::io::Write;

    let bytes = serde_json::to_vec_pretty(record).map_err(|err| {
        FrameworkError::Bank(format!(
            "serialising bank_identity.json to {}: {err}",
            path.display()
        ))
    })?;
    let parent = path
        .parent()
        .ok_or_else(|| FrameworkError::Bank(format!("path {} has no parent", path.display())))?;
    std::fs::create_dir_all(parent)
        .map_err(|err| FrameworkError::Bank(format!("creating {}: {err}", parent.display())))?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|err| {
        FrameworkError::Bank(format!("creating temp file in {}: {err}", parent.display()))
    })?;
    tmp.write_all(&bytes).map_err(|err| {
        FrameworkError::Bank(format!("writing temp file {}: {err}", tmp.path().display()))
    })?;
    tmp.as_file_mut().flush().map_err(|err| {
        FrameworkError::Bank(format!(
            "flushing temp file {}: {err}",
            tmp.path().display()
        ))
    })?;
    tmp.persist(path).map_err(|err| {
        FrameworkError::Bank(format!("persisting temp file -> {}: {err}", path.display()))
    })?;
    Ok(())
}

/// Path to the persisted bank-identity record under `workdir`.
/// Exposed so tests can introspect / reset the file.
pub fn persisted_path(workdir: &Path) -> PathBuf {
    workdir.join("bank_identity.json")
}
