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
