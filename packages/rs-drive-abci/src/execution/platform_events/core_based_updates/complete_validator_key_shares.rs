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
use std::sync::Arc;
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
    /// `update` moves Tenderdash to a quorum other than `current_quorum_hash`, the shares cover
    /// every member of the validator set being sent, and the update's keys then recover the
    /// quorum's public key, as Tenderdash checks a fully keyed set; otherwise leaves `update` and
    /// `block_platform_state` as they are.
    ///
    /// Drive asks Core for a quorum's members once, in the first block that sees the quorum. A
    /// Core that does not have the quorum's verification vector at that moment answers without
    /// key shares, and usually gets the vector soon after. Tenderdash takes a quorum's keys when
    /// it is sent a set for a quorum it does not hold, so the keys are filled in then. They are
    /// also set on the validator set in `block_platform_state`, so later blocks and restarts keep
    /// them, and the keys the check covered are kept in memory, so that executing the block again
    /// gives the same keys without Core and later rotations into the quorum need not ask Core
    /// again.
    ///
    /// An update for the quorum Tenderdash holds is left alone: Tenderdash merges it into the set
    /// it has, keeping that set's keys for validators the update leaves out (banned members), so
    /// filling it would mix validators with and without keys.
    ///
    /// Asks Core only when such an update lacks a key and the kept keys do not cover every member
    /// of the set being sent, once and within a deadline for the whole request. Never fails: an
    /// answer that cannot fill every key is logged and changes nothing.
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

        let (key_shares, fetched) = match self.quorum_key_shares.get(&quorum_hash) {
            Some(kept) if covers_members(&kept, validator_set) => (kept, false),
            _ => match self.fetch_key_shares(validator_set) {
                Some(fetched) => (Arc::new(fetched), true),
                None => return,
            },
        };

        // Every key the update would carry, decided before anything changes, so the update is
        // either complete or untouched: a set with some keys and not others is no better for
        // Tenderdash than one without any, and is refused where Tenderdash checks light blocks.
        let mut sent_keys = Vec::with_capacity(update.validator_updates.len());
        let mut update_keys = Vec::new();
        for (index, validator) in update.validator_updates.iter().enumerate() {
            let pro_tx_hash = ProTxHash::from_slice(&validator.pro_tx_hash).ok();
            let key = match &validator.pub_key {
                Some(sent) => key_of(sent),
                None => {
                    let share = pro_tx_hash.and_then(|pro_tx_hash| key_shares.get(&pro_tx_hash));
                    if let Some(share) = share {
                        update_keys.push((index, *share));
                    }
                    share.copied()
                }
            };
            let (Some(pro_tx_hash), Some(key)) = (pro_tx_hash, key) else {
                tracing::warn!(
                    quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                    pro_tx_hash = hex::encode(&validator.pro_tx_hash),
                    "validator set update has a validator whose key share cannot be filled in; sending it without the missing key shares"
                );
                return;
            };
            sent_keys.push((pro_tx_hash, key));
        }
        // Every key the stored set would hold, banned members' too, which a later update for the
        // quorum Tenderdash holds may carry.
        let stored_keys: Vec<(ProTxHash, BlsPublicKey)> = validator_set
            .members()
            .iter()
            .filter_map(|(pro_tx_hash, validator)| {
                validator
                    .public_key
                    .or_else(|| key_shares.get(pro_tx_hash).copied())
                    .map(|key| (*pro_tx_hash, key))
            })
            .collect();
        // Tenderdash refuses a fully keyed set whose keys do not recover the quorum's public key,
        // so shares that do not, in the update or in the stored set, are neither sent, stored nor
        // kept. Without banned members both lists are the same and one recovery checks both.
        let quorum_public_key = validator_set.threshold_public_key();
        if !recovers_quorum_public_key(&sent_keys, quorum_public_key)
            || (stored_keys != sent_keys
                && !recovers_quorum_public_key(&stored_keys, quorum_public_key))
        {
            tracing::warn!(
                quorum_hash = hex::encode(quorum_hash.as_byte_array()),
                "Core's key shares do not recover the validator quorum's public key; sending its validator set without them"
            );
            return;
        }
        // Only the keys the checks covered: Core's answer may hold shares for members this
        // block's set no longer holds, which another execution of the rotation may still hold.
        if fetched {
            self.quorum_key_shares
                .keep(quorum_hash, stored_keys.iter().copied().collect());
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
            .get_quorum_info_within_deadline(self.config.validator_set.quorum_type, &quorum_hash)
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

/// The BLS key a validator update carries, if it is one.
fn key_of(sent: &PublicKey) -> Option<BlsPublicKey> {
    match &sent.sum {
        Some(Bls12381(bytes)) => BlsPublicKey::try_from(bytes.as_slice()).ok(),
        _ => None,
    }
}

/// Whether the members' key shares `keys` give the quorum's public key as Tenderdash checks a
/// fully keyed set: a single member's key must be the quorum's key, more members' keys must
/// recover it.
fn recovers_quorum_public_key(
    keys: &[(ProTxHash, BlsPublicKey)],
    quorum_public_key: &BlsPublicKey,
) -> bool {
    match keys {
        [(_, key)] => key == quorum_public_key,
        _ => BlsPublicKey::recover_threshold(keys)
            .is_ok_and(|recovered| &recovered == quorum_public_key),
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
