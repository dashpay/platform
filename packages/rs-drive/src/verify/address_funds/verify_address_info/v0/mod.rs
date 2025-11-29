use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::AddressNonce;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_address_info_v0(
        proof: &[u8],
        key_of_type: &KeyOfType,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<(AddressNonce, Credits)>), Error> {
        let path_query = Self::balance_for_address_query(key_of_type);

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
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

        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we should always get back one element".to_string(),
            )));
        }

        let element = proved_key_values.remove(0).2;

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
                let nonce = AddressNonce::from_be_bytes(nonce_bytes);

                if balance_i64 < 0 {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "balance cannot be negative".to_string(),
                    )));
                }
                let balance = balance_i64 as Credits;

                Ok((nonce, balance))
            })
            .transpose()?;

        Ok((root_hash, balance_info))
    }
}
