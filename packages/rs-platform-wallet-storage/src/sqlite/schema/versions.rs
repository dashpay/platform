//! Store-scoped version + generation metadata: `meta_data_versions` and
//! `meta_store_generation`.
//!
//! `meta_data_versions` carries a monotonic `seq` per `(wallet_id, domain)`,
//! bumped inside the flush transaction so a domain's cache-invalidation
//! marker and its data commit atomically. `meta_store_generation` holds the
//! single store-generation token, stable across flushes and regenerated on
//! restore.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::{Merge, PlatformWalletChangeSet};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;

/// A wallet-state family whose durable version `seq` is bumped when the
/// matching changeset field is flushed. One variant per persisted
/// changeset field — the cache-invalidation keystone (R8).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Domain {
    Core,
    Identities,
    IdentityKeys,
    Contacts,
    PlatformAddresses,
    AssetLocks,
    TokenBalances,
    DashpayProfiles,
    DashpayPaymentsOverlay,
    WalletMetadata,
    AccountRegistrations,
    AccountAddressPools,
    PendingContactCrypto,
}

impl Domain {
    /// Stable `meta_data_versions.domain` label. `Debug` is not a stable
    /// wire format; this match is the contract.
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Core => "core",
            Domain::Identities => "identities",
            Domain::IdentityKeys => "identity_keys",
            Domain::Contacts => "contacts",
            Domain::PlatformAddresses => "platform_addresses",
            Domain::AssetLocks => "asset_locks",
            Domain::TokenBalances => "token_balances",
            Domain::DashpayProfiles => "dashpay_profiles",
            Domain::DashpayPaymentsOverlay => "dashpay_payments_overlay",
            Domain::WalletMetadata => "wallet_metadata",
            Domain::AccountRegistrations => "account_registrations",
            Domain::AccountAddressPools => "account_address_pools",
            Domain::PendingContactCrypto => "pending_contact_crypto",
        }
    }

    /// Every domain, for coverage tests.
    #[cfg(any(test, feature = "__test-helpers"))]
    pub const ALL: [Domain; 13] = [
        Domain::Core,
        Domain::Identities,
        Domain::IdentityKeys,
        Domain::Contacts,
        Domain::PlatformAddresses,
        Domain::AssetLocks,
        Domain::TokenBalances,
        Domain::DashpayProfiles,
        Domain::DashpayPaymentsOverlay,
        Domain::WalletMetadata,
        Domain::AccountRegistrations,
        Domain::AccountAddressPools,
        Domain::PendingContactCrypto,
    ];
}

/// Domains carrying data in `cs`. The destructure is exhaustive (no `..`), so
/// adding a field to `PlatformWalletChangeSet` is a compile error here until
/// it gains a `Domain` variant and an arm below — the R8 forgotten-domain
/// guard. `shielded` (feature-gated; storage versions no shielded state here)
/// is bound and deliberately ignored; `provider_key_account_registrations` is
/// not yet mapped to a `Domain`, but a non-empty one is surfaced via a
/// `tracing::warn` rather than dropped in silence (deferred — see its binding
/// below).
pub fn touched_domains(cs: &PlatformWalletChangeSet) -> Vec<Domain> {
    let PlatformWalletChangeSet {
        core,
        identities,
        identity_keys,
        contacts,
        platform_addresses,
        asset_locks,
        token_balances,
        dashpay_profiles,
        dashpay_payments_overlay,
        wallet_metadata,
        account_registrations,
        provider_key_account_registrations,
        account_address_pools,
        pending_contact_crypto_added,
        pending_contact_crypto_cleared,
        #[cfg(feature = "shielded")]
        shielded,
    } = cs;
    #[cfg(feature = "shielded")]
    let _ = shielded;
    // `provider_key_account_registrations` (BLS operator-key / EdDSA
    // platform-node-key accounts) carries a live data path — both key types are
    // default-on — but is not yet mapped to a `Domain`. Deferred, not dropped:
    // nothing ever persisted it (this persister is new, so there is no
    // regression), and no consumer reads it back yet. Tracked in
    // https://github.com/dashpay/platform/issues/4113.
    if !provider_key_account_registrations.is_empty() {
        tracing::warn!(
            count = provider_key_account_registrations.len(),
            "provider-key account registrations in this changeset are not persisted yet \
             (deferred, tracked in dashpay/platform#4113) and will not survive a reload"
        );
    }

    // A sub-changeset carried but empty (`Some(default)`) is not a real
    // change; the `Merge::is_empty` bound is the shared emptiness contract.
    fn present<T: Merge>(opt: &Option<T>) -> bool {
        !opt.is_empty()
    }

    let mut out = Vec::new();
    if present(core) {
        out.push(Domain::Core);
    }
    if present(identities) {
        out.push(Domain::Identities);
    }
    if present(identity_keys) {
        out.push(Domain::IdentityKeys);
    }
    if present(contacts) {
        out.push(Domain::Contacts);
    }
    if present(platform_addresses) {
        out.push(Domain::PlatformAddresses);
    }
    if present(asset_locks) {
        out.push(Domain::AssetLocks);
    }
    if present(token_balances) {
        out.push(Domain::TokenBalances);
    }
    if dashpay_profiles.as_ref().is_some_and(|m| !m.is_empty()) {
        out.push(Domain::DashpayProfiles);
    }
    if dashpay_payments_overlay
        .as_ref()
        .is_some_and(|m| !m.is_empty())
    {
        out.push(Domain::DashpayPaymentsOverlay);
    }
    if wallet_metadata.is_some() {
        out.push(Domain::WalletMetadata);
    }
    if !account_registrations.is_empty() {
        out.push(Domain::AccountRegistrations);
    }
    if !account_address_pools.is_empty() {
        out.push(Domain::AccountAddressPools);
    }
    if !pending_contact_crypto_added.is_empty() || !pending_contact_crypto_cleared.is_empty() {
        out.push(Domain::PendingContactCrypto);
    }
    out
}

/// Saturating increment of one domain's `seq`, inside the caller's flush tx.
/// The first bump sets `seq = 1`; thereafter it increments but never wraps past
/// `i64::MAX` — a wrap to a lower value would look like a rollback to a
/// client's memoized `(generation, domain, seq)` cache and silently
/// reintroduce staleness (the exact bug class R8 exists to prevent).
pub fn bump_domain(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    domain: Domain,
) -> Result<(), WalletStorageError> {
    tx.prepare_cached(
        "INSERT INTO meta_data_versions (wallet_id, domain, seq) VALUES (?1, ?2, 1) \
         ON CONFLICT(wallet_id, domain) DO UPDATE SET \
            seq = CASE WHEN seq >= 9223372036854775807 THEN seq ELSE seq + 1 END",
    )?
    .execute(params![wallet_id.as_slice(), domain.as_str()])?;
    Ok(())
}

/// Bump every domain touched by `cs`, inside the caller's flush tx.
pub fn bump_touched_domains(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformWalletChangeSet,
) -> Result<(), WalletStorageError> {
    for domain in touched_domains(cs) {
        bump_domain(tx, wallet_id, domain)?;
    }
    Ok(())
}

/// Read the current `seq` for one `(wallet_id, domain)`; `0` when the domain
/// has never been bumped (no row).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn read_seq(
    conn: &rusqlite::Connection,
    wallet_id: &WalletId,
    domain: Domain,
) -> Result<i64, WalletStorageError> {
    use rusqlite::OptionalExtension;
    let seq: Option<i64> = conn
        .query_row(
            "SELECT seq FROM meta_data_versions WHERE wallet_id = ?1 AND domain = ?2",
            params![wallet_id.as_slice(), domain.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    Ok(seq.unwrap_or(0))
}

/// Read the 16-byte store-generation token written by V003. `None` on a
/// pre-V003 store (the table is absent).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn read_generation(
    conn: &rusqlite::Connection,
) -> Result<Option<[u8; 16]>, WalletStorageError> {
    use rusqlite::OptionalExtension;
    if !generation_table_exists(conn)? {
        return Ok(None);
    }
    let bytes: Option<Vec<u8>> = conn
        .query_row(
            "SELECT generation FROM meta_store_generation WHERE id = 0",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match bytes {
        None => Ok(None),
        Some(b) => {
            let arr: [u8; 16] = b.as_slice().try_into().map_err(|_| {
                WalletStorageError::blob_decode("meta_store_generation.generation not 16 bytes")
            })?;
            Ok(Some(arr))
        }
    }
}

/// Regenerate the store-generation token so a restored copy is
/// distinguishable from its source. A no-op on a pre-V003 store (no table);
/// such a store gets a fresh token when it later migrates to V003.
pub fn regenerate_generation(conn: &rusqlite::Connection) -> Result<(), WalletStorageError> {
    if !generation_table_exists(conn)? {
        return Ok(());
    }
    conn.execute(
        "UPDATE meta_store_generation SET generation = randomblob(16) WHERE id = 0",
        [],
    )?;
    Ok(())
}

fn generation_table_exists(conn: &rusqlite::Connection) -> Result<bool, WalletStorageError> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta_store_generation'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
