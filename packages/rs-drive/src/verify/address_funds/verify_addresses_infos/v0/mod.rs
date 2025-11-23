use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::KeyOfTypeNonce;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_addresses_infos_v0<
        'a,
        I: IntoIterator<Item = &'a KeyOfType>,
        T: FromIterator<(KeyOfType, Option<(KeyOfTypeNonce, Credits)>)>,
    >(
        proof: &[u8],
        keys_of_type: I,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::balances_for_addresses_query(keys_of_type);

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        let values = proved_key_values
            .into_iter()
            .map(|(_path, key, element)| {
                // Reconstruct KeyOfType from the key bytes
                let key_of_type = KeyOfType::from_bytes(&key).map_err(|e| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "failed to deserialize KeyOfType: {}",
                        e
                    )))
                })?;

                let balance_info = element
                    .map(|element| {
                        let Element::ItemWithSumItem(nonce_vec, balance_i64, _) = element else {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "expected an item with sum item element".to_string(),
                            )));
                        };

                        let nonce_bytes: [u8; 8] = nonce_vec.try_into().map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize("nonce should be 8 bytes"))
                        })?;
                        let nonce = KeyOfTypeNonce::from_be_bytes(nonce_bytes);

                        if balance_i64 < 0 {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "balance cannot be negative".to_string(),
                            )));
                        }
                        let balance = balance_i64 as Credits;

                        Ok((nonce, balance))
                    })
                    .transpose()?;

                Ok((key_of_type, balance_info))
            })
            .collect::<Result<T, Error>>()?;

        Ok((root_hash, values))
    }
}
