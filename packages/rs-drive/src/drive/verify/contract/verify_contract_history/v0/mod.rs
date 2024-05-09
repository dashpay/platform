use crate::drive::contract::paths::contract_storage_path_vec;
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;

use dpp::prelude::DataContract;
use std::collections::BTreeMap;

use crate::common::decode;
use crate::error::drive::DriveError;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    /// Verifies that the contract's history is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `contract_id`: The contract's unique identifier.
    /// - `start_at_date`: The start date for the contract's history.
    /// - `limit`: An optional limit for the number of items to be retrieved.
    /// - `offset`: An optional offset for the items to be retrieved.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<BTreeMap<u64, DataContract>>`. The `Option<BTreeMap<u64, DataContract>>`
    /// represents a mapping from dates to contracts if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - The contract serialization fails.
    #[inline(always)]
    pub(super) fn verify_contract_history_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<BTreeMap<u64, DataContract>>), Error> {
        let path_query =
            Self::fetch_contract_history_query(contract_id, start_at_date, limit, offset)?;

        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;

        let mut contracts: BTreeMap<u64, DataContract> = BTreeMap::new();
        for (path, key, maybe_element) in proved_key_values.drain(..) {
            if path != contract_storage_path_vec(&contract_id) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path for the historical contract".to_string(),
                )));
            }

            let date = decode::decode_u64(&key).map_err(|_| {
                Error::Drive(DriveError::CorruptedContractPath(
                    "contract key is not a valid u64",
                ))
            })?;

            let maybe_contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            // we don't need to validate the contract locally because it was proved to be in platform
                            // and hence it is valid
                            DataContract::versioned_deserialize(&bytes, false, platform_version)
                                .map_err(Error::Protocol)
                        })
                })
                .transpose()?;

            if let Some(contract) = maybe_contract {
                contracts.insert(date, contract);
            } else {
                return Err(Error::Drive(DriveError::CorruptedContractPath(
                    "expected a contract at this path",
                )));
            }
        }

        Ok((root_hash, Some(contracts)))
    }
}
