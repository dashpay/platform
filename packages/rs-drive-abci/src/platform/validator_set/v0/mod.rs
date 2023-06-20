use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform::validator::v0::Validator;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, PubkeyHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem};
use dashcore_rpc::json::QuorumInfoResult;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::{abci, crypto};

/// The validator set is only slightly different from a quorum as it does not contain non valid
/// members
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ValidatorSet {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// Active height
    pub core_height: u32,
    /// The list of masternodes
    pub members: BTreeMap<ProTxHash, Validator>,
    /// The threshold quorum public key
    pub threshold_public_key: BlsPublicKey,
}

impl ValidatorSet {
    /// For changes between two validator sets, we take the new (rhs) element if is different
    /// for every validator
    #[allow(dead_code)]
    pub(crate) fn update_difference(
        &self,
        rhs: &ValidatorSet,
    ) -> Result<ValidatorSetUpdate, Error> {
        if self.quorum_hash != rhs.quorum_hash {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                "updating validator set doesn't match quorum hash",
            )));
        }

        if self.core_height != rhs.core_height {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                "updating validator set doesn't match core height",
            )));
        }

        if self.threshold_public_key != rhs.threshold_public_key {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                "updating validator set doesn't match threshold public key",
            )));
        }

        let validator_updates = self
            .members
            .iter()
            .filter_map(|(pro_tx_hash, old_validator_state)| {
                rhs.members.get(pro_tx_hash).map_or_else(
                    || {
                        Some(Err(Error::Execution(ExecutionError::CorruptedCachedState(
                            "validator set does not contain all same members",
                        ))))
                    },
                    |new_validator_state| {
                        if new_validator_state != old_validator_state {
                            let Validator {
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
                                    hex::encode(node_id.into_inner()),
                                    node_ip,
                                    platform_p2p_port
                                );

                                Some(Ok(abci::ValidatorUpdate {
                                    pub_key: public_key.clone().map(|public_key| {
                                        crypto::PublicKey {
                                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                                        }
                                    }),
                                    power: 100,
                                    pro_tx_hash: reverse(pro_tx_hash),
                                    node_address,
                                }))
                            }
                        } else {
                            let Validator {
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
                                    hex::encode(node_id.into_inner()),
                                    node_ip,
                                    platform_p2p_port
                                );

                                Some(Ok(abci::ValidatorUpdate {
                                    pub_key: public_key.clone().map(|public_key| {
                                        crypto::PublicKey {
                                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                                        }
                                    }),
                                    power: 100,
                                    pro_tx_hash: reverse(pro_tx_hash),
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
                sum: Some(Bls12381(self.threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: reverse(&self.quorum_hash),
        })
    }
}

/// In this case we are changing to this validator set from another validator set and there are no
/// changes
impl From<ValidatorSet> for ValidatorSetUpdate {
    fn from(value: ValidatorSet) -> Self {
        let ValidatorSet {
            quorum_hash,
            members: validator_set,
            threshold_public_key,
            ..
        } = value;
        ValidatorSetUpdate {
            validator_updates: validator_set
                .into_values()
                .filter_map(|validator| {
                    let Validator {
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
                        hex::encode(node_id.into_inner()),
                        node_ip,
                        platform_p2p_port
                    );

                    Some(abci::ValidatorUpdate {
                        pub_key: public_key.map(|public_key| crypto::PublicKey {
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: reverse(&pro_tx_hash),
                        node_address,
                    })
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: reverse(&quorum_hash),
        }
    }
}

/// Reverse bytes
///
/// TODO: This is a workaround for reversed data returned by dashcore_rpc (little endian / big endian handling issue).
/// We need to decide on a consistent approach to endianness and follow it.
fn reverse(data: &[u8]) -> Vec<u8> {
    // data.reverse();

    data.to_vec()
}

impl From<&ValidatorSet> for ValidatorSetUpdate {
    fn from(value: &ValidatorSet) -> Self {
        let ValidatorSet {
            quorum_hash,
            members: validator_set,
            threshold_public_key,
            ..
        } = value;
        ValidatorSetUpdate {
            validator_updates: validator_set
                .iter()
                .filter_map(|(_, validator)| {
                    let Validator {
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
                        hex::encode(node_id.into_inner()),
                        node_ip,
                        platform_p2p_port
                    );
                    Some(abci::ValidatorUpdate {
                        pub_key: public_key.as_ref().map(|public_key| crypto::PublicKey {
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: reverse(pro_tx_hash),
                        node_address,
                    })
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: reverse(quorum_hash),
        }
    }
}

impl ValidatorSet {
    /// Try to create a quorum from info from the Masternode list (given with state),
    /// and for information return for quorum members
    pub fn try_from_quorum_info_result(
        value: QuorumInfoResult,
        state: &PlatformState,
    ) -> Result<Self, Error> {
        let QuorumInfoResult {
            height,
            quorum_hash,
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
                    match BlsPublicKey::from_bytes(public_key_share.as_slice())
                        .map_err(ExecutionError::BlsErrorFromDashCoreResponse)
                    {
                        Ok(public_key) => Some(public_key),
                        Err(e) => return Some(Err(e.into())),
                    }
                } else {
                    None
                };

                let validator = Validator::new_validator_if_masternode_in_state(
                    quorum_member.pro_tx_hash,
                    public_key,
                    state,
                )?;
                Some(Ok((quorum_member.pro_tx_hash, validator)))
            })
            .collect::<Result<BTreeMap<ProTxHash, Validator>, Error>>()?;

        let threshold_public_key = BlsPublicKey::from_bytes(quorum_public_key.as_slice())
            .map_err(ExecutionError::BlsErrorFromDashCoreResponse)?;

        Ok(ValidatorSet {
            quorum_hash,
            core_height: height,
            members: validator_set,
            threshold_public_key,
        })
    }
}
