//! Bank identity — transient mid-run sink, persisted across runs for
//! legacy compatibility.
//!
//! Identity-side test sweeps now drain directly to the bank's Platform
//! address (the single Platform-side funding pool — see
//! [`super::bank_rebalance`] for the design contract), so this identity
//! no longer accumulates credits during a run. It remains registered
//! and persisted at `<workdir>/bank_identity.json` because:
//!
//! - The core-refill chain ([`super::bank_rebalance::refill_core_from_platform_if_below_threshold`])
//!   uses it as a transient buffer when chaining
//!   `top_up_from_addresses` → `withdraw_credits_with_external_signer`.
//! - Any residual balance from older runs is drained back to the bank
//!   Platform address at suite start by
//!   [`super::bank_rebalance::drain_bank_identity_to_addresses`].
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
use super::bank_rebalance::{
    self, bootstrap_lock_duff, BOOTSTRAP_ASSET_LOCK_FEE_RESERVE, PLATFORM_BOOTSTRAP_FEE_RESERVE,
};
use super::signer::{derive_identity_key, SeedBackedIdentitySigner};
use super::wait::{wait_for_identity_balance, wait_for_identity_visible_to_platform};
use super::{FrameworkError, FrameworkResult};

/// DIP-9 identity index reserved for the bank identity. Tests use
/// 0..N for their own identities; pinning the bank to a high index
/// keeps the two namespaces from colliding when a sweep run also
/// registers a fresh test identity at index 0.
pub const BANK_IDENTITY_INDEX: u32 = 0xBA77;

/// Minimum credits the bank's primary Platform address must hold before
/// the bootstrap registers the bank identity. Doubles as the self-fund
/// trigger floor. Live paloma runs show the registration transition's
/// chain-time `required_balance` is ~96M credits, so this sits well above
/// that so a partially-funded address (e.g. ~87M from an interrupted prior
/// run) still triggers a top-up rather than dead-ending in registration.
pub const BANK_IDENTITY_BOOTSTRAP_FUNDING: Credits = 200_000_000;

/// Core (duff) headroom required on top of the asset-lock amount for the
/// L1 lock transaction's own fee. The asset-lock builder picks the exact
/// fee at broadcast time; this is a conservative pre-check floor so a
/// self-fund attempt that can't even pay the Core fee fails with an
/// actionable operator error instead of deep inside the broadcast.
///
/// The pre-check is a coarse gate against [`BankWallet::core_balance_confirmed`],
/// which is not transactionally consistent with the spendable-UTXO set —
/// it can pass on a confirmed figure the builder can't realise (funds in
/// in-flight change, reserved UTXOs). The broadcast inside
/// [`bank_rebalance::asset_lock_core_to_platform`] is the authoritative
/// check; this only fails the obviously-too-poor bank fast and clearly.
const BOOTSTRAP_CORE_FEE_RESERVE_DUFF: u64 = 10_000;

/// Upper bound on the Core (duff) outlay one bootstrap self-fund can consume:
/// the largest possible asset-lock — the full gross target locked from a
/// zero-balance address — plus [`BOOTSTRAP_CORE_FEE_RESERVE_DUFF`]. The
/// harness polls the confirmed Core atomic back to within this bound of its
/// pre-bootstrap value before the fund-planner snapshot, because that atomic
/// is fed asynchronously by the SPV pipeline and lags the awaited self-fund.
pub const MAX_BOOTSTRAP_CORE_OUTLAY_DUFF: u64 = bootstrap_lock_duff(
    0,
    BANK_IDENTITY_BOOTSTRAP_FUNDING
        .saturating_add(PLATFORM_BOOTSTRAP_FEE_RESERVE)
        .saturating_add(BOOTSTRAP_ASSET_LOCK_FEE_RESERVE),
)
.saturating_add(BOOTSTRAP_CORE_FEE_RESERVE_DUFF);

/// Funding-type tag (see `PlatformWalletManager::tracked_asset_locks_blocking`'s
/// `lock_type`) for asset-locks that top up a Platform address — what the
/// bootstrap self-fund mints. Used to spot a prior run's actionable lock
/// before minting a second (QA-001 double-spend guard).
const LOCK_TYPE_ASSET_LOCK_ADDRESS_TOP_UP: u8 = 4;

/// Terminal `lock_type` status (see `tracked_asset_locks_blocking`):
/// `4 = Consumed`. Anything below is still actionable / unconsumed.
const LOCK_STATUS_CONSUMED: u8 = 4;

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
    disable_spv: bool,
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
        let id =
            bootstrap_register(manager, bank, network, &master_key, &high_key, disable_spv).await?;
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
///
/// When the primary Platform address is short of
/// [`BANK_IDENTITY_BOOTSTRAP_FUNDING`] and SPV is enabled, this self-funds
/// the shortfall (plus [`BOOTSTRAP_FEE_RESERVE`]) via a one-time
/// Core→Platform asset-lock ([`bank_rebalance::asset_lock_core_to_platform`])
/// before registering. It hard-errors with an operator-actionable message
/// only when self-funding genuinely cannot proceed: `disable_spv` is set,
/// or the bank's confirmed Core balance can't cover the lock plus its L1
/// fee.
async fn bootstrap_register(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    network: Network,
    master_key: &IdentityPublicKey,
    high_key: &IdentityPublicKey,
    disable_spv: bool,
) -> FrameworkResult<Identifier> {
    let bank_wallet = bank.platform_wallet();
    let seed = bank.seed_bytes();
    let funding_address = *bank.primary_receive_address();

    // The bank's primary Platform address must cover the bootstrap
    // funding before registering. If it's short, self-fund the shortfall
    // from the bank's Core balance via a one-time asset-lock (SPV-gated) —
    // only hard-error when self-funding can't proceed. On the Ok path
    // `fund_from_asset_lock` writes the proof-attested balance
    // synchronously before returning, so no post-fund re-check is needed.
    let primary_balance = primary_platform_balance(bank, &funding_address).await;
    if needs_self_fund(primary_balance) {
        self_fund_bootstrap(
            manager,
            bank,
            &funding_address,
            primary_balance,
            network,
            disable_spv,
        )
        .await?;
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

    let (registered, _address_infos, _) = bank_wallet
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

    // The asset-lock-funded path can have a lagging DAPI replica return
    // `Ok(None)` for the fresh identity even after the balance wait clears
    // on one node; the very next harness step (`provision_transfer_key_if_missing`)
    // fetches this identity and silently skips on a miss. Gate on a
    // 2-success streak so that fetch sees a converged replica (QA-911).
    wait_for_identity_visible_to_platform(
        bank_wallet.sdk(),
        registered.id(),
        BOOTSTRAP_VISIBILITY_TIMEOUT,
        2,
    )
    .await?;

    Ok(registered.id())
}

/// Confirmed credit balance of the bank's `address` on Platform, read
/// from the wallet's address-balance map (`0` if absent).
async fn primary_platform_balance(bank: &BankWallet, address: &PlatformAddress) -> Credits {
    bank.platform_wallet()
        .platform()
        .addresses_with_balances()
        .await
        .into_iter()
        .collect::<BTreeMap<PlatformAddress, Credits>>()
        .get(address)
        .copied()
        .unwrap_or(0)
}

/// The bootstrap must self-fund when the primary Platform balance is
/// strictly below [`BANK_IDENTITY_BOOTSTRAP_FUNDING`]. A balance exactly at
/// the floor is sufficient — no self-fund.
fn needs_self_fund(primary_credits: Credits) -> bool {
    primary_credits < BANK_IDENTITY_BOOTSTRAP_FUNDING
}

/// A tracked asset-lock is unconsumed (still holds committed Core value)
/// when it tops up a Platform address and has not reached the terminal
/// `Consumed` status. Minting a fresh lock while one of these exists would
/// orphan the prior lock's duffs (QA-001 double-spend).
fn is_unconsumed_address_lock(lock_type: u8, status: u8) -> bool {
    lock_type == LOCK_TYPE_ASSET_LOCK_ADDRESS_TOP_UP && status != LOCK_STATUS_CONSUMED
}

/// Top the bank's primary Platform `address` up to at least
/// [`BANK_IDENTITY_BOOTSTRAP_FUNDING`] (plus
/// [`PLATFORM_BOOTSTRAP_FEE_RESERVE`]) via a Core→Platform asset-lock,
/// sizing the lock from the current balance.
///
/// Pre-checks that self-funding can actually proceed and returns an
/// operator-actionable [`FrameworkError::Bank`] otherwise:
/// - `disable_spv` is set (the asset-lock proof needs SPV);
/// - a prior run left an unconsumed Platform asset-lock (minting a second
///   would orphan its Core value — see TODO(QA-001)); or
/// - the bank's confirmed Core balance can't cover the lock plus its L1
///   fee — names the Core top-up address and the shortfall.
async fn self_fund_bootstrap(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    address: &PlatformAddress,
    current_credits: Credits,
    network: Network,
    disable_spv: bool,
) -> FrameworkResult<()> {
    if disable_spv {
        return Err(FrameworkError::Bank(format!(
            "bank primary address {} balance {} below bootstrap funding {} and \
             PLATFORM_WALLET_E2E_DISABLE_SPV is set, so the Core→Platform asset-lock \
             self-fund (which needs SPV for the ChainLocked proof) can't run. Enable SPV \
             or fund the bank's Platform address directly, then re-run.",
            address.to_bech32m_string(network),
            current_credits,
            BANK_IDENTITY_BOOTSTRAP_FUNDING,
        )));
    }

    // QA-001 guard: refuse to mint a fresh lock if a prior run already
    // broadcast one (process died before the credit write + persistence).
    // The tracked-lock snapshot does not carry the recipient address, so we
    // can't yet target a resume to THIS address from here — bail loudly
    // instead of silently double-spending Core into a second lock.
    // TODO(QA-001): resume the existing lock via
    // `AssetLockFunding::FromExistingAssetLock { out_point }` once the
    // snapshot exposes the recipient (or a per-address lookup lands), so a
    // crashed mid-bootstrap re-run advances the in-flight lock instead of
    // erroring.
    let wallet_id = bank.platform_wallet().wallet_id();
    let pending: Vec<_> = manager
        .tracked_asset_locks(&wallet_id)
        .await
        .into_iter()
        .filter(|l| is_unconsumed_address_lock(l.lock_type, l.status))
        .collect();
    if !pending.is_empty() {
        for lock in &pending {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_identity",
                outpoint = %lock.outpoint,
                status = lock.status,
                "unconsumed Platform asset-lock from a prior run — NOT minting a fresh lock (QA-001)"
            );
        }
        return Err(FrameworkError::Bank(format!(
            "bank has {} unconsumed Platform asset-lock(s) from a prior run; a fresh self-fund \
             would orphan their Core value (double-spend). Resume or consume the existing \
             lock(s) before re-running the bootstrap (TODO(QA-001): auto-resume here).",
            pending.len(),
        )));
    }

    // Gross lock target: the registration funding floor + post-bootstrap
    // leaf-funding reserve + the asset-lock funding fee (deducted from the
    // lock before it lands). The first two set the NET the address must
    // hold; the third keeps that net intact through the funding fee.
    let target_credits = BANK_IDENTITY_BOOTSTRAP_FUNDING
        .saturating_add(PLATFORM_BOOTSTRAP_FEE_RESERVE)
        .saturating_add(BOOTSTRAP_ASSET_LOCK_FEE_RESERVE);
    let lock_duff = bootstrap_lock_duff(current_credits, target_credits);
    let required_core_duff = lock_duff.saturating_add(BOOTSTRAP_CORE_FEE_RESERVE_DUFF);
    let confirmed_core_duff = bank.core_balance_confirmed();
    if confirmed_core_duff < required_core_duff {
        let top_up_addr = match bank.primary_core_receive_address().await {
            Ok(addr) => addr.to_string(),
            Err(err) => format!("<unresolved: {err}>"),
        };
        return Err(FrameworkError::Bank(format!(
            "bank Core balance too low to self-fund the bank-identity bootstrap.\n  \
             Platform address : {addr}\n  \
             Platform balance : {current_credits} credits (need {need} to register)\n  \
             Core confirmed   : {confirmed_core_duff} duffs\n  \
             Core required    : {required_core_duff} duffs (asset-lock {lock_duff} + L1 fee reserve {fee})\n  \
             Core top-up addr : {top_up_addr}\n\
             \n\
             Send testnet Core duffs to the Core address above, then re-run — the framework \
             will asset-lock them into Platform credits automatically.",
            addr = address.to_bech32m_string(network),
            need = BANK_IDENTITY_BOOTSTRAP_FUNDING,
            fee = BOOTSTRAP_CORE_FEE_RESERVE_DUFF,
        )));
    }

    tracing::info!(
        target: "platform_wallet::e2e::bank_identity",
        platform_address = %address.to_bech32m_string(network),
        current_credits,
        target_credits,
        lock_duff,
        confirmed_core_duff,
        "bank-identity bootstrap: Platform short of funding, self-funding via Core→Platform asset-lock"
    );

    bank_rebalance::asset_lock_core_to_platform(bank, lock_duff, disable_spv).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_fund_triggers_only_strictly_below_floor() {
        assert!(needs_self_fund(BANK_IDENTITY_BOOTSTRAP_FUNDING - 1));
        // Exactly at the floor is sufficient — no self-fund (the `<` boundary).
        assert!(!needs_self_fund(BANK_IDENTITY_BOOTSTRAP_FUNDING));
        assert!(!needs_self_fund(BANK_IDENTITY_BOOTSTRAP_FUNDING + 1));
    }

    #[test]
    fn lock_sizing_from_empty_hits_the_exact_duff_count() {
        // 200M + 100M + 150M = 450M credits / 1000 credits-per-duff == 450_000 duffs.
        let target = BANK_IDENTITY_BOOTSTRAP_FUNDING
            .saturating_add(PLATFORM_BOOTSTRAP_FEE_RESERVE)
            .saturating_add(BOOTSTRAP_ASSET_LOCK_FEE_RESERVE);
        assert_eq!(bootstrap_lock_duff(0, target), 450_000);
    }

    #[test]
    fn only_unconsumed_address_locks_block_minting() {
        // AssetLockAddressTopUp (4) below Consumed (4) → blocks a fresh mint.
        assert!(is_unconsumed_address_lock(
            LOCK_TYPE_ASSET_LOCK_ADDRESS_TOP_UP,
            0
        )); // Built
        assert!(is_unconsumed_address_lock(
            LOCK_TYPE_ASSET_LOCK_ADDRESS_TOP_UP,
            1
        )); // Broadcast
            // Consumed → no longer holds value, doesn't block.
        assert!(!is_unconsumed_address_lock(
            LOCK_TYPE_ASSET_LOCK_ADDRESS_TOP_UP,
            LOCK_STATUS_CONSUMED
        ));
        // A different funding type (e.g. identity registration) isn't ours.
        assert!(!is_unconsumed_address_lock(0, 1));
    }
}
