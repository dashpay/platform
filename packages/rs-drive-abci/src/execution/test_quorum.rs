use crate::execution::quorum::{Quorum, ValidatorWithPublicKeyShare};
use dashcore::hashes::Hash;
use dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{QuorumInfoResult, QuorumMember, QuorumType};
use dpp::bls_signatures;
use dpp::bls_signatures::{PrivateKey as BlsPrivateKey, PublicKey as BlsPublicKey};
use rand::rngs::StdRng;
use std::collections::BTreeMap;

/// ValidatorInQuorum represents a validator in a quorum or consensus algorithm.
/// Each validator is identified by a `ProTxHash` and has a corresponding BLS private key and public key.
#[derive(Clone)]
pub struct ValidatorInQuorum {
    /// The hash of the transaction that identifies this validator in the network.
    pub pro_tx_hash: ProTxHash,
    /// The private key for this validator's BLS signature scheme.
    pub private_key: BlsPrivateKey,
    /// The public key for this validator's BLS signature scheme.
    pub public_key: BlsPublicKey,
}

impl From<&ValidatorInQuorum> for ValidatorWithPublicKeyShare {
    fn from(value: &ValidatorInQuorum) -> Self {
        let ValidatorInQuorum {
            pro_tx_hash,
            public_key,
            ..
        } = value;
        ValidatorWithPublicKeyShare {
            pro_tx_hash: *pro_tx_hash,
            public_key: public_key.clone(),
        }
    }
}

impl From<ValidatorInQuorum> for ValidatorWithPublicKeyShare {
    fn from(value: ValidatorInQuorum) -> Self {
        let ValidatorInQuorum {
            pro_tx_hash,
            public_key,
            ..
        } = value;
        ValidatorWithPublicKeyShare {
            pro_tx_hash,
            public_key,
        }
    }
}

/// TestQuorumInfo represents a test quorum used for threshold signing.
/// A quorum is identified by a `QuorumHash` and contains a set of validators, as well as a map of validators
/// indexed by their `ProTxHash` identifiers.
#[derive(Clone)]
pub struct TestQuorumInfo {
    /// The hash of the quorum.
    pub quorum_hash: QuorumHash,
    /// The set of validators that belong to the quorum.
    pub validator_set: Vec<ValidatorInQuorum>,
    /// A map of validators indexed by their `ProTxHash` identifiers.
    pub validator_map: BTreeMap<ProTxHash, ValidatorInQuorum>,
    /// The private key used to sign messages for the quorum (for testing purposes only).
    pub private_key: BlsPrivateKey,
    /// The public key corresponding to the private key used for signing.
    pub public_key: BlsPublicKey,
}

impl TestQuorumInfo {
    /// Constructs a new `TestQuorumInfo` object from a quorum hash and a list of `ProTxHash` identifiers.
    /// The `TestQuorumInfo` object contains a set of validators, as well as a map of validators indexed by their
    /// `ProTxHash` identifiers. The private and public keys are generated randomly using the given RNG.
    pub fn from_quorum_hash_and_pro_tx_hashes(
        quorum_hash: QuorumHash,
        pro_tx_hashes: Vec<ProTxHash>,
        rng: &mut StdRng,
    ) -> Self {
        let private_keys = bls_signatures::PrivateKey::generate_dash_many(pro_tx_hashes.len(), rng)
            .expect("expected to generate private keys");
        let bls_id_private_key_pairs = private_keys
            .into_iter()
            .zip(pro_tx_hashes)
            .map(|(private_key, pro_tx_hashes)| (pro_tx_hashes.to_vec(), private_key))
            .collect::<Vec<_>>();
        let recovered_private_key =
            bls_signatures::PrivateKey::threshold_recover(&bls_id_private_key_pairs)
                .expect("expected to recover a private key");
        let validator_set: Vec<_> = bls_id_private_key_pairs
            .into_iter()
            .map(|(pro_tx_hash, key)| {
                let public_key = key.g1_element().expect("expected to get public key");
                ValidatorInQuorum {
                    pro_tx_hash: ProTxHash::from_slice(pro_tx_hash.as_slice())
                        .expect("expected 32 bytes for pro_tx_hash"),
                    private_key: key,
                    public_key,
                }
            })
            .collect();
        let public_key = recovered_private_key
            .g1_element()
            .expect("expected to get G1 Element");
        let map = validator_set
            .iter()
            .map(|v| (v.pro_tx_hash, v.clone()))
            .collect();
        TestQuorumInfo {
            quorum_hash,
            validator_set,
            validator_map: map,
            private_key: recovered_private_key,
            public_key,
        }
    }
}

impl From<&TestQuorumInfo> for Quorum {
    fn from(value: &TestQuorumInfo) -> Self {
        let TestQuorumInfo {
            quorum_hash,
            validator_set,
            private_key: _,
            public_key,
            ..
        } = value;

        Quorum {
            quorum_hash: *quorum_hash,
            validator_set: validator_set
                .iter()
                .map(|v| (v.pro_tx_hash, v.into()))
                .collect(),
            threshold_public_key: public_key.clone(),
        }
    }
}

impl From<TestQuorumInfo> for Quorum {
    fn from(value: TestQuorumInfo) -> Self {
        let TestQuorumInfo {
            quorum_hash,
            validator_set,
            private_key: _,
            public_key,
            ..
        } = value;

        Quorum {
            quorum_hash,
            validator_set: validator_set
                .iter()
                .map(|v| (v.pro_tx_hash, v.into()))
                .collect(),
            threshold_public_key: public_key,
        }
    }
}

impl From<&TestQuorumInfo> for QuorumInfoResult {
    fn from(value: &TestQuorumInfo) -> Self {
        let TestQuorumInfo {
            quorum_hash,
            validator_set,
            private_key: _,
            public_key,
            ..
        } = value;
        let members = validator_set
            .iter()
            .map(|validator_in_quorum| {
                let ValidatorInQuorum {
                    pro_tx_hash,
                    public_key,
                    ..
                } = validator_in_quorum;
                QuorumMember {
                    pro_tx_hash: *pro_tx_hash,
                    pub_key_operator: vec![], //doesn't matter
                    valid: true,
                    pub_key_share: Some(public_key.to_bytes().to_vec()),
                }
            })
            .collect();
        QuorumInfoResult {
            height: 0,
            quorum_type: QuorumType::LlmqDevnetPlatform,
            quorum_hash: *quorum_hash,
            quorum_index: 0,
            mined_block: vec![],
            members,
            quorum_public_key: public_key.to_bytes().to_vec(),
            secret_key_share: None,
        }
    }
}
