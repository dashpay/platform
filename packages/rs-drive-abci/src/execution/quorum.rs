use crate::error::execution::ExecutionError;
use crate::error::Error;
use dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::json::QuorumInfoResult;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::{abci, crypto};

/// Quorum information
#[derive(Clone)]
pub struct Quorum {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// Active height
    pub core_height: u32,
    /// The list of masternodes
    pub validator_set: BTreeMap<ProTxHash, ValidatorWithPublicKeyShare>,
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
                .into_iter()
                .map(|(_, validator_with_public_key_share)| {
                    let ValidatorWithPublicKeyShare {
                        pro_tx_hash,
                        public_key,
                        node_address,
                    } = validator_with_public_key_share;
                    abci::ValidatorUpdate {
                        pub_key: Some(crypto::PublicKey {
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: pro_tx_hash.to_vec(),
                        node_address,
                    }
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: quorum_hash.to_vec(),
        }
    }
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
                .map(|(_, validator_with_public_key_share)| {
                    let ValidatorWithPublicKeyShare {
                        pro_tx_hash,
                        public_key,
                        node_address,
                    } = validator_with_public_key_share;
                    abci::ValidatorUpdate {
                        pub_key: Some(crypto::PublicKey {
                            sum: Some(Bls12381(public_key.to_bytes().to_vec())),
                        }),
                        power: 100,
                        pro_tx_hash: pro_tx_hash.to_vec(),
                        node_address: node_address.clone(),
                    }
                })
                .collect(),
            threshold_public_key: Some(crypto::PublicKey {
                sum: Some(Bls12381(threshold_public_key.to_bytes().to_vec())),
            }),
            quorum_hash: quorum_hash.to_vec(),
        }
    }
}

impl TryFrom<QuorumInfoResult> for Quorum {
    type Error = Error;

    fn try_from(value: QuorumInfoResult) -> Result<Self, Self::Error> {
        let QuorumInfoResult {
            height,
            quorum_hash,
            quorum_public_key,
            members,
            ..
        } = value;

        let validator_set = members.into_iter().map(|quorum_member| {
            let Some(pub_key_share) = quorum_member.pub_key_share else {
                //todo: check to make sure there are no cases where this could be "normal" from core's side
                return Err(Error::Execution(ExecutionError::DashCoreBadResponseError("quorum member did not have a public key share".to_string())));
            };

            let public_key = BlsPublicKey::from_bytes(pub_key_share.as_slice()).map_err(ExecutionError::BlsErrorFromDashCoreResponse)?;
            let validator = ValidatorWithPublicKeyShare {
                pro_tx_hash: quorum_member.pro_tx_hash,
                public_key,
                node_address: "".to_string(), // FixMe
            };

            Ok((quorum_member.pro_tx_hash, validator))
        }).collect::<Result<BTreeMap<ProTxHash, ValidatorWithPublicKeyShare>, Error>>()?;

        Ok(Quorum {
            quorum_hash,
            core_height: height as u32,
            validator_set,
            threshold_public_key: BlsPublicKey::from_bytes(quorum_public_key.as_slice())
                .map_err(ExecutionError::BlsErrorFromDashCoreResponse)?,
        })
    }
}

/// A validator in the context of a quorum
#[derive(Clone)]
pub struct ValidatorWithPublicKeyShare {
    /// The proTxHash
    pub pro_tx_hash: ProTxHash,
    /// The public key share of this validator for this quorum
    pub public_key: BlsPublicKey,
    /// node_address is an URI containing address of validator (proto://node_id@ip_address:port),
    /// for example: tcp://f2dbd9b0a1f541a7c44d34a58674d0262f5feca5@12.34.5.6:1234
    pub node_address: String,
}
