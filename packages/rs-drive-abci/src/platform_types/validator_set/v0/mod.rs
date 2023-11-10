use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::validator::v0::ValidatorV0;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};

use crate::platform_types::platform_state::PlatformState;
use dashcore_rpc::json::QuorumInfoResult;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::{abci, crypto};

/// The validator set is only slightly different from a quorum as it does not contain non valid
/// members
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ValidatorSetV0 {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// Active height
    pub core_height: u32,
    /// The list of masternodes
    pub members: BTreeMap<ProTxHash, ValidatorV0>,
    /// The threshold quorum public key
    pub threshold_public_key: BlsPublicKey,
}

impl Debug for ValidatorSetV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorSetV0")
            .field("quorum_hash", &self.quorum_hash.to_string())
            .field("core_height", &self.core_height)
            .field(
                "members",
                &self
                    .members
                    .iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<BTreeMap<String, &ValidatorV0>>(),
            )
            .field("threshold_public_key", &self.threshold_public_key)
            .finish()
    }
}

impl ValidatorSetV0 {
    /// For changes between two validator sets, we take the new (rhs) element if is different
    /// for every validator
    #[allow(dead_code)]
    pub(crate) fn update_difference(
        &self,
        rhs: &ValidatorSetV0,
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
                                    pub_key: public_key.clone().map(|public_key| {
                                        crypto::PublicKey {
                                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                                        }
                                    }),
                                    power: 100,
                                    pro_tx_hash: reverse(pro_tx_hash.as_byte_array()),
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
                                    pub_key: public_key.clone().map(|public_key| {
                                        crypto::PublicKey {
                                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                                        }
                                    }),
                                    power: 100,
                                    pro_tx_hash: reverse(&pro_tx_hash.to_byte_array()),
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
            quorum_hash: reverse(&self.quorum_hash.to_byte_array()),
        })
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

/// In this case we are changing to this validator set from another validator set and there are no
/// changes
impl From<ValidatorSetV0> for ValidatorSetUpdate {
    fn from(value: ValidatorSetV0) -> Self {
        let ValidatorSetV0 {
            quorum_hash,
            members: validator_set,
            threshold_public_key,
            ..
        } = value;
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
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: reverse(&pro_tx_hash.to_byte_array()),
                        node_address,
                    })
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: reverse(&quorum_hash.to_byte_array()),
        }
    }
}

impl From<&ValidatorSetV0> for ValidatorSetUpdate {
    fn from(value: &ValidatorSetV0) -> Self {
        let ValidatorSetV0 {
            quorum_hash,
            members: validator_set,
            threshold_public_key,
            ..
        } = value;
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
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: reverse(&pro_tx_hash.to_byte_array()),
                        node_address,
                    })
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: reverse(&quorum_hash.to_byte_array()),
        }
    }
}

impl ValidatorSetV0 {
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

                let validator = ValidatorV0::new_validator_if_masternode_in_state(
                    quorum_member.pro_tx_hash,
                    public_key,
                    state,
                )?;
                Some(Ok((quorum_member.pro_tx_hash, validator)))
            })
            .collect::<Result<BTreeMap<ProTxHash, ValidatorV0>, Error>>()?;

        let threshold_public_key = BlsPublicKey::from_bytes(quorum_public_key.as_slice())
            .map_err(ExecutionError::BlsErrorFromDashCoreResponse)?;

        Ok(ValidatorSetV0 {
            quorum_hash,
            core_height: height,
            members: validator_set,
            threshold_public_key,
        })
    }
}

/// Trait providing getter methods for `ValidatorSetV0` struct
pub trait ValidatorSetV0Getters {
    /// Returns the quorum hash of the validator set.
    fn quorum_hash(&self) -> &QuorumHash;
    /// Returns the active height of the validator set.
    fn core_height(&self) -> u32;
    /// Returns the members of the validator set.
    fn members(&self) -> &BTreeMap<ProTxHash, ValidatorV0>;
    /// Returns the members of the validator set.
    fn members_mut(&mut self) -> &mut BTreeMap<ProTxHash, ValidatorV0>;
    /// Returns the members of the validator set.
    fn members_owned(self) -> BTreeMap<ProTxHash, ValidatorV0>;
    /// Returns the threshold public key of the validator set.
    fn threshold_public_key(&self) -> &BlsPublicKey;
}

/// Trait providing setter methods for `ValidatorSetV0` struct
pub trait ValidatorSetV0Setters {
    /// Sets the quorum hash of the validator set.
    fn set_quorum_hash(&mut self, quorum_hash: QuorumHash);
    /// Sets the active height of the validator set.
    fn set_core_height(&mut self, core_height: u32);
    /// Sets the members of the validator set.
    fn set_members(&mut self, members: BTreeMap<ProTxHash, ValidatorV0>);
    /// Sets the threshold public key of the validator set.
    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey);
}

impl ValidatorSetV0Getters for ValidatorSetV0 {
    fn quorum_hash(&self) -> &QuorumHash {
        &self.quorum_hash
    }

    fn core_height(&self) -> u32 {
        self.core_height
    }

    fn members(&self) -> &BTreeMap<ProTxHash, ValidatorV0> {
        &self.members
    }

    fn members_mut(&mut self) -> &mut BTreeMap<ProTxHash, ValidatorV0> {
        &mut self.members
    }

    fn members_owned(self) -> BTreeMap<ProTxHash, ValidatorV0> {
        self.members
    }

    fn threshold_public_key(&self) -> &BlsPublicKey {
        &self.threshold_public_key
    }
}

impl ValidatorSetV0Setters for ValidatorSetV0 {
    fn set_quorum_hash(&mut self, quorum_hash: QuorumHash) {
        self.quorum_hash = quorum_hash;
    }

    fn set_core_height(&mut self, core_height: u32) {
        self.core_height = core_height;
    }

    fn set_members(&mut self, members: BTreeMap<ProTxHash, ValidatorV0>) {
        self.members = members;
    }

    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey) {
        self.threshold_public_key = threshold_public_key;
    }
}
