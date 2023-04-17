use bls_signatures;
use bls_signatures::{PrivateKey as BlsPrivateKey, PublicKey as BlsPublicKey};
use dashcore::hashes::Hash;
use dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{
    MasternodeListItem, QuorumInfoResult, QuorumMember, QuorumType,
};
use drive_abci::execution::quorum::{Quorum, ValidatorWithPublicKeyShare};
use rand::rngs::StdRng;

#[derive(Clone)]
pub struct ValidatorInQuorum {
    pub pro_tx_hash: ProTxHash,
    pub private_key: BlsPrivateKey,
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
            pro_tx_hash: pro_tx_hash.clone(),
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

#[derive(Clone)]
pub struct TestQuorumInfo {
    pub quorum_hash: QuorumHash,
    pub validator_set: Vec<ValidatorInQuorum>,
    // in reality quorums don't have a private key,
    // however for these tests, we can just sign with a private key to mimic threshold signing
    pub private_key: BlsPrivateKey,
    pub public_key: BlsPublicKey,
}

impl TestQuorumInfo {
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
        let validator_set = bls_id_private_key_pairs
            .into_iter()
            .map(|(pro_tx_hash, key)| ValidatorInQuorum {
                pro_tx_hash: ProTxHash::from_slice(pro_tx_hash.as_slice()).expect("expected 32 bytes for pro_tx_hash"),
                private_key: key,
                public_key: key.g1_element().expect("expected to get public key"),
            })
            .collect();
        TestQuorumInfo {
            quorum_hash,
            validator_set,
            private_key: recovered_private_key,
            public_key: recovered_private_key
                .g1_element()
                .expect("expected to get G1 Element"),
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
        } = value;

        Quorum {
            quorum_hash: quorum_hash.clone(),
            validator_set: validator_set.iter().map(|v| (v.pro_tx_hash, v.into())).collect(),
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
        } = value;

        Quorum {
            quorum_hash,
            validator_set: validator_set.iter().map(|v| (v.pro_tx_hash, v.into())).collect(),
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
        } = value;
        let members = validator_set
            .into_iter()
            .map(|validator_in_quorum| {
                let ValidatorInQuorum {
                    pro_tx_hash,
                    public_key,
                    ..
                } = validator_in_quorum;
                QuorumMember {
                    pro_tx_hash: pro_tx_hash.clone(),
                    pub_key_operator: vec![], //doesn't matter
                    valid: true,
                    pub_key_share: Some(public_key.to_bytes().to_vec()),
                }
            })
            .collect();
        QuorumInfoResult {
            height: 0,
            quorum_type: QuorumType::LlmqDevnetPlatform,
            quorum_hash: quorum_hash.clone(),
            quorum_index: 0,
            mined_block: vec![],
            members,
            quorum_public_key: public_key.to_bytes().to_vec(),
            secret_key_share: None,
        }
    }
}
