//! Watch-only wallet reconstruction from the keyless account manifest.
//!
//! Load is seedless — each wallet is rebuilt watch-only from its manifest and
//! the manager consumes the carried snapshot directly, so no wrong-seed check
//! runs here; that gate lives in the resolver-backed signing entrypoints.

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::Account;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::changeset::AccountRegistrationEntry;
use crate::manager::load_outcome::CorruptKind;

/// Build a watch-only [`Wallet`] from the keyless account manifest, stamping
/// `expected_wallet_id` onto the reconstructed [`AccountCollection`]. Returns
/// [`CorruptKind`] when the row is structurally unusable; no key material
/// crosses this function.
///
/// # Trust boundary
///
/// `expected_wallet_id` is **not** cryptographically bound to the manifest: the
/// id hashes the *root* xpub, but only account-level xpubs are persisted, so the
/// root cannot be recovered here to re-verify it. A well-formed but **wrong**
/// `account_xpub` is therefore accepted — anyone able to write the backing store
/// can swap in their own xpub under the unchanged id and silently redirect
/// incoming funds. Closing this needs storage-layer manifest authentication (a
/// MAC over `{wallet_id, network, manifest}`, verified fail-closed on load),
/// tracked in the `platform-wallet-storage` crate.
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
        // `Account::from_xpub` is infallible in the pinned key-wallet rev; this
        // map_err is a defensive guard for when that signature becomes fallible.
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
