//! Masternodes / evonodes as the wallet layer sees them.
//!
//! * [`record`] — the pure aggregation model ([`MasternodeRecord`]) and the
//!   DML-membership → status mapping.
//! * [`PlatformWalletManager::wallet_masternodes_blocking`] — the one
//!   library entry point that lists a wallet's masternodes: aggregation,
//!   status against the DML snapshot, and derive-and-compare ownership of
//!   the operator / platform-node keys. Both FFI crates and the withdrawal
//!   path read through it, so every host renders the same records.

pub mod key_candidates;
pub mod list;
pub mod locator;
pub mod record;
pub mod tracked;
pub mod update_registrar;
pub mod update_service;

pub use key_candidates::{provider_key_candidates, ProviderKeyCandidate};
pub use list::{find_in_summaries, MasternodeListQuery, MasternodeListSummary};
pub use locator::{
    locate_in_summaries, parse_locator_input, parse_secret_for_role, verify_masternode_key,
    verify_masternode_key_text, KeyVerification, LocateOptions, LocatorMatchKind,
    LocatorParseError, LocatorSecret, MasternodeKeyReference, MasternodeKeyRole,
    MasternodeLocateError, MasternodeLocateMatch, MasternodeLocateResult, MasternodeLocator,
    MasternodeLocatorInput, ParsedLocatorInput, PlatformLookup,
};
pub use record::{
    aggregate_masternodes, provider_payload_fields, ListMembership, MasternodeRecord,
    MasternodeSource, MasternodeStatus, ProviderPayloadFields,
};
pub use tracked::{
    capabilities_for_roles, snapshot_from_json, snapshot_to_json, MasternodeCapabilities,
    PlatformKeySnapshot, RegistrationDetails, TrackedMasternode, TrackedMasternodeSnapshot,
};
pub use update_registrar::{
    execute_masternode_update_registrar, prepare_masternode_update_registrar,
    MasternodeUpdateRegistrarParams, OwnerSecret,
};
pub use update_service::{
    execute_masternode_update_service, execute_masternode_update_service_with_values,
    prepare_masternode_update_service, prepare_masternode_update_service_with_values,
    MasternodeUpdateServiceParams, UpdateServiceValues,
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

    /// proTxHash (wire) ⇒ wallet id for every loaded wallet's masternodes —
    /// the "already in wallet" index the locator marks matches with.
    /// Blocking (see [`Self::wallet_masternodes_blocking`]).
    pub fn wallet_masternode_index_blocking(
        &self,
    ) -> std::collections::HashMap<[u8; 32], WalletId> {
        let mut index = std::collections::HashMap::new();
        for wallet_id in self.list_wallet_ids_blocking() {
            if let Some(masternodes) = self.wallet_masternodes_blocking(&wallet_id) {
                for record in masternodes.records {
                    index.entry(record.pro_tx_hash).or_insert(wallet_id);
                }
            }
        }
        index
    }

    /// Snapshot everything a locate needs (SPV, SDK, network, the wallets'
    /// own masternodes) into a `Send + Sync` [`MasternodeLocator`] that can
    /// run on a worker without holding the manager. Blocking.
    pub fn masternode_locator_blocking(&self) -> MasternodeLocator {
        MasternodeLocator {
            spv: self.spv_arc(),
            sdk: self.sdk_arc(),
            network: self.sdk().network,
            in_wallet: self.wallet_masternode_index_blocking(),
            tracked: self.tracked_masternode_hashes(),
        }
    }

    /// The key references known for `pro_tx_hash` (wire order): the DML
    /// summary (voting / operator / platform node) merged with the owning
    /// wallet's record (owner / payout) when it is one of a loaded wallet's
    /// masternodes, and with the tracked-registry snapshot (owner / payout
    /// from Platform, registration keys) when it is tracked. `None` when
    /// nobody knows it. Blocking.
    pub fn masternode_key_reference_blocking(
        &self,
        pro_tx_hash: &[u8; 32],
    ) -> Option<MasternodeKeyReference> {
        let from_list = self
            .spv()
            .masternode_list_summaries_blocking()
            .and_then(|summaries| {
                summaries
                    .iter()
                    .find(|s| &s.pro_tx_hash == pro_tx_hash)
                    .map(MasternodeKeyReference::from_summary)
            });
        let from_wallet = self
            .list_wallet_ids_blocking()
            .into_iter()
            .find_map(|wallet_id| {
                self.wallet_masternodes_blocking(&wallet_id)?
                    .find(pro_tx_hash)
                    .map(MasternodeKeyReference::from_record)
            });
        let from_tracked = self
            .tracked_masternode(pro_tx_hash)
            .map(|tracked| tracked.key_reference());
        [from_list, from_wallet, from_tracked]
            .into_iter()
            .flatten()
            .reduce(|merged, next| merged.merged_with(&next))
    }
}
