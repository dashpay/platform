use crate::error::execution::ExecutionError;
use crate::error::Error;
use bls_signatures::PublicKey as BlsPublicKey;
use dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::json::QuorumInfoResult;
use std::collections::BTreeMap;

/// Quorum information
#[derive(Clone)]
pub struct Quorum {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// The list of masternodes
    pub validator_set: BTreeMap<ProTxHash, ValidatorWithPublicKeyShare>,
    /// The threshold quorum public key
    pub threshold_public_key: BlsPublicKey,
}

impl TryFrom<QuorumInfoResult> for Quorum {
    type Error = Error;

    fn try_from(value: QuorumInfoResult) -> Result<Self, Self::Error> {
        let QuorumInfoResult {
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
            };

            Ok((quorum_member.pro_tx_hash, validator))
        }).collect::<Result<BTreeMap<ProTxHash, ValidatorWithPublicKeyShare>, Error>>()?;

        Ok(Quorum {
            quorum_hash,
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
}
