//! Masternodes / evonodes as the wallet layer sees them.
//!
//! * [`record`] — the pure aggregation model ([`MasternodeRecord`]) and the
//!   DML-membership → status mapping.
//! * [`PlatformWalletManager::wallet_masternodes_blocking`] — the one
//!   library entry point that lists a wallet's masternodes: aggregation,
//!   status against the DML snapshot, and derive-and-compare ownership of
//!   the operator / platform-node keys. Both FFI crates and the withdrawal
//!   path read through it, so every host renders the same records.

pub mod record;

pub use record::{
    aggregate_masternodes, provider_payload_fields, ListMembership, MasternodeRecord,
    MasternodeSource, MasternodeStatus, ProviderPayloadFields,
};

use crate::changeset::PlatformWalletPersistence;
use crate::manager::PlatformWalletManager;
use crate::wallet::platform_wallet::WalletId;

/// A wallet's masternodes plus the network they belong to (needed to
/// encode key hashes as base58 addresses at the host boundary).
#[derive(Debug, Clone)]
pub struct WalletMasternodes {
    pub network: dashcore::Network,
    /// Sorted by registration order; `order_index` is each record's
    /// position in this vec.
    pub records: Vec<MasternodeRecord>,
}

impl WalletMasternodes {
    /// The record for `pro_tx_hash` (wire order), if this wallet has one.
    pub fn find(&self, pro_tx_hash: &[u8; 32]) -> Option<&MasternodeRecord> {
        self.records
            .iter()
            .find(|mn| &mn.pro_tx_hash == pro_tx_hash)
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// List the wallet's masternodes: aggregate its retained provider
    /// special transactions (see
    /// [`Self::provider_masternode_txs_blocking`]), resolve each record's
    /// status against the current DML snapshot (`None` ⇒ `Unknown`, so a
    /// persister keeps its prior value), and resolve operator / platform
    /// key ownership by derive-and-compare. Owner / voting ownership is
    /// NOT resolved here — those keys are on-chain addresses and hosts
    /// join them against their persisted address rows.
    ///
    /// Returns `None` when the wallet isn't loaded. Blocking (reads the
    /// wallet-manager, SPV client and engine locks via `blocking_read`) —
    /// call from a blocking thread, never from the async runtime.
    pub fn wallet_masternodes_blocking(&self, wallet_id: &WalletId) -> Option<WalletMasternodes> {
        let (network, txs, dml, operator_index, platform_index) =
            self.provider_masternode_txs_blocking(wallet_id)?;

        let membership = |pro_tx_hash: &[u8; 32]| -> ListMembership {
            match &dml {
                None => ListMembership::ListUnavailable,
                Some(map) => match map.get(pro_tx_hash) {
                    Some(true) => ListMembership::ValidEntry,
                    Some(false) => ListMembership::InvalidEntry,
                    None => ListMembership::Absent,
                },
            }
        };

        let mut records =
            aggregate_masternodes(txs.iter().map(|(h, p, tx)| (*h, *p, tx)), membership);
        // The check was possible iff the wallet's derived platform-node index
        // had entries to compare against. Empty index ⇒ no platform pool / not
        // yet rehydrated ⇒ ownership is "unchecked", and a persister must
        // retain any prior value rather than clobber it.
        let platform_ownership_checked = !platform_index.is_empty();
        for (idx, mn) in records.iter_mut().enumerate() {
            mn.order_index = idx as u32;
            mn.source = MasternodeSource::Wallet;
            mn.operator_key_index = mn
                .operator_public_key
                .and_then(|k| operator_index.get(&k).copied());
            mn.platform_key_index = mn
                .platform_node_id
                .and_then(|id| platform_index.get(&id).copied());
            mn.platform_ownership_checked = platform_ownership_checked;
        }

        Some(WalletMasternodes { network, records })
    }
}
