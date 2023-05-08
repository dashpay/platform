use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::state::PlatformState;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, PubkeyHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem};
use dashcore_rpc::json::QuorumInfoResult;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::{abci, crypto};

/// Quorum information
#[derive(Clone, Debug)]
pub struct Quorum {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// Active height
    pub core_height: u32,
    /// The list of masternodes
    pub validator_set: BTreeMap<ProTxHash, Validator>,
    /// The threshold quorum public key
    pub threshold_public_key: BlsPublicKey,
}

impl From<Quorum> for ValidatorSetUpdate {
    fn from(value: Quorum) -> Self {
        let Quorum {
            quorum_hash,
            validator_set,
            threshold_public_key,
            ..
        } = value;
        ValidatorSetUpdate {
            validator_updates: validator_set
                .into_values()
                .map(|validator| {
                    let Validator {
                        pro_tx_hash,
                        public_key,
                        node_ip,
                        node_id,
                        platform_p2p_port,
                        ..
                    } = validator;
                    let node_address = format!(
                        "tcp://{}@{}:{}",
                        hex::encode(node_id.into_inner()),
                        node_ip,
                        platform_p2p_port
                    );

                    abci::ValidatorUpdate {
                        pub_key: Some(crypto::PublicKey {
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: reverse(&pro_tx_hash),
                        node_address,
                    }
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

impl From<&Quorum> for ValidatorSetUpdate {
    fn from(value: &Quorum) -> Self {
        let Quorum {
            quorum_hash,
            validator_set,
            threshold_public_key,
            ..
        } = value;
        ValidatorSetUpdate {
            validator_updates: validator_set
                .iter()
                .map(|(_, validator)| {
                    let Validator {
                        pro_tx_hash,
                        public_key,
                        node_ip,
                        node_id,
                        platform_p2p_port,
                        ..
                    } = validator;
                    let node_address = format!(
                        "tcp://{}@{}:{}",
                        hex::encode(node_id.into_inner()),
                        node_ip,
                        platform_p2p_port
                    );
                    abci::ValidatorUpdate {
                        pub_key: Some(crypto::PublicKey {
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: reverse(pro_tx_hash),
                        node_address,
                    }
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: reverse(quorum_hash),
        }
    }
}

impl Quorum {
    /// Try to create a quorum from info from the Masternode list (given with state),
    /// and for information return for quorum members
    pub fn try_from_info_result(
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

        let validator_set = members.into_iter().filter_map(|quorum_member| {
            if !quorum_member.valid {
                return None;
            }

            let Some(pub_key_share) = quorum_member.pub_key_share else {
                tracing::debug!(method = "quorum::try_from_info_result", "No public key share for quorum member {}", quorum_member.pro_tx_hash);
                return None;
            };

            let public_key = match BlsPublicKey::from_bytes(pub_key_share.as_slice()).map_err(ExecutionError::BlsErrorFromDashCoreResponse) {
                Ok(public_key) => public_key,
                Err(e) => return Some(Err(e.into())),
            };
            let validator = Validator::new_validator_if_masternode_in_state(quorum_member.pro_tx_hash, public_key, state)?;
            Some(Ok((quorum_member.pro_tx_hash, validator)))
        }).collect::<Result<BTreeMap<ProTxHash, Validator>, Error>>()?;

        let threshold_public_key = BlsPublicKey::from_bytes(quorum_public_key.as_slice())
            .map_err(ExecutionError::BlsErrorFromDashCoreResponse)?;

        Ok(Quorum {
            quorum_hash,
            core_height: height,
            validator_set,
            threshold_public_key,
        })
    }
}

/// A validator in the context of a quorum
#[derive(Clone, Debug)]
pub struct Validator {
    /// The proTxHash
    pub pro_tx_hash: ProTxHash,
    /// The public key share of this validator for this quorum
    pub public_key: BlsPublicKey,
    /// The node address
    pub node_ip: String,
    /// The node id
    pub node_id: PubkeyHash,
    /// Core port
    pub core_port: u16,
    /// Http port
    pub platform_http_port: u16,
    /// Tenderdash port
    pub platform_p2p_port: u16,
}

impl Validator {
    /// Makes a validator if the masternode is in the list and is valid
    pub fn new_validator_if_masternode_in_state(
        pro_tx_hash: ProTxHash,
        public_key: BlsPublicKey,
        state: &PlatformState,
    ) -> Option<Self> {
        let MasternodeListItem { state, .. } = state.hpmn_masternode_list.get(&pro_tx_hash)?;

        let DMNState {
            service,
            platform_node_id,
            pose_ban_height,
            platform_p2p_port,
            platform_http_port,
            ..
        } = state;
        if pose_ban_height.is_some() {
            // if we are banned then we remove the validator from the list
            return None;
        };
        let Some(platform_http_port) = platform_http_port else {
            return None;
        };
        let Some(platform_p2p_port) = platform_p2p_port else {
            return None;
        };
        let platform_node_id = (*platform_node_id)?;
        Some(Validator {
            pro_tx_hash,
            public_key,
            node_ip: service.ip().to_string(),
            node_id: PubkeyHash::from_inner(platform_node_id),
            core_port: service.port(),
            platform_http_port: *platform_http_port as u16,
            platform_p2p_port: *platform_p2p_port as u16,
        })
    }
}
