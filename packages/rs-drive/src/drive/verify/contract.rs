use crate::common::encode::encode_u64;
use crate::drive::contract::paths::contract_root_path;
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use dpp::prelude::DataContract;
use dpp::serialization_traits::PlatformDeserializable;
use grovedb::GroveDb;

impl Drive {
    /// Verifies that the contract is in the Proof
    pub fn verify_contract(
        proof: &[u8],
        is_proof_subset: bool,
        contract_id: [u8; 32],
    ) -> Result<(RootHash, Option<DataContract>), Error> {
        let path_query = Self::fetch_contract_query(contract_id);
        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if path != contract_root_path(&contract_id) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path for the contract",
                )));
            }
            if key != vec![0] {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key for the contract",
                )));
            }
            let contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            DataContract::deserialize_no_limit(&bytes).map_err(Error::Protocol)
                        })
                })
                .transpose()?;
            Ok((root_hash, contract))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one contract id",
            )))
        }
    }
}
