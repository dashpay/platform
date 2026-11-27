use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::quorum_key_share_cache::QuorumKeyShares;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;
use dpp::bls::PublicKey as BlsPublicKey;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::dashcore_rpc::json::QuorumInfoResult;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::crypto::PublicKey;

/// Why Core's answer gave no key shares to fill in.
#[derive(Debug)]
enum NoKeyShares {
    /// The answer is about another quorum, or a quorum with another public key or height.
    OtherQuorum,
    /// Core gave a share that is not a public key.
    InvalidShare(ProTxHash),
    /// Core gave no share for this many members of the set, usually because it does not have
    /// the quorum's verification vector (yet).
    Missing(usize),
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Gives every validator in `update` that has no public key share the one Core has now, when
    /// `update` moves Tenderdash to a quorum other than `current_quorum_hash` and the shares cover
    /// every member of the validator set being sent; otherwise leaves `update` and
    /// `block_platform_state` as they are.
    ///
    /// Drive asks Core for a quorum's members once, in the first block that sees the quorum. A
    /// Core that does not have the quorum's verification vector at that moment answers without
    /// key shares, and usually gets the vector soon after. Tenderdash takes a quorum's keys when
    /// it is sent a set for a quorum it does not hold, so the keys are filled in then. They are
    /// also set on the validator set in `block_platform_state`, so later blocks and restarts keep
    /// them, and kept in memory, so that executing the block again gives the same keys without
    /// Core and later rotations into the quorum need not ask Core again.
    ///
    /// An update for the quorum Tenderdash holds is left alone: Tenderdash merges it into the set
    /// it has, keeping that set's keys for validators the update leaves out (banned members), so
    /// filling it would mix validators with and without keys.
    ///
    /// Asks Core only when such an update lacks a key and no shares are kept for the quorum, once
    /// and with a short timeout. Never fails: an answer that cannot fill every key is logged and
    /// changes nothing.
    pub(in crate::execution::platform_events) fn complete_validator_key_shares(
        &self,
        update: &mut ValidatorSetUpdate,
        current_quorum_hash: &QuorumHash,
        block_platform_state: &mut PlatformState,
    ) {
        if update
            .validator_updates
            .iter()
            .all(|validator| validator.pub_key.is_some())
            || update.quorum_hash.as_slice() == current_quorum_hash.as_byte_array()
        {
            return;
        }
        let Ok(quorum_hash) = QuorumHash::from_slice(&update.quorum_hash) else {
            tracing::warn!(
                quorum_hash = hex::encode(&update.quorum_hash),
                "validator set update has a malformed quorum hash; sending it without the missing key shares"
            );
            return;
        };
        let Some(validator_set) = block_platform_state.validator_sets().get(&quorum_hash) else {
            tracing::warn!(
                quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                "validator set update is for a quorum the block state does not hold; sending it without the missing key shares"
            );
            return;
        };

        let key_shares = match self.quorum_key_shares.get(&quorum_hash) {
            Some(kept) if covers_members(&kept, validator_set) => kept,
            _ => {
                let Some(key_shares) = self.fetch_key_shares(validator_set) else {
                    return;
                };
                self.quorum_key_shares.keep(quorum_hash, key_shares)
            }
        };

        // Decided in full before anything changes, so the update is either complete or
        // untouched: a set with some keys and not others is no better for Tenderdash than one
        // without any, and is refused where Tenderdash checks light blocks.
        let mut update_keys = Vec::new();
        for (index, validator) in update.validator_updates.iter().enumerate() {
            if validator.pub_key.is_some() {
                continue;
            }
            let Some(key) = ProTxHash::from_slice(&validator.pro_tx_hash)
                .ok()
                .and_then(|pro_tx_hash| key_shares.get(&pro_tx_hash))
            else {
                tracing::warn!(
                    quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                    pro_tx_hash = hex::encode(&validator.pro_tx_hash),
                    "validator set update has a validator the set does not hold; sending it without the missing key shares"
                );
                return;
            };
            update_keys.push((index, *key));
        }
        let state_keys: Vec<(ProTxHash, BlsPublicKey)> = validator_set
            .members()
            .iter()
            .filter(|(_, validator)| validator.public_key.is_none())
            .filter_map(|(pro_tx_hash, _)| {
                key_shares.get(pro_tx_hash).map(|key| (*pro_tx_hash, *key))
            })
            .collect();

        for (index, key) in update_keys {
            update.validator_updates[index].pub_key = Some(PublicKey {
                sum: Some(Bls12381(key.to_bytes().to_vec())),
            });
        }
        // Taken only to change something: it marks the set's stored entry for rewriting.
        if !state_keys.is_empty() {
            if let Some(validator_set) = block_platform_state.validator_set_mut(&quorum_hash) {
                let members = validator_set.members_mut();
                for (pro_tx_hash, key) in state_keys {
                    if let Some(validator) = members.get_mut(&pro_tx_hash) {
                        validator.public_key = Some(key);
                    }
                }
            }
        }
        tracing::debug!(
            quorum_hash = hex::encode(quorum_hash.as_byte_array()),
            "filled in validator key shares Core did not have when the quorum was first seen"
        );
    }

    /// The key shares Core has now for every member of `validator_set`, or `None` (logged) when
    /// Core cannot give all of them.
    fn fetch_key_shares(&self, validator_set: &ValidatorSet) -> Option<QuorumKeyShares> {
        let quorum_hash = *validator_set.quorum_hash();
        let info = match self
            .core_rpc
            .get_quorum_info_with_short_timeout(self.config.validator_set.quorum_type, &quorum_hash)
        {
            Ok(info) => info,
            Err(error) => {
                tracing::warn!(
                    quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                    ?error,
                    "could not ask Core for the key shares of a validator quorum; sending its validator set without them"
                );
                return None;
            }
        };
        match key_shares_from_quorum_info(validator_set, info) {
            Ok(key_shares) => Some(key_shares),
            Err(NoKeyShares::Missing(missing)) => {
                // The usual answer for a quorum this node is not a member of.
                tracing::debug!(
                    quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                    missing,
                    "Core has no key shares for some members of a validator quorum; sending its validator set without them"
                );
                None
            }
            Err(NoKeyShares::InvalidShare(pro_tx_hash)) => {
                tracing::warn!(
                    quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                    %pro_tx_hash,
                    "Core answered with a key share that is not a public key; sending the validator set without key shares"
                );
                None
            }
            Err(NoKeyShares::OtherQuorum) => {
                tracing::warn!(
                    quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                    "Core answered about a quorum with another hash, height or public key; sending the validator set without key shares"
                );
                None
            }
        }
    }
}

/// Whether `key_shares` has a share for every member of `validator_set`.
fn covers_members(key_shares: &QuorumKeyShares, validator_set: &ValidatorSet) -> bool {
    validator_set
        .members()
        .keys()
        .all(|pro_tx_hash| key_shares.contains_key(pro_tx_hash))
}

/// The key shares Core's `info` gives for the valid members of the quorum of `validator_set`,
/// when `info` is about that quorum and has a share for every member of the set.
fn key_shares_from_quorum_info(
    validator_set: &ValidatorSet,
    info: QuorumInfoResult,
) -> Result<QuorumKeyShares, NoKeyShares> {
    let same_public_key = BlsPublicKey::try_from(info.quorum_public_key.as_slice())
        .is_ok_and(|public_key| &public_key == validator_set.threshold_public_key());
    if &info.quorum_hash != validator_set.quorum_hash()
        || info.height != validator_set.core_height()
        || !same_public_key
    {
        return Err(NoKeyShares::OtherQuorum);
    }

    let mut key_shares = BTreeMap::new();
    for member in info.members {
        let (true, Some(share)) = (member.valid, member.pub_key_share) else {
            continue;
        };
        let key = BlsPublicKey::try_from(share.as_slice())
            .map_err(|_| NoKeyShares::InvalidShare(member.pro_tx_hash))?;
        key_shares.insert(member.pro_tx_hash, key);
    }

    let missing = validator_set
        .members()
        .keys()
        .filter(|pro_tx_hash| !key_shares.contains_key(*pro_tx_hash))
        .count();
    if missing == 0 {
        Ok(key_shares)
    } else {
        Err(NoKeyShares::Missing(missing))
    }
}
