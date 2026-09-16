use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform_state::masternode_list_changes::MasternodeListChanges;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSet;
use dpp::block::block_info::{BlockInfo, DEFAULT_BLOCK_INFO};
use dpp::block::epoch::{Epoch, EPOCH_0};
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::core_types::validator_set::v0::ValidatorSetV0Getters;
use dpp::core_types::validator_set::ValidatorSet;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNStateDiff, MasternodeListItem, MasternodeType};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Platform state methods introduced in version 0 of Platform State Struct
pub trait PlatformStateV0Methods {
    /// The last block height or 0 for genesis
    fn last_committed_block_height(&self) -> u64;
    /// The height of the platform, only committed blocks increase height
    fn last_committed_known_block_height_or(&self, default: u64) -> u64;
    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_core_height(&self) -> u32;
    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_known_core_height_or(&self, default: u32) -> u32;
    /// The last block time in milliseconds
    fn last_committed_block_time_ms(&self) -> Option<u64>;
    /// The last quorum hash
    fn last_committed_quorum_hash(&self) -> [u8; 32];
    /// The last block proposer pro tx hash
    fn last_committed_block_proposer_pro_tx_hash(&self) -> [u8; 32];
    /// The last block signature
    fn last_committed_block_signature(&self) -> [u8; 96];
    /// The last block app hash
    fn last_committed_block_app_hash(&self) -> Option<[u8; 32]>;
    /// The last block round
    fn last_committed_block_round(&self) -> u32;
    /// The current epoch
    fn last_committed_block_epoch(&self) -> Epoch;
    /// HPMN list len
    fn hpmn_list_len(&self) -> usize;
    /// Get the current quorum
    fn current_validator_set(&self) -> Result<&ValidatorSet, Error>;
    /// Returns information about the last committed block.
    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo>;
    /// Returns the current protocol version that is in consensus.
    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion;
    /// Get the current platform version
    fn current_platform_version(&self) -> Result<&'static PlatformVersion, Error> {
        PlatformVersion::get(self.current_protocol_version_in_consensus()).map_err(Error::from)
    }
    /// Returns the upcoming protocol version for the next epoch.
    fn next_epoch_protocol_version(&self) -> ProtocolVersion;

    /// Returns the quorum hash of the current validator set.
    fn current_validator_set_quorum_hash(&self) -> QuorumHash;

    /// Get validator sets sorted by their core height by most recent order coming first
    fn validator_sets_sorted_by_core_height_by_most_recent(&self) -> Vec<&ValidatorSet> {
        // Get the validator sets and collect them into a vector for sorting
        let mut validator_sets: Vec<&ValidatorSet> = self.validator_sets().values().collect();

        // Sort the validator sets by core height in descending order
        validator_sets.sort_by_key(|b| std::cmp::Reverse(b.core_height()));

        validator_sets
    }

    /// Where is the current validator set in the list
    fn current_validator_set_position_in_list_by_most_recent(&self) -> Option<u16> {
        // Get the current quorum hash
        let current_quorum_hash = self.current_validator_set_quorum_hash();

        // Get the validator sets by post recent
        let validator_sets = self.validator_sets_sorted_by_core_height_by_most_recent();

        // Find the position of the current validator set in the sorted list
        validator_sets
            .iter()
            .position(|&validator_set| validator_set.quorum_hash() == &current_quorum_hash)
            .map(|position| position as u16) // Convert position to u16
    }

    /// Returns the quorum hash of the next validator set, if it exists.
    fn next_validator_set_quorum_hash(&self) -> &Option<QuorumHash>;

    /// Returns the quorum hash of the next validator set, if it exists and replaces current value with none.
    fn take_next_validator_set_quorum_hash(&mut self) -> Option<QuorumHash>;

    /// Returns the current validator sets.
    fn validator_sets(&self) -> &IndexMap<QuorumHash, ValidatorSet>;

    /// Returns the quorums used to validate chain locks.
    fn chain_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet;

    /// Returns quorums used to validate instant locks.
    fn instant_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet;

    /// Returns the full list of masternodes.
    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns the list of high performance masternodes.
    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns information about the platform initialization state, if it exists.
    fn genesis_block_info(&self) -> Option<&BlockInfo>;

    /// Returns the last committed block info if present or the genesis block info if not or default one
    fn last_block_info(&self) -> &BlockInfo;

    /// Sets the last committed block info.
    fn set_last_committed_block_info(&mut self, info: Option<ExtendedBlockInfo>);

    /// Sets the current protocol version in consensus.
    fn set_current_protocol_version_in_consensus(&mut self, version: ProtocolVersion);

    /// Sets the next epoch protocol version.
    fn set_next_epoch_protocol_version(&mut self, version: ProtocolVersion);

    /// Sets the current validator set quorum hash.
    fn set_current_validator_set_quorum_hash(&mut self, hash: QuorumHash);

    /// Sets the next validator set quorum hash.
    fn set_next_validator_set_quorum_hash(&mut self, hash: Option<QuorumHash>);

    /// Sets the current validator sets.
    fn set_validator_sets(&mut self, sets: IndexMap<QuorumHash, ValidatorSet>);

    /// Sets the current chain lock validating quorums.
    fn set_chain_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet);

    /// Sets the current instant lock validating quorums.
    fn set_instant_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet);

    /// Sets the full masternode list.
    fn set_full_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>);

    /// Sets the list of high performance masternodes.
    fn set_hpmn_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>);
    /// Sets the platform initialization information.
    fn set_genesis_block_info(&mut self, info: Option<BlockInfo>);

    /// Returns a mutable reference to the last committed block info.
    fn last_committed_block_info_mut(&mut self) -> &mut Option<ExtendedBlockInfo>;

    /// Returns a mutable reference to the current protocol version in consensus.
    fn current_protocol_version_in_consensus_mut(&mut self) -> &mut ProtocolVersion;

    /// Returns a mutable reference to the next epoch protocol version.
    fn next_epoch_protocol_version_mut(&mut self) -> &mut ProtocolVersion;

    /// Returns a mutable reference to the current validator set quorum hash.
    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash;

    /// Returns a mutable reference to the next validator set quorum hash.
    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash>;

    /// Returns a mutable reference to the current validator sets.
    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet>;

    /// Returns a mutable reference to the chain lock validating quorums.
    fn chain_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet;

    /// Returns a mutable reference to the instant lock validating quorums.
    fn instant_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet;

    /// Returns a mutable reference to the full masternode list.
    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns a mutable reference to the list of high performance masternodes.
    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem>;

    /// Adds or replaces a masternode in the full list, and in the HPMN list when
    /// it is an Evo node. Records the change for the per-entry store.
    fn insert_masternode(&mut self, masternode: MasternodeListItem);

    /// Applies a Core state diff to a listed masternode in both lists. Returns
    /// false, changing nothing, when the masternode is not listed.
    fn apply_masternode_state_diff(
        &mut self,
        pro_tx_hash: &ProTxHash,
        state_diff: &DMNStateDiff,
    ) -> bool;

    /// Removes a masternode from both lists.
    fn remove_masternode(&mut self, pro_tx_hash: &ProTxHash) -> Option<MasternodeListItem>;

    /// Empties both masternode lists.
    fn clear_masternode_lists(&mut self);

    /// One validator set for mutation, recorded for the per-entry store.
    fn validator_set_mut(&mut self, quorum_hash: &QuorumHash) -> Option<&mut ValidatorSet>;

    /// Adds or replaces a validator set, appended at the end of the order.
    fn insert_validator_set(&mut self, quorum_hash: QuorumHash, validator_set: ValidatorSet);

    /// Removes a validator set, keeping the order of the rest.
    fn remove_validator_set(&mut self, quorum_hash: &QuorumHash) -> Option<ValidatorSet>;

    /// Reorders the validator sets. The order is saved with the record, so this
    /// does not rewrite any entry.
    fn sort_validator_sets_by(
        &mut self,
        compare: &mut dyn FnMut(&ValidatorSet, &ValidatorSet) -> Ordering,
    );

    /// The epoch ref
    fn last_committed_block_epoch_ref(&self) -> &Epoch;
    /// The last block id hash
    fn last_committed_block_id_hash(&self) -> [u8; 32];

    /// Returns reference to the previous feeversions
    fn previous_fee_versions(&self) -> &CachedEpochIndexFeeVersions;

    /// Returns a mutable reference to the previous feeversions
    fn previous_fee_versions_mut(&mut self) -> &mut CachedEpochIndexFeeVersions;

    /// The changes in the full masternode list between two platform states
    fn full_masternode_list_changes(&self, previous: &Self) -> MasternodeListChanges
    where
        Self: Sized;

    /// The changes in the high performance masternode list (evonodes) between two platform states
    fn hpmn_masternode_list_changes(&self, previous: &Self) -> MasternodeListChanges
    where
        Self: Sized;

    /// The size of the hpmn list that are currently not banned
    fn hpmn_active_list_len(&self) -> usize;
}

impl PlatformStateV0Methods for PlatformState {
    /// The last block height or 0 for genesis
    fn last_committed_block_height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or_default()
    }

    /// The height of the platform, only committed blocks increase height
    fn last_committed_known_block_height_or(&self, default: u64) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or(default)
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_core_height(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().core_height)
            .unwrap_or_else(|| {
                self.genesis_block_info
                    .as_ref()
                    .map(|initialization_information| initialization_information.core_height)
                    .unwrap_or_default()
            })
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_known_core_height_or(&self, default: u32) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().core_height)
            .unwrap_or_else(|| {
                self.genesis_block_info
                    .as_ref()
                    .map(|block_info| block_info.core_height)
                    .unwrap_or(default)
            })
    }

    /// The last block time in milliseconds
    fn last_committed_block_time_ms(&self) -> Option<u64> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().time_ms)
    }

    /// The last quorum hash
    fn last_committed_quorum_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.quorum_hash())
            .unwrap_or_default()
    }

    /// The last committed block proposer's pro tx hash
    fn last_committed_block_proposer_pro_tx_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.proposer_pro_tx_hash())
            .unwrap_or_default()
    }

    /// The last block signature
    fn last_committed_block_signature(&self) -> [u8; 96] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.signature())
            .unwrap_or([0u8; 96])
    }

    /// The last block app hash
    fn last_committed_block_app_hash(&self) -> Option<[u8; 32]> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.app_hash())
    }

    /// The last block round
    fn last_committed_block_round(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.round())
            .unwrap_or_default()
    }

    /// The current epoch
    fn last_committed_block_epoch(&self) -> Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().epoch)
            .unwrap_or_default()
    }

    /// HPMN list len
    fn hpmn_list_len(&self) -> usize {
        self.hpmn_masternode_list.len()
    }

    /// HPMN active list len
    fn hpmn_active_list_len(&self) -> usize {
        self.hpmn_masternode_list
            .values()
            .filter(|masternode| masternode.state.pose_ban_height.is_none())
            .count()
    }

    /// Get the current quorum
    fn current_validator_set(&self) -> Result<&ValidatorSet, Error> {
        self.validator_sets
            .get(&self.current_validator_set_quorum_hash)
            .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                format!("current_validator_set: current validator quorum hash {} not in current known validator sets {} last committed block is {} (we might be processing new block)", self.current_validator_set_quorum_hash, self.validator_sets.keys().map(|quorum_hash| quorum_hash.to_string()).join(" | "),
                        self.last_committed_block_info.as_ref().map(|block_info| block_info.basic_info().height).unwrap_or_default()),
            )))
    }

    /// Returns information about the last committed block.
    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo> {
        &self.last_committed_block_info
    }

    /// Get the current protocol version in consensus
    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion {
        self.current_protocol_version_in_consensus
    }

    /// Returns the upcoming protocol version for the next epoch.
    fn next_epoch_protocol_version(&self) -> ProtocolVersion {
        self.next_epoch_protocol_version
    }

    /// Returns the quorum hash of the current validator set.
    fn current_validator_set_quorum_hash(&self) -> QuorumHash {
        self.current_validator_set_quorum_hash
    }

    /// Returns the quorum hash of the next validator set, if it exists.
    fn next_validator_set_quorum_hash(&self) -> &Option<QuorumHash> {
        &self.next_validator_set_quorum_hash
    }

    /// Returns the quorum hash of the next validator set, if it exists, and replaces current value with None.
    fn take_next_validator_set_quorum_hash(&mut self) -> Option<QuorumHash> {
        self.next_validator_set_quorum_hash.take()
    }

    /// Returns the current validator sets.
    fn validator_sets(&self) -> &IndexMap<QuorumHash, ValidatorSet> {
        &self.validator_sets
    }

    /// Returns the quorums used to validate chain locks.
    fn chain_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet {
        &self.chain_lock_validating_quorums
    }

    /// Returns the quorums used to validate instant locks.
    fn instant_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet {
        &self.instant_lock_validating_quorums
    }

    /// Returns the full list of masternodes.
    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        &self.full_masternode_list
    }

    /// Returns the list of high performance masternodes.
    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        &self.hpmn_masternode_list
    }

    /// Returns information about the platform initialization state, if it exists.
    fn genesis_block_info(&self) -> Option<&BlockInfo> {
        self.genesis_block_info.as_ref()
    }

    fn last_block_info(&self) -> &BlockInfo {
        self.last_committed_block_info
            .as_ref()
            .map(|b| b.basic_info())
            .unwrap_or_else(|| {
                self.genesis_block_info
                    .as_ref()
                    .unwrap_or(&DEFAULT_BLOCK_INFO)
            })
    }

    /// Sets the last committed block info.
    fn set_last_committed_block_info(&mut self, info: Option<ExtendedBlockInfo>) {
        self.last_committed_block_info = info;
    }

    /// Sets the current protocol version in consensus.
    fn set_current_protocol_version_in_consensus(&mut self, version: ProtocolVersion) {
        self.current_protocol_version_in_consensus = version;
        // The protocol version chooses the structure the full record is written
        // in, so a change has to rewrite it rather than leave an older structure
        // on disk with a newer version recorded beside it. A structure that keeps
        // the collections as entries needs every entry written the first time.
        self.mark_all_unsaved();
    }

    /// Sets the next epoch protocol version.
    fn set_next_epoch_protocol_version(&mut self, version: ProtocolVersion) {
        self.next_epoch_protocol_version = version;
        // Carried only by the full record: it moves once per epoch at most, so
        // rewriting the record then is cheaper than writing it every block.
        self.heavy_fields_dirty = true;
    }

    /// Sets the current validator set quorum hash.
    fn set_current_validator_set_quorum_hash(&mut self, hash: QuorumHash) {
        self.current_validator_set_quorum_hash = hash;
    }

    /// Sets the next validator set quorum hash.
    fn set_next_validator_set_quorum_hash(&mut self, hash: Option<QuorumHash>) {
        self.next_validator_set_quorum_hash = hash;
    }

    /// Sets the current validator sets.
    fn set_validator_sets(&mut self, sets: IndexMap<QuorumHash, ValidatorSet>) {
        self.validator_sets = sets;
        self.heavy_fields_dirty = true;
        self.validator_set_changes.mark_rewrite_all();
    }

    /// Sets the current chain lock validating quorums.
    fn set_chain_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet) {
        self.chain_lock_validating_quorums = quorums;
        self.heavy_fields_dirty = true;
    }

    /// Sets the current instant lock validating quorums.
    fn set_instant_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet) {
        self.instant_lock_validating_quorums = quorums;
        self.heavy_fields_dirty = true;
    }

    /// Sets the full masternode list.
    fn set_full_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>) {
        self.full_masternode_list = list;
        self.heavy_fields_dirty = true;
        self.masternode_changes.mark_rewrite_all();
    }

    /// Sets the list of high performance masternodes.
    fn set_hpmn_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>) {
        self.hpmn_masternode_list = list;
        self.heavy_fields_dirty = true;
        // The HPMN list is not stored (it is the Evo subset of the full list),
        // but a caller replacing it wholesale may have changed the full list too.
        self.masternode_changes.mark_rewrite_all();
    }

    /// Sets the platform initialization information.
    fn set_genesis_block_info(&mut self, info: Option<BlockInfo>) {
        // Carried only by the full record, yet deliberately not dirtying: the
        // block end clears it before every store, so a state that has been
        // stored at all is on disk with `None`, and marking the clear dirty
        // would rewrite the full record on every block.
        self.genesis_block_info = info;
    }

    fn last_committed_block_info_mut(&mut self) -> &mut Option<ExtendedBlockInfo> {
        &mut self.last_committed_block_info
    }

    fn current_protocol_version_in_consensus_mut(&mut self) -> &mut ProtocolVersion {
        self.mark_all_unsaved();
        &mut self.current_protocol_version_in_consensus
    }

    fn next_epoch_protocol_version_mut(&mut self) -> &mut ProtocolVersion {
        self.heavy_fields_dirty = true;
        &mut self.next_epoch_protocol_version
    }

    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash {
        &mut self.current_validator_set_quorum_hash
    }

    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash> {
        &mut self.next_validator_set_quorum_hash
    }

    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet> {
        self.heavy_fields_dirty = true;
        // The caller may change any member through the borrow.
        self.validator_set_changes.mark_rewrite_all();
        &mut self.validator_sets
    }

    fn chain_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet {
        self.heavy_fields_dirty = true;
        &mut self.chain_lock_validating_quorums
    }

    fn instant_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet {
        self.heavy_fields_dirty = true;
        &mut self.instant_lock_validating_quorums
    }

    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        self.heavy_fields_dirty = true;
        // The caller may change any member through the borrow.
        self.masternode_changes.mark_rewrite_all();
        &mut self.full_masternode_list
    }

    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        self.heavy_fields_dirty = true;
        self.masternode_changes.mark_rewrite_all();
        &mut self.hpmn_masternode_list
    }

    fn insert_masternode(&mut self, masternode: MasternodeListItem) {
        let pro_tx_hash = masternode.pro_tx_hash;
        if masternode.node_type == MasternodeType::Evo {
            self.hpmn_masternode_list
                .insert(pro_tx_hash, masternode.clone());
        } else {
            self.hpmn_masternode_list.remove(&pro_tx_hash);
        }
        self.full_masternode_list.insert(pro_tx_hash, masternode);
        self.heavy_fields_dirty = true;
        self.masternode_changes.upsert(pro_tx_hash);
    }

    fn apply_masternode_state_diff(
        &mut self,
        pro_tx_hash: &ProTxHash,
        state_diff: &DMNStateDiff,
    ) -> bool {
        let Some(masternode) = self.full_masternode_list.get_mut(pro_tx_hash) else {
            return false;
        };
        masternode.state.apply_diff(state_diff.clone());
        if let Some(hpmn) = self.hpmn_masternode_list.get_mut(pro_tx_hash) {
            hpmn.state.apply_diff(state_diff.clone());
        }
        self.heavy_fields_dirty = true;
        self.masternode_changes.upsert(*pro_tx_hash);
        true
    }

    fn remove_masternode(&mut self, pro_tx_hash: &ProTxHash) -> Option<MasternodeListItem> {
        self.hpmn_masternode_list.remove(pro_tx_hash);
        let removed = self.full_masternode_list.remove(pro_tx_hash);
        if removed.is_some() {
            self.heavy_fields_dirty = true;
            self.masternode_changes.remove(*pro_tx_hash);
        }
        removed
    }

    fn clear_masternode_lists(&mut self) {
        self.full_masternode_list.clear();
        self.hpmn_masternode_list.clear();
        self.heavy_fields_dirty = true;
        self.masternode_changes.mark_rewrite_all();
    }

    fn validator_set_mut(&mut self, quorum_hash: &QuorumHash) -> Option<&mut ValidatorSet> {
        let validator_set = self.validator_sets.get_mut(quorum_hash)?;
        self.heavy_fields_dirty = true;
        self.validator_set_changes.upsert(*quorum_hash);
        Some(validator_set)
    }

    fn insert_validator_set(&mut self, quorum_hash: QuorumHash, validator_set: ValidatorSet) {
        self.validator_sets.insert(quorum_hash, validator_set);
        self.heavy_fields_dirty = true;
        self.validator_set_changes.upsert(quorum_hash);
    }

    fn remove_validator_set(&mut self, quorum_hash: &QuorumHash) -> Option<ValidatorSet> {
        let removed = self.validator_sets.shift_remove(quorum_hash);
        if removed.is_some() {
            self.heavy_fields_dirty = true;
            self.validator_set_changes.remove(*quorum_hash);
        }
        removed
    }

    fn sort_validator_sets_by(
        &mut self,
        compare: &mut dyn FnMut(&ValidatorSet, &ValidatorSet) -> Ordering,
    ) {
        self.validator_sets.sort_by(|_, a, _, b| compare(a, b));
        // The order is part of the record, which structure 0 rewrites when dirty
        // and structure 1 writes every block; no entry changes.
        self.heavy_fields_dirty = true;
    }

    fn last_committed_block_epoch_ref(&self) -> &Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| &block_info.basic_info().epoch)
            .unwrap_or(&EPOCH_0)
    }

    /// The last block id hash
    fn last_committed_block_id_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.block_id_hash())
            .unwrap_or_default()
    }

    fn full_masternode_list_changes(&self, previous: &PlatformState) -> MasternodeListChanges {
        let mut new_masternodes = Vec::new();
        let mut removed_masternodes = Vec::new();
        let mut banned_masternodes = Vec::new();
        let mut unbanned_masternodes = Vec::new();
        let mut new_banned_masternodes = Vec::new();

        // Check for new, banned/unbanned, and new banned masternodes
        for (pro_tx_hash, current_item) in &self.full_masternode_list {
            if let Some(previous_item) = previous.full_masternode_list.get(pro_tx_hash) {
                let current_ban_height = current_item.state.pose_ban_height;
                let previous_ban_height = previous_item.state.pose_ban_height;

                if current_ban_height.is_some() && previous_ban_height.is_none() {
                    // Masternode was banned
                    banned_masternodes.push(*pro_tx_hash);
                    if previous_item.state.pose_ban_height.is_none() {
                        // New banned masternode
                        new_banned_masternodes.push(*pro_tx_hash);
                    }
                } else if current_ban_height.is_none() && previous_ban_height.is_some() {
                    // Masternode was unbanned
                    unbanned_masternodes.push(*pro_tx_hash);
                }
            } else {
                // New masternode
                new_masternodes.push(*pro_tx_hash);
                if current_item.state.pose_ban_height.is_some() {
                    // New banned masternode
                    new_banned_masternodes.push(*pro_tx_hash);
                }
            }
        }

        // Check for removed masternodes
        for pro_tx_hash in previous.full_masternode_list.keys() {
            if !self.full_masternode_list.contains_key(pro_tx_hash) {
                removed_masternodes.push(*pro_tx_hash);
            }
        }

        MasternodeListChanges {
            new_masternodes,
            removed_masternodes,
            banned_masternodes,
            unbanned_masternodes,
            new_banned_masternodes,
        }
    }

    fn hpmn_masternode_list_changes(&self, previous: &PlatformState) -> MasternodeListChanges {
        let mut new_masternodes = Vec::new();
        let mut removed_masternodes = Vec::new();
        let mut banned_masternodes = Vec::new();
        let mut unbanned_masternodes = Vec::new();
        let mut new_banned_masternodes = Vec::new();

        // Check for new, banned/unbanned, and new banned masternodes
        for (pro_tx_hash, current_item) in &self.hpmn_masternode_list {
            if let Some(previous_item) = previous.hpmn_masternode_list.get(pro_tx_hash) {
                let current_ban_height = current_item.state.pose_ban_height;
                let previous_ban_height = previous_item.state.pose_ban_height;

                if current_ban_height.is_some() && previous_ban_height.is_none() {
                    // Masternode was banned
                    banned_masternodes.push(*pro_tx_hash);
                    if previous_item.state.pose_ban_height.is_none() {
                        // New banned masternode
                        new_banned_masternodes.push(*pro_tx_hash);
                    }
                } else if current_ban_height.is_none() && previous_ban_height.is_some() {
                    // Masternode was unbanned
                    unbanned_masternodes.push(*pro_tx_hash);
                }
            } else {
                // New masternode
                new_masternodes.push(*pro_tx_hash);
                if current_item.state.pose_ban_height.is_some() {
                    // New banned masternode
                    new_banned_masternodes.push(*pro_tx_hash);
                }
            }
        }

        // Check for removed masternodes
        for pro_tx_hash in previous.hpmn_masternode_list.keys() {
            if !self.hpmn_masternode_list.contains_key(pro_tx_hash) {
                removed_masternodes.push(*pro_tx_hash);
            }
        }

        MasternodeListChanges {
            new_masternodes,
            removed_masternodes,
            banned_masternodes,
            unbanned_masternodes,
            new_banned_masternodes,
        }
    }

    fn previous_fee_versions(&self) -> &CachedEpochIndexFeeVersions {
        &self.previous_fee_versions
    }

    fn previous_fee_versions_mut(&mut self) -> &mut CachedEpochIndexFeeVersions {
        self.heavy_fields_dirty = true;
        &mut self.previous_fee_versions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfig;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Network;

    fn clean_state() -> PlatformState {
        let platform_version = PlatformVersion::latest();
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        state.mark_saved();
        state
    }

    /// Runs `mutate` on a clean state and reports whether it left the state dirty.
    fn leaves_dirty(mutate: impl FnOnce(&mut PlatformState)) -> bool {
        let mut state = clean_state();
        mutate(&mut state);
        state.heavy_fields_dirty
    }

    /// A state that has never been written in full must start dirty, or its
    /// first historical block would skip the full write.
    #[test]
    fn a_new_state_starts_dirty() {
        let platform_version = PlatformVersion::latest();
        let state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");

        assert!(state.heavy_fields_dirty);
    }

    /// Every accessor that can change a field carried only by the full saved
    /// record must mark the state dirty. If one stops doing so, a historical
    /// block that changes that field through it skips the full write, and a
    /// node restarted from disk comes back with the old value.
    #[test]
    fn heavy_field_accessors_mark_the_state_dirty() {
        let quorums = clean_state().chain_lock_validating_quorums().clone();

        assert!(leaves_dirty(
            |s| s.set_current_protocol_version_in_consensus(1)
        ));
        assert!(leaves_dirty(|s| s.set_next_epoch_protocol_version(1)));
        assert!(leaves_dirty(|s| s.set_validator_sets(IndexMap::new())));
        assert!(leaves_dirty(
            |s| s.set_chain_lock_validating_quorums(quorums.clone())
        ));
        assert!(leaves_dirty(
            |s| s.set_instant_lock_validating_quorums(quorums.clone())
        ));
        assert!(leaves_dirty(|s| s.set_full_masternode_list(BTreeMap::new())));
        assert!(leaves_dirty(|s| s.set_hpmn_masternode_list(BTreeMap::new())));

        // Handing out the mutable borrow is enough: the caller may change the
        // field through it without the state seeing the write.
        assert!(leaves_dirty(|s| {
            s.current_protocol_version_in_consensus_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.next_epoch_protocol_version_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.validator_sets_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.chain_lock_validating_quorums_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.instant_lock_validating_quorums_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.full_masternode_list_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.hpmn_masternode_list_mut();
        }));
        assert!(leaves_dirty(|s| {
            s.previous_fee_versions_mut();
        }));
    }

    /// The fields the small per-block record carries are written every block
    /// regardless, so changing them must not force a full rewrite. The genesis
    /// block info is the one field in neither set: the block end clears it
    /// before every store, so dirtying the clear would rewrite the full record
    /// on every block.
    #[test]
    fn per_block_field_accessors_leave_the_state_clean() {
        assert!(!leaves_dirty(|s| s.set_last_committed_block_info(None)));
        assert!(!leaves_dirty(|s| {
            s.set_current_validator_set_quorum_hash(QuorumHash::all_zeros())
        }));
        assert!(!leaves_dirty(|s| s.set_next_validator_set_quorum_hash(None)));
        assert!(!leaves_dirty(|s| s.set_genesis_block_info(None)));
        assert!(!leaves_dirty(|s| {
            s.take_next_validator_set_quorum_hash();
        }));

        assert!(!leaves_dirty(|s| {
            s.last_committed_block_info_mut();
        }));
        assert!(!leaves_dirty(|s| {
            s.current_validator_set_quorum_hash_mut();
        }));
        assert!(!leaves_dirty(|s| {
            s.next_validator_set_quorum_hash_mut();
        }));
    }

    fn masternode(pro_tx_hash: ProTxHash, node_type: MasternodeType) -> MasternodeListItem {
        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::Txid;
        use dpp::dashcore_rpc::dashcore_rpc_json::DMNState;

        MasternodeListItem {
            node_type,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: [0u8; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:1234".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: [0u8; 20],
                voting_address: [0u8; 20],
                payout_address: [0u8; 20],
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        }
    }

    fn service_change() -> DMNStateDiff {
        DMNStateDiff {
            service: Some("5.6.7.8:5678".parse().expect("socket address")),
            registered_height: None,
            last_paid_height: None,
            consecutive_payments: None,
            pose_penalty: None,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: None,
            owner_address: None,
            voting_address: None,
            payout_address: None,
            pub_key_operator: None,
            operator_payout_address: None,
            platform_node_id: None,
            platform_p2p_port: None,
            platform_http_port: None,
        }
    }

    /// The per-member accessors record exactly the masternodes they touched, so
    /// the structure 1 store writes only those entries.
    #[test]
    fn per_member_masternode_accessors_record_exactly_what_they_touched() {
        use dpp::dashcore::hashes::Hash;
        use std::collections::BTreeSet;

        let mut state = clean_state();
        let evo = ProTxHash::from_byte_array([1u8; 32]);
        let regular = ProTxHash::from_byte_array([2u8; 32]);
        let unknown = ProTxHash::from_byte_array([3u8; 32]);

        state.insert_masternode(masternode(evo, MasternodeType::Evo));
        state.insert_masternode(masternode(regular, MasternodeType::Regular));
        assert_eq!(
            state.masternode_changes.upserted,
            BTreeSet::from([evo, regular])
        );
        assert_eq!(
            state.hpmn_masternode_list().keys().collect::<Vec<_>>(),
            vec![&evo],
            "only the Evo node joins the HPMN list"
        );

        assert!(state.apply_masternode_state_diff(&evo, &service_change()));
        assert!(!state.apply_masternode_state_diff(&unknown, &service_change()));
        assert_eq!(
            state.masternode_changes.upserted,
            BTreeSet::from([evo, regular])
        );
        assert_eq!(
            state.hpmn_masternode_list()[&evo].state.service,
            state.full_masternode_list()[&evo].state.service,
            "the diff reaches both lists"
        );

        assert!(state.remove_masternode(&evo).is_some());
        assert!(state.remove_masternode(&unknown).is_none());
        assert_eq!(state.masternode_changes.upserted, BTreeSet::from([regular]));
        assert_eq!(state.masternode_changes.removed, BTreeSet::from([evo]));
        assert!(state.hpmn_masternode_list().is_empty());

        assert!(!state.masternode_changes.rewrite_all);
        assert!(
            state.heavy_fields_dirty,
            "structure 0 still rewrites its record"
        );
    }

    /// An accessor that hands out or replaces a whole collection cannot say
    /// which members changed, so the store rewrites every entry.
    #[test]
    fn whole_collection_accessors_mark_a_rewrite() {
        fn masternodes_rewritten(mutate: impl FnOnce(&mut PlatformState)) -> bool {
            let mut state = clean_state();
            mutate(&mut state);
            state.masternode_changes.rewrite_all
        }
        fn validator_sets_rewritten(mutate: impl FnOnce(&mut PlatformState)) -> bool {
            let mut state = clean_state();
            mutate(&mut state);
            state.validator_set_changes.rewrite_all
        }

        assert!(masternodes_rewritten(
            |s| s.set_full_masternode_list(BTreeMap::new())
        ));
        assert!(masternodes_rewritten(
            |s| s.set_hpmn_masternode_list(BTreeMap::new())
        ));
        assert!(masternodes_rewritten(|s| {
            s.full_masternode_list_mut();
        }));
        assert!(masternodes_rewritten(|s| {
            s.hpmn_masternode_list_mut();
        }));
        assert!(masternodes_rewritten(|s| s.clear_masternode_lists()));
        assert!(validator_sets_rewritten(
            |s| s.set_validator_sets(IndexMap::new())
        ));
        assert!(validator_sets_rewritten(|s| {
            s.validator_sets_mut();
        }));

        // The per-member accessors and everything else do not.
        assert!(!masternodes_rewritten(|s| {
            s.remove_masternode(&ProTxHash::all_zeros());
        }));
        assert!(!validator_sets_rewritten(|s| {
            s.remove_validator_set(&QuorumHash::all_zeros());
        }));
        assert!(!masternodes_rewritten(
            |s| s.set_last_committed_block_info(None)
        ));
    }

    /// A protocol version change may switch the saved structure to one that
    /// keeps the collections as entries, so every entry has to be written.
    #[test]
    fn a_protocol_version_change_marks_everything_unsaved() {
        let mut state = clean_state();
        state.set_current_protocol_version_in_consensus(1);
        assert!(state.heavy_fields_dirty);
        assert!(state.masternode_changes.rewrite_all);
        assert!(state.validator_set_changes.rewrite_all);

        let mut state = clean_state();
        state.current_protocol_version_in_consensus_mut();
        assert!(state.masternode_changes.rewrite_all);
        assert!(state.validator_set_changes.rewrite_all);
    }

    /// A new state has everything pending; a stored one has nothing; a state
    /// whose store did not reach disk has everything again.
    #[test]
    fn mark_saved_and_mark_all_unsaved_cover_every_tracker() {
        let platform_version = PlatformVersion::latest();
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        assert!(state.masternode_changes.rewrite_all);
        assert!(state.validator_set_changes.rewrite_all);

        state.mark_saved();
        assert!(!state.heavy_fields_dirty);
        assert!(state.masternode_changes.is_empty());
        assert!(state.validator_set_changes.is_empty());

        state.mark_all_unsaved();
        assert!(state.heavy_fields_dirty);
        assert!(state.masternode_changes.rewrite_all);
        assert!(state.validator_set_changes.rewrite_all);
    }
}
