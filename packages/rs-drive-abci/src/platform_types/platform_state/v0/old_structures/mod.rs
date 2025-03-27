#[cfg(feature = "bls-signatures")]
use dpp::bls_signatures::PublicKey;
use dpp::dashcore::{ProTxHash, PubkeyHash, QuorumHash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OldStructureValidatorSet {
    /// Version 0
    V0(OldStructureValidatorSetV0),
}

impl From<OldStructureValidatorSet> for dpp::core_types::validator_set::ValidatorSet {
    fn from(value: OldStructureValidatorSet) -> Self {
        match value {
            OldStructureValidatorSet::V0(v0) => {
                dpp::core_types::validator_set::ValidatorSet::V0(v0.into())
            }
        }
    }
}

/// The validator set is only slightly different from a quorum as it does not contain non valid
/// members
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OldStructureValidatorSetV0 {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// Rotation quorum index is available only for DIP24 quorums
    pub quorum_index: Option<u32>,
    /// Active height
    pub core_height: u32,
    /// The list of masternodes
    pub members: BTreeMap<ProTxHash, ValidatorV0>,
    /// The threshold quorum public key
    #[cfg(feature = "bls-signatures")]
    pub threshold_public_key: bls_signatures::PublicKey,

    /// Extra fields for forward compatibility.
    #[cfg(not(feature = "bls-signatures"))]
    #[serde(flatten)]
    pub extra_fields: std::collections::HashMap<String, serde_json::Value>,
}

impl From<OldStructureValidatorSetV0> for dpp::core_types::validator_set::v0::ValidatorSetV0 {
    fn from(value: OldStructureValidatorSetV0) -> Self {
        let OldStructureValidatorSetV0 {
            quorum_hash,
            quorum_index,
            core_height,
            members,
            #[cfg(feature = "bls-signatures")]
            threshold_public_key,
            ..
        } = value;
        Self {
            quorum_hash,
            quorum_index,
            core_height,
            members: members
                .into_iter()
                .map(|(pro_tx_hash, validator)| (pro_tx_hash, validator.into()))
                .collect(),
            #[cfg(feature = "bls-signatures")]
            threshold_public_key: PublicKey::try_from(threshold_public_key.to_bytes().as_slice())
                .expect("this should not be possible to error as the threshold_public_key was already verified on disk"),
            #[cfg(not(feature = "bls-signatures"))]
            threshold_public_key: Default::default(),
        }
    }
}

/// A validator in the context of a quorum
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorV0 {
    /// The proTxHash
    pub pro_tx_hash: ProTxHash,
    /// The public key share of this validator for this quorum
    #[cfg(feature = "bls-signatures")]
    pub public_key: Option<bls_signatures::PublicKey>,
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
    /// Is the validator banned
    pub is_banned: bool,
}

impl From<ValidatorV0> for dpp::core_types::validator::v0::ValidatorV0 {
    fn from(value: ValidatorV0) -> Self {
        let ValidatorV0 {
            pro_tx_hash,
            #[cfg(feature = "bls-signatures")]
            public_key,
            node_ip,
            node_id,
            core_port,
            platform_http_port,
            platform_p2p_port,
            is_banned,
        } = value;
        Self {
            pro_tx_hash,
            #[cfg(feature = "bls-signatures")]
            public_key: public_key.map(|pk| PublicKey::try_from(pk.to_bytes().as_slice()).expect("this should not be possible to error as the public_key was already verified on disk")),
            #[cfg(not(feature = "bls-signatures"))]
            public_key: None,
            node_ip,
            node_id,
            core_port,
            platform_http_port,
            platform_p2p_port,
            is_banned,
        }
    }
}
