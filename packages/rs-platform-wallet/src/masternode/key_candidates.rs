//! Fresh-key candidates for a registrar update: the wallet's provider keys
//! by index, each joined against the masternode list so "unused" means
//! unused network-wide — for operator keys that is a consensus requirement
//! (they are unique across the list), for voting keys a courtesy default.
//!
//! Public-only derivation from the account xpubs; no seed is touched, which
//! is also why only the secp/BLS families are supported here — Ed25519
//! platform-node keys derive hardened and would need the seed.

use dashcore::hashes::{hash160, Hash};

/// Upper bound on one candidates query. Far above any realistic provider
/// pool, and small enough that `count` can never drive an allocation
/// failure — the FFI re-exports the same value for hosts.
pub const MAX_PROVIDER_KEY_CANDIDATES: u32 = 256;

use super::list::MasternodeListSummary;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::PlatformWallet;
use crate::wallet::provider_key_at_index::ProviderKeyKind;

/// One wallet provider key, with its network-wide usage.
#[derive(Debug, Clone)]
pub struct ProviderKeyCandidate {
    /// Index within the provider pool.
    pub index: u32,
    /// Modern-serialization public key bytes: 48 for a BLS operator key,
    /// 33 for a compressed secp voting key.
    pub public_key_bytes: Vec<u8>,
    /// P2PKH address (voting keys only — BLS keys have no address form).
    pub address: Option<String>,
    /// proTxHash (wire order) of the masternode-list entry currently using
    /// this key, when one does.
    pub used_by: Option<[u8; 32]>,
}

/// Derive the first `count` keys of `kind` and join each against the list.
/// Supports [`ProviderKeyKind::Operator`] (matched against entry operator
/// keys under both serializations) and [`ProviderKeyKind::Voting`] (hash160
/// matched against entry voting key ids); other kinds are refused — owner
/// keys are immutable and never candidates, platform-node keys need the
/// seed.
pub fn provider_key_candidates(
    wallet: &PlatformWallet,
    summaries: &[MasternodeListSummary],
    kind: ProviderKeyKind,
    count: u32,
) -> Result<Vec<ProviderKeyCandidate>, PlatformWalletError> {
    match kind {
        ProviderKeyKind::Operator | ProviderKeyKind::Voting => {}
        _ => {
            return Err(PlatformWalletError::InvalidParameter(
                "key candidates are available for operator and voting keys only".to_string(),
            ));
        }
    }
    if count > MAX_PROVIDER_KEY_CANDIDATES {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "at most {MAX_PROVIDER_KEY_CANDIDATES} key candidates can be listed per call, \
             {count} were requested"
        )));
    }

    let mut candidates = Vec::with_capacity(count as usize);
    for index in 0..count {
        let derived = wallet.derive_provider_key_at_index(kind, index, None, false)?;
        let used_by = match kind {
            ProviderKeyKind::Operator => {
                let modern: Option<[u8; 48]> = derived.public_key_bytes.as_slice().try_into().ok();
                let legacy: Option<[u8; 48]> = derived
                    .legacy_public_key_bytes
                    .as_deref()
                    .and_then(|b| b.try_into().ok());
                summaries
                    .iter()
                    .find(|entry| {
                        modern.is_some_and(|k| entry.operator_public_key == k)
                            || legacy.is_some_and(|k| entry.operator_public_key == k)
                    })
                    .map(|entry| entry.pro_tx_hash)
            }
            ProviderKeyKind::Voting => {
                let key_id = hash160::Hash::hash(&derived.public_key_bytes).to_byte_array();
                summaries
                    .iter()
                    .find(|entry| entry.voting_key_id == key_id)
                    .map(|entry| entry.pro_tx_hash)
            }
            _ => unreachable!("kind validated above"),
        };
        candidates.push(ProviderKeyCandidate {
            index,
            public_key_bytes: derived.public_key_bytes,
            address: derived.address,
            used_by,
        });
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::super::list::test_support::masternode;
    use super::super::locator::bls_public_keys;
    use super::*;
    use crate::test_support::test_platform_wallet_manager;
    use crate::wallet::platform_wallet::PlatformWallet;
    use std::sync::Arc;

    /// Derivation and the wallet-manager reads are blocking, so the async
    /// setup runs on its own runtime and the candidates query runs outside
    /// it — the same threading shape the FFI worker gives it.
    fn test_wallet() -> Arc<PlatformWallet> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = manager
            .get_wallet_blocking(&wallet_id)
            .expect("test wallet");
        // Keep the manager alive alongside the wallet handle.
        std::mem::forget(manager);
        wallet
    }

    #[test]
    fn candidates_join_operator_keys_under_both_serializations() {
        let wallet = test_wallet();

        let derived0 = wallet
            .derive_provider_key_at_index(ProviderKeyKind::Operator, 0, None, false)
            .expect("operator key 0");
        let modern0: [u8; 48] = derived0.public_key_bytes.as_slice().try_into().expect("48");
        let legacy1: [u8; 48] = wallet
            .derive_provider_key_at_index(ProviderKeyKind::Operator, 1, None, false)
            .expect("operator key 1")
            .legacy_public_key_bytes
            .expect("operator keys carry a legacy form")
            .as_slice()
            .try_into()
            .expect("48");

        let mut used_modern = masternode(0x11);
        used_modern.operator_public_key = modern0;
        let mut used_legacy = masternode(0x22);
        used_legacy.operator_public_key = legacy1;
        let summaries = vec![used_modern, used_legacy];

        let candidates = provider_key_candidates(&wallet, &summaries, ProviderKeyKind::Operator, 3)
            .expect("candidates");
        assert_eq!(candidates.len(), 3);
        assert_eq!(
            candidates[0].used_by,
            Some([0x11; 32]),
            "modern-serialization usage is joined"
        );
        assert_eq!(
            candidates[1].used_by,
            Some([0x22; 32]),
            "legacy-serialization usage is joined"
        );
        assert_eq!(candidates[2].used_by, None, "an unused key stays eligible");
        assert!(
            candidates[0].address.is_none(),
            "BLS keys have no address form"
        );
        assert_eq!(candidates[0].public_key_bytes.len(), 48);
    }

    #[test]
    fn candidates_join_voting_keys_by_key_id() {
        let wallet = test_wallet();

        let derived = wallet
            .derive_provider_key_at_index(ProviderKeyKind::Voting, 0, None, false)
            .expect("voting key 0");
        let key_id = hash160::Hash::hash(&derived.public_key_bytes).to_byte_array();

        let mut used = masternode(0x33);
        used.voting_key_id = key_id;
        let summaries = vec![used];

        let candidates = provider_key_candidates(&wallet, &summaries, ProviderKeyKind::Voting, 2)
            .expect("candidates");
        assert_eq!(candidates[0].used_by, Some([0x33; 32]));
        assert_eq!(candidates[1].used_by, None);
        assert!(
            candidates[0].address.is_some(),
            "voting keys carry their P2PKH address for display"
        );
        assert_eq!(candidates[0].public_key_bytes.len(), 33);
    }

    #[test]
    fn unsupported_kinds_zero_counts_and_oversized_counts_are_handled() {
        let wallet = test_wallet();
        let summaries = vec![masternode(0x44)];

        for kind in [ProviderKeyKind::Owner, ProviderKeyKind::PlatformNode] {
            let err = provider_key_candidates(&wallet, &summaries, kind, 1)
                .expect_err("owner / platform-node kinds are refused");
            assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
        }

        let empty = provider_key_candidates(&wallet, &summaries, ProviderKeyKind::Operator, 0)
            .expect("zero count is an empty listing");
        assert!(empty.is_empty());

        let err = provider_key_candidates(
            &wallet,
            &summaries,
            ProviderKeyKind::Operator,
            MAX_PROVIDER_KEY_CANDIDATES + 1,
        )
        .expect_err("counts above the bound are refused before any allocation");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // Sanity: a valid operator secret's own serializations round
        // through the same join the picker uses.
        let (basic, legacy) = bls_public_keys(&[7u8; 32]).expect("valid scalar");
        assert_ne!(basic, legacy, "the two serializations differ in flag bits");
    }
}
