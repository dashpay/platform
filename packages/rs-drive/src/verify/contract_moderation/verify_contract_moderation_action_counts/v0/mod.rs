use crate::drive::contract::moderation::types::decode_moderation_action_count_entry;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn verify_contract_moderation_action_counts_v0(
        proof: &[u8],
        contract_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<Identifier, u32>), Error> {
        let path_query =
            Self::contract_moderation_action_counts_query(contract_id.to_buffer(), None);
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let counts = proved_key_values
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)))
            .map(|(key, element)| {
                decode_moderation_action_count_entry(&key, &element).map_err(|description| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "contract moderation action counts proof is malformed: {}",
                        description
                    )))
                })
            })
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok((root_hash, counts))
    }
}
