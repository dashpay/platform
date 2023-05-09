use crate::frequency::Frequency;
use crate::strategy::{ChainExecutionParameters, StrategyRandomness};
use dapi_grpc::platform::v0::{
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse,
};
use dpp::identity::{Identity, KeyID, PartialIdentity};
use dpp::serialization_traits::PlatformDeserializable;
use drive::drive::verify::RootHash;
use drive::drive::Drive;
use drive_abci::abci::AbciApplication;
use drive_abci::config::PlatformConfig;
use drive_abci::rpc::core::MockCoreRPCLike;
use prost::Message;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct QueryStrategy {
    pub query_identities_by_public_key_hashes: Frequency,
}

#[derive(Debug)]
pub struct ProofVerification<'a> {
    pub root_app_hash: &'a [u8; 32],
    pub block_signature: &'a Vec<u8>,
    pub quorum_hash: &'a [u8; 32],
}

impl QueryStrategy {
    pub(crate) fn query_chain_for_strategy(
        &self,
        proof_verification: &ProofVerification,
        current_identities: &Vec<Identity>,
        abci_app: &AbciApplication<MockCoreRPCLike>,
        seed: StrategyRandomness,
    ) {
        let mut rng = match seed {
            StrategyRandomness::SeedEntropy(seed) => StdRng::seed_from_u64(seed),
            StrategyRandomness::RNGEntropy(rng) => rng,
        };
        let QueryStrategy {
            query_identities_by_public_key_hashes,
        } = self;
        if query_identities_by_public_key_hashes.is_set() {
            Self::query_identities_by_public_key_hashes(
                proof_verification,
                current_identities,
                query_identities_by_public_key_hashes,
                abci_app,
                &mut rng,
            );
        }
    }

    pub(crate) fn query_identities_by_public_key_hashes(
        proof_verification: &ProofVerification,
        current_identities: &Vec<Identity>,
        frequency: &Frequency,
        abci_app: &AbciApplication<MockCoreRPCLike>,
        rng: &mut StdRng,
    ) {
        let events = frequency.events_if_hit(rng);

        for _i in 0..events {
            let identity_count = rng.gen_range(1..10);
            let chosen_identities = current_identities.choose_multiple(rng, identity_count);
            let public_key_hashes = chosen_identities
                .into_iter()
                .filter_map(|identity| {
                    let unique_public_keys: Vec<_> = identity
                        .public_keys
                        .iter()
                        .filter(|(_, public_key)| public_key.key_type.is_unique_key_type())
                        .collect();

                    if unique_public_keys.is_empty() {
                        None
                    } else {
                        let key_num = rng.gen_range(0..unique_public_keys.len());
                        let public_key = unique_public_keys[key_num].1;
                        Some((
                            public_key.hash().unwrap(),
                            identity.clone().into_partial_identity_info_no_balance(),
                        ))
                    }
                })
                .collect::<HashMap<[u8; 20], PartialIdentity>>();

            let prove: bool = rng.gen();

            let request = GetIdentitiesByPublicKeyHashesRequest {
                public_key_hashes: public_key_hashes
                    .iter()
                    .map(|(hash, _)| hash.to_vec())
                    .collect(),
                prove,
            };
            let encoded_request = request.encode_to_vec();
            let query_validation_result = abci_app
                .platform
                .query("/identities/by-public-key-hash", encoded_request.as_slice())
                .expect("expected to run query");

            assert!(
                query_validation_result.errors.is_empty(),
                "{:?}",
                query_validation_result.errors
            );

            let query_data = query_validation_result
                .into_data()
                .expect("expected data on query_validation_result");
            let response = GetIdentitiesByPublicKeyHashesResponse::decode(query_data.as_slice())
                .expect("expected to deserialize");
            if prove {
                let proof = response.proof.expect("expect to receive proof back");
                let (proof_root_hash, identities): (RootHash, HashMap<[u8; 20], Option<Identity>>) =
                    Drive::verify_full_identities_by_public_key_hashes(
                        &proof.grovedb_proof,
                        public_key_hashes
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .as_slice(),
                    )
                    .expect("expected to verify proof");
                let identities: HashMap<[u8; 20], PartialIdentity> = identities
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            v.expect("expect an identity")
                                .into_partial_identity_info_no_balance(),
                        )
                    })
                    .collect();
                assert_eq!(proof_verification.root_app_hash, &proof_root_hash);
                assert_eq!(identities, public_key_hashes);
            } else {
                let identities_returned = response
                    .identities
                    .into_iter()
                    .map(|serialized| {
                        Identity::deserialize(&serialized)
                            .expect("expected to deserialize identity")
                            .id
                    })
                    .collect::<HashSet<_>>();
                assert_eq!(
                    identities_returned,
                    public_key_hashes
                        .values()
                        .map(|partial_identity| partial_identity.id)
                        .collect()
                );
            }
        }
    }
}
