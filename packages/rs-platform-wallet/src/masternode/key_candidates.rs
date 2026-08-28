//! Fresh-key candidates for a registrar update: the wallet's provider keys
//! by index, each joined against the masternode list so "unused" means
//! unused network-wide — for operator keys that is a consensus requirement
//! (they are unique across the list), for voting keys a courtesy default.
//!
//! Public-only derivation from the account xpubs; no seed is touched, which
//! is also why only the secp/BLS families are supported here — Ed25519
//! platform-node keys derive hardened and would need the seed.

use dashcore::hashes::{hash160, Hash};

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
