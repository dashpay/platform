use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::serialization::PlatformDeserializable;
use dpp::tokens::info::IdentityTokenInfo;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_infos_for_identity_id_v0<
        T: FromIterator<(I, Option<IdentityTokenInfo>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::token_infos_for_identity_id_query(token_ids, identity_id);
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
        if proved_key_values.len() == token_ids.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let token_id: [u8; 32] = proved_key_value
                        .0
                        .get(2)
                        .ok_or(Error::Proof(ProofError::IncorrectProof(
                            "path should have at least 3 elements in returned proof".to_string(),
                        )))?
                        .clone()
                        .try_into()
                        .map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize("token id size"))
                        })?;
                    match proved_key_value.2 {
                        Some(Item(value, ..)) => Ok((
                            token_id.into(),
                            Some(IdentityTokenInfo::deserialize_from_bytes(&value)?),
                        )),
                        None => Ok((token_id.into(), None)),
                        _ => Err(Error::Proof(ProofError::IncorrectProof(
                            "proof did not point to an item as expected for token info".to_string(),
                        ))),
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: token_ids.len(),
                got: proved_key_values.len(),
            }))
        }
    }
}
