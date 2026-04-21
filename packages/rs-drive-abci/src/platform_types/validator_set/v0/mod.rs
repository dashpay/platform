use crate::error::execution::ExecutionError;
use crate::error::Error;

use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;

use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::validator::v0::NewValidatorIfMasternodeInState;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::core_types::validator::v0::ValidatorV0;
pub use dpp::core_types::validator_set::v0::*;
use dpp::dashcore_rpc::json::QuorumInfoResult;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::{abci, crypto};

#[allow(dead_code)]
pub(crate) trait ValidatorSetMethodsV0 {
    #[allow(unused)]
    fn update_difference(&self, rhs: &ValidatorSetV0) -> Result<ValidatorSetUpdate, Error>;

    fn to_update(&self) -> ValidatorSetUpdate;
    #[allow(dead_code)]
    fn to_update_owned(self) -> ValidatorSetUpdate;
    /// Try to create a quorum from info from the Masternode list (given with state),
    /// and for information return for quorum members
    fn try_from_quorum_info_result(
        value: QuorumInfoResult,
        state: &PlatformState,
    ) -> Result<ValidatorSetV0, Error>;
}

impl ValidatorSetMethodsV0 for ValidatorSetV0 {
    /// For changes between two validator sets, we take the new (rhs) element if is different
    /// for every validator
    fn update_difference(&self, rhs: &ValidatorSetV0) -> Result<ValidatorSetUpdate, Error> {
        if self.quorum_hash != rhs.quorum_hash {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                format!(
                    "updating validator set doesn't match quorum hash ours: {} theirs: {}",
                    self.quorum_hash, rhs.quorum_hash
                ),
            )));
        }

        if self.core_height != rhs.core_height {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                format!(
                    "updating validator set doesn't match core height ours: {} theirs: {}",
                    self.core_height, rhs.core_height
                ),
            )));
        }

        if self.threshold_public_key != rhs.threshold_public_key {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                format!(
                    "updating validator set doesn't match threshold public key ours: {} theirs: {}",
                    hex::encode(self.threshold_public_key.0.to_compressed()),
                    hex::encode(rhs.threshold_public_key.0.to_compressed())
                ),
            )));
        }

        let validator_updates = self
            .members
            .iter()
            .filter_map(|(pro_tx_hash, old_validator_state)| {
                rhs.members.get(pro_tx_hash).map_or_else(
                    || {
                        Some(Err(Error::Execution(ExecutionError::CorruptedCachedState(
                            format!(
                                "validator set does not contain all same members, missing {}",
                                pro_tx_hash
                            ),
                        ))))
                    },
                    |new_validator_state| {
                        if new_validator_state != old_validator_state {
                            let ValidatorV0 {
                                pro_tx_hash,
                                public_key,
                                node_ip,
                                node_id,
                                platform_p2p_port,
                                is_banned,
                                ..
                            } = new_validator_state;

                            if *is_banned {
                                None
                            } else {
                                let node_address = format!(
                                    "tcp://{}@{}:{}",
                                    hex::encode(node_id.to_byte_array()),
                                    node_ip,
                                    platform_p2p_port
                                );

                                Some(Ok(abci::ValidatorUpdate {
                                    pub_key: (*public_key).map(|public_key| crypto::PublicKey {
                                        sum: Some(Bls12381(public_key.0.to_compressed().to_vec())),
                                    }),
                                    power: 100,
                                    pro_tx_hash: pro_tx_hash.as_byte_array().to_vec(),
                                    node_address,
                                }))
                            }
                        } else {
                            let ValidatorV0 {
                                pro_tx_hash,
                                public_key,
                                node_ip,
                                node_id,
                                platform_p2p_port,
                                is_banned,
                                ..
                            } = old_validator_state;

                            if *is_banned {
                                None
                            } else {
                                let node_address = format!(
                                    "tcp://{}@{}:{}",
                                    hex::encode(node_id.to_byte_array()),
                                    node_ip,
                                    platform_p2p_port
                                );

                                Some(Ok(abci::ValidatorUpdate {
                                    pub_key: (*public_key).map(|public_key| crypto::PublicKey {
                                        sum: Some(Bls12381(public_key.0.to_compressed().to_vec())),
                                    }),
                                    power: 100,
                                    pro_tx_hash: pro_tx_hash.to_byte_array().to_vec(),
                                    node_address,
                                }))
                            }
                        }
                    },
                )
            })
            .collect::<Result<Vec<abci::ValidatorUpdate>, Error>>()?;

        Ok(ValidatorSetUpdate {
            validator_updates,
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(
                    self.threshold_public_key.0.to_compressed().to_vec(),
                )),
            }),
            quorum_hash: self.quorum_hash.to_byte_array().to_vec(),
        })
    }

    fn to_update(&self) -> ValidatorSetUpdate {
        let ValidatorSetV0 {
            quorum_hash,
            members: validator_set,
            threshold_public_key,
            ..
        } = self;
        ValidatorSetUpdate {
            validator_updates: validator_set
                .iter()
                .filter_map(|(_, validator)| {
                    let ValidatorV0 {
                        pro_tx_hash,
                        public_key,
                        node_ip,
                        node_id,
                        platform_p2p_port,
                        is_banned,
                        ..
                    } = validator;

                    if *is_banned {
                        return None;
                    }
                    let node_address = format!(
                        "tcp://{}@{}:{}",
                        hex::encode(node_id.to_byte_array()),
                        node_ip,
                        platform_p2p_port
                    );
                    Some(abci::ValidatorUpdate {
                        pub_key: public_key.as_ref().map(|public_key| crypto::PublicKey {
                            sum: Some(Bls12381(public_key.0.to_compressed().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: pro_tx_hash.to_byte_array().to_vec(),
                        node_address,
                    })
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.0.to_compressed().to_vec())),
            }),
            quorum_hash: quorum_hash.to_byte_array().to_vec(),
        }
    }

    fn to_update_owned(self) -> ValidatorSetUpdate {
        let ValidatorSetV0 {
            quorum_hash,
            members: validator_set,
            threshold_public_key,
            ..
        } = self;
        ValidatorSetUpdate {
            validator_updates: validator_set
                .into_values()
                .filter_map(|validator| {
                    let ValidatorV0 {
                        pro_tx_hash,
                        public_key,
                        node_ip,
                        node_id,
                        platform_p2p_port,
                        is_banned,
                        ..
                    } = validator;
                    if is_banned {
                        return None;
                    }
                    let node_address = format!(
                        "tcp://{}@{}:{}",
                        hex::encode(node_id.to_byte_array()),
                        node_ip,
                        platform_p2p_port
                    );

                    Some(abci::ValidatorUpdate {
                        pub_key: public_key.map(|public_key| crypto::PublicKey {
                            sum: Some(Bls12381(public_key.0.to_compressed().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: pro_tx_hash.to_byte_array().to_vec(),
                        node_address,
                    })
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.0.to_compressed().to_vec())),
            }),
            quorum_hash: quorum_hash.to_byte_array().to_vec(),
        }
    }

    /// Try to create a quorum from info from the Masternode list (given with state),
    /// and for information return for quorum members
    fn try_from_quorum_info_result(
        value: QuorumInfoResult,
        state: &PlatformState,
    ) -> Result<Self, Error> {
        let QuorumInfoResult {
            height,
            quorum_hash,
            quorum_index,
            quorum_public_key,
            members,
            ..
        } = value;

        let validator_set = members
            .into_iter()
            .filter_map(|quorum_member| {
                if !quorum_member.valid {
                    return None;
                }

                let public_key = if let Some(public_key_share) = quorum_member.pub_key_share {
                    match BlsPublicKey::try_from(public_key_share.as_slice())
                        .map_err(ExecutionError::BlsErrorFromDashCoreResponse)
                    {
                        Ok(public_key) => Some(public_key),
                        Err(e) => return Some(Err(e.into())),
                    }
                } else {
                    None
                };

                let validator = ValidatorV0::new_validator_if_masternode_in_state(
                    quorum_member.pro_tx_hash,
                    public_key,
                    state,
                )?;
                Some(Ok((quorum_member.pro_tx_hash, validator)))
            })
            .collect::<Result<BTreeMap<ProTxHash, ValidatorV0>, Error>>()?;

        let threshold_public_key = BlsPublicKey::try_from(quorum_public_key.as_slice())
            .map_err(ExecutionError::BlsErrorFromDashCoreResponse)?;

        let optional_quorum_index = if quorum_index == 0 {
            None
        } else {
            Some(quorum_index)
        };

        Ok(ValidatorSetV0 {
            quorum_hash,
            quorum_index: optional_quorum_index,
            core_height: height,
            members: validator_set,
            threshold_public_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, PubkeyHash, QuorumHash};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_validator(pro_tx_hash: ProTxHash, is_banned: bool) -> ValidatorV0 {
        let mut rng = StdRng::seed_from_u64(1);
        let public_key = Some(SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key());
        ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip: "192.168.0.1".to_string(),
            node_id: PubkeyHash::from_slice(&[7; 20]).unwrap(),
            core_port: 10,
            platform_http_port: 20,
            platform_p2p_port: 30,
            is_banned,
        }
    }

    fn make_set(
        quorum_hash_seed: u8,
        core_height: u32,
        threshold_seed: u64,
        members: BTreeMap<ProTxHash, ValidatorV0>,
    ) -> ValidatorSetV0 {
        let mut rng = StdRng::seed_from_u64(threshold_seed);
        let threshold_public_key = SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();
        ValidatorSetV0 {
            quorum_hash: QuorumHash::from_slice(&[quorum_hash_seed; 32]).unwrap(),
            quorum_index: Some(1),
            core_height,
            members,
            threshold_public_key,
        }
    }

    #[test]
    fn test_update_difference_quorum_hash_mismatch() {
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members_a = BTreeMap::new();
        members_a.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let mut members_b = BTreeMap::new();
        members_b.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let lhs = make_set(1, 100, 1, members_a);
        let rhs = make_set(2, 100, 1, members_b);

        let err = lhs.update_difference(&rhs).unwrap_err();
        assert!(
            matches!(err, Error::Execution(ExecutionError::CorruptedCachedState(ref msg)) if msg.contains("quorum hash"))
        );
    }

    #[test]
    fn test_update_difference_core_height_mismatch() {
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members_a = BTreeMap::new();
        members_a.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let mut members_b = BTreeMap::new();
        members_b.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let lhs = make_set(1, 100, 1, members_a);
        let rhs = make_set(1, 200, 1, members_b);

        let err = lhs.update_difference(&rhs).unwrap_err();
        assert!(
            matches!(err, Error::Execution(ExecutionError::CorruptedCachedState(ref msg)) if msg.contains("core height"))
        );
    }

    #[test]
    fn test_update_difference_threshold_public_key_mismatch() {
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members_a = BTreeMap::new();
        members_a.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let mut members_b = BTreeMap::new();
        members_b.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        // Same quorum_hash & core_height but distinct threshold keys (different seeds)
        let lhs = make_set(1, 100, 1, members_a);
        let rhs = make_set(1, 100, 2, members_b);

        let err = lhs.update_difference(&rhs).unwrap_err();
        assert!(
            matches!(err, Error::Execution(ExecutionError::CorruptedCachedState(ref msg)) if msg.contains("threshold public key"))
        );
    }

    #[test]
    fn test_update_difference_missing_member() {
        // lhs has a member that is not present in rhs -> error
        let pro_tx_hash_a = ProTxHash::from_slice(&[9; 32]).unwrap();
        let pro_tx_hash_b = ProTxHash::from_slice(&[10; 32]).unwrap();
        let mut members_a = BTreeMap::new();
        members_a.insert(pro_tx_hash_a, make_validator(pro_tx_hash_a, false));

        let mut members_b = BTreeMap::new();
        members_b.insert(pro_tx_hash_b, make_validator(pro_tx_hash_b, false));

        let lhs = make_set(1, 100, 1, members_a);
        // Same threshold key as lhs by reusing seed 1
        let rhs_temp = make_set(1, 100, 1, members_b);
        // force threshold keys equal by copying
        let rhs = ValidatorSetV0 {
            threshold_public_key: lhs.threshold_public_key,
            ..rhs_temp
        };

        let err = lhs.update_difference(&rhs).unwrap_err();
        assert!(
            matches!(err, Error::Execution(ExecutionError::CorruptedCachedState(ref msg)) if msg.contains("does not contain all same members"))
        );
    }

    #[test]
    fn test_update_difference_identical_sets_emit_for_non_banned_old_members() {
        // When rhs members are structurally equal to lhs members, the code
        // takes the "else" branch that emits an update for each non-banned
        // old-state validator.
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members_a = BTreeMap::new();
        members_a.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let lhs = make_set(1, 100, 1, members_a.clone());
        let rhs = ValidatorSetV0 {
            threshold_public_key: lhs.threshold_public_key,
            ..make_set(1, 100, 1, members_a)
        };

        let update = lhs.update_difference(&rhs).expect("expected ok");
        assert_eq!(update.validator_updates.len(), 1);
        assert_eq!(
            update.validator_updates[0].pro_tx_hash,
            pro_tx_hash.to_byte_array().to_vec()
        );
        assert_eq!(update.quorum_hash.len(), 32);
    }

    #[test]
    fn test_update_difference_identical_but_banned_emits_nothing() {
        // Both sets have identical, banned validator => banned short-circuits
        // to None in the equal branch.
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members_a = BTreeMap::new();
        members_a.insert(pro_tx_hash, make_validator(pro_tx_hash, true));

        let lhs = make_set(1, 100, 1, members_a.clone());
        let rhs = ValidatorSetV0 {
            threshold_public_key: lhs.threshold_public_key,
            ..make_set(1, 100, 1, members_a)
        };

        let update = lhs.update_difference(&rhs).expect("expected ok");
        assert!(update.validator_updates.is_empty());
    }

    #[test]
    fn test_update_difference_banned_new_is_skipped() {
        // new_validator_state differs (is_banned=true) -> the function takes the
        // "different" branch and emits None because banned.
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members_old = BTreeMap::new();
        members_old.insert(pro_tx_hash, make_validator(pro_tx_hash, false));

        let mut members_new = BTreeMap::new();
        members_new.insert(pro_tx_hash, make_validator(pro_tx_hash, true));

        let lhs = make_set(1, 100, 1, members_old);
        let rhs = ValidatorSetV0 {
            threshold_public_key: lhs.threshold_public_key,
            ..make_set(1, 100, 1, members_new)
        };

        let update = lhs.update_difference(&rhs).expect("expected ok");
        assert!(update.validator_updates.is_empty());
    }

    #[test]
    fn test_to_update_excludes_banned_validators() {
        let pro_tx_hash_a = ProTxHash::from_slice(&[9; 32]).unwrap();
        let pro_tx_hash_b = ProTxHash::from_slice(&[10; 32]).unwrap();
        let mut members = BTreeMap::new();
        members.insert(pro_tx_hash_a, make_validator(pro_tx_hash_a, false));
        members.insert(pro_tx_hash_b, make_validator(pro_tx_hash_b, true));

        let set = make_set(1, 100, 1, members);

        let update = set.to_update();
        assert_eq!(update.validator_updates.len(), 1);
        assert_eq!(
            update.validator_updates[0].pro_tx_hash,
            pro_tx_hash_a.to_byte_array().to_vec()
        );
        // quorum hash round-trips
        assert_eq!(update.quorum_hash, vec![1u8; 32]);
    }

    #[test]
    fn test_to_update_owned_excludes_banned_validators() {
        let pro_tx_hash_a = ProTxHash::from_slice(&[9; 32]).unwrap();
        let pro_tx_hash_b = ProTxHash::from_slice(&[10; 32]).unwrap();
        let mut members = BTreeMap::new();
        members.insert(pro_tx_hash_a, make_validator(pro_tx_hash_a, true));
        members.insert(pro_tx_hash_b, make_validator(pro_tx_hash_b, false));

        let set = make_set(3, 100, 1, members);

        let update = set.to_update_owned();
        assert_eq!(update.validator_updates.len(), 1);
        assert_eq!(
            update.validator_updates[0].pro_tx_hash,
            pro_tx_hash_b.to_byte_array().to_vec()
        );
        assert_eq!(update.quorum_hash, vec![3u8; 32]);
    }

    #[test]
    fn test_to_update_all_banned_produces_no_updates() {
        let pro_tx_hash = ProTxHash::from_slice(&[9; 32]).unwrap();
        let mut members = BTreeMap::new();
        members.insert(pro_tx_hash, make_validator(pro_tx_hash, true));

        let set = make_set(4, 100, 1, members);

        let update = set.to_update();
        assert!(update.validator_updates.is_empty());
        // threshold_public_key is still present
        assert!(update.threshold_public_key.is_some());
    }
}
