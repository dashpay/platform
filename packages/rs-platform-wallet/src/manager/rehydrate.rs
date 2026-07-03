//! Watch-only wallet reconstruction from the keyless account manifest.
//!
//! Load is **seedless** (see [`load_from_persistor`]). For each
//! persisted wallet we build a watch-only [`Wallet`] from its keyless
//! `AccountRegistrationEntry` manifest; the manager then consumes the
//! carried [`ManagedWalletInfo`](key_wallet::wallet::managed_wallet_info::ManagedWalletInfo)
//! snapshot directly. No seed, no signing-key derivation.
//!
//! Because load never touches the seed, it performs no wrong-seed check.
//! Wrong-seed validation lives in the resolver-backed signing
//! entrypoints (`sign_with_mnemonic_resolver` and the FFI resolver sign
//! path), which fail-closed gate the resolver-supplied seed against the
//! loaded `wallet_id`; the seedless load path here never sees the seed.
//!
//! [`load_from_persistor`]: super::PlatformWalletManager::load_from_persistor

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::Account;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::changeset::AccountRegistrationEntry;
use crate::manager::load_outcome::CorruptKind;

/// Build a watch-only [`Wallet`] from the keyless account manifest.
///
/// Each `AccountRegistrationEntry` becomes an [`Account::from_xpub`]
/// (watch-only) keyed to `expected_wallet_id`; the assembled
/// [`AccountCollection`] is handed to [`Wallet::new_watch_only`] under
/// the same id. No key material crosses this function.
///
/// Returns [`CorruptKind`] when the row is structurally unusable
/// (caller wraps it in a per-row [`SkipReason`]).
///
/// [`SkipReason`]: crate::manager::load_outcome::SkipReason
///
/// # Trust boundary
///
/// `expected_wallet_id` is stamped onto the reconstructed [`Wallet`]
/// verbatim and is **not** cryptographically bound to the manifest: the
/// id hashes the *root* xpub, but only account-level (hardened, one-way)
/// xpubs are persisted, so the root cannot be recovered here to re-derive
/// and verify the id. Only a structural decode runs, so a well-formed but
/// **wrong** `account_xpub` is accepted.
///
/// Concretely, the attack this leaves open: an attacker who can write to
/// the backing store (or a malicious/rolled-back backup restored into it)
/// substitutes a valid xpub of their own for a wallet's `account_xpub`,
/// leaving `expected_wallet_id` unchanged. The wallet is rebuilt under the
/// original id but now derives its receive addresses from the attacker's
/// key, so future incoming funds are silently redirected — the id looks
/// unchanged to the user while the money flows elsewhere. This crate
/// **does not** defend against it: closing the gap requires the storage
/// layer to authenticate the manifest (a persisted commitment/MAC over
/// `{wallet_id, network, manifest}`, verified fail-closed on load), which
/// is a storage-schema change tracked in the `platform-wallet-storage`
/// crate. See the trust-boundary note on
/// [`PlatformWalletPersistence::load`](crate::changeset::PlatformWalletPersistence::load).
pub(super) fn build_watch_only_wallet(
    network: Network,
    expected_wallet_id: [u8; 32],
    manifest: &[AccountRegistrationEntry],
) -> Result<Wallet, CorruptKind> {
    if manifest.is_empty() {
        return Err(CorruptKind::MissingManifest);
    }
    let mut accounts = AccountCollection::new();
    for entry in manifest {
        // NOTE: `Account::from_xpub` is infallible in the pinned key-wallet rev
        // (unconditional `Ok`); this map_err is a defensive guard for when its
        // signature becomes fallible (e.g. xpub/type validation).
        let account = Account::from_xpub(
            Some(expected_wallet_id),
            entry.account_type,
            entry.account_xpub,
            network,
        )
        .map_err(|_| CorruptKind::MalformedXpub)?;
        accounts
            .insert(account)
            .map_err(|e| CorruptKind::DecodeError(e.to_string()))?;
    }
    Ok(Wallet::new_watch_only(
        network,
        expected_wallet_id,
        accounts,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    fn manifest_for(w: &Wallet) -> Vec<AccountRegistrationEntry> {
        w.accounts
            .all_accounts()
            .into_iter()
            .map(|a| AccountRegistrationEntry {
                account_type: a.account_type,
                account_xpub: a.account_xpub,
            })
            .collect()
    }

    #[test]
    fn watch_only_rebuild_round_trips_manifest_and_id() {
        let seed = [3u8; 64];
        let w = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let id = w.compute_wallet_id();
        let manifest = manifest_for(&w);

        let restored = build_watch_only_wallet(Network::Testnet, id, &manifest).unwrap();
        assert_eq!(restored.wallet_id, id);
        assert_eq!(restored.compute_wallet_id(), id);
        // Every manifest account survives the round trip (count, types).
        let restored_types: Vec<_> = restored
            .accounts
            .all_accounts()
            .into_iter()
            .map(|a| a.account_type)
            .collect();
        let manifest_types: Vec<_> = manifest.iter().map(|e| e.account_type).collect();
        assert_eq!(restored_types.len(), manifest_types.len());
        for t in &manifest_types {
            assert!(restored_types.contains(t));
        }
    }

    #[test]
    fn empty_manifest_is_missing_manifest() {
        let err = build_watch_only_wallet(Network::Testnet, [0u8; 32], &[])
            .expect_err("empty manifest must be MissingManifest");
        assert!(matches!(err, CorruptKind::MissingManifest));
    }
}
