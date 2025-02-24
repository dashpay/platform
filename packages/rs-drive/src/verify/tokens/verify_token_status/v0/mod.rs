use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::serialization::PlatformDeserializable;
use dpp::tokens::status::TokenStatus;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_status_v0(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<TokenStatus>), Error> {
        let path_query = Self::token_status_query(token_id);
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
        if proved_key_values.len() == 1 {
            let proved_key_value = proved_key_values.remove(0);
            match proved_key_value.2 {
                Some(Item(value, ..)) => Ok((
                    root_hash,
                    Some(TokenStatus::deserialize_from_bytes(&value)?),
                )),
                None => Ok((root_hash, None)),
                _ => Err(Error::Proof(ProofError::IncorrectValueSize(
                    "proof did not point to an item",
                ))),
            }
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: proved_key_values.len(),
            }))
        }
    }
}
