use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::proof::ProofError::IncorrectElementPath;
use crate::error::Error;
use dpp::prelude::Identity;
use grovedb::{Element, GroveDb};
use std::collections::BTreeMap;

pub type RootHash = [u8; 32];

impl Drive {
    /// Verifies the identity id with a public key hash
    pub fn verify_identity_id_by_public_key_hash(
        proof: &[u8],
        public_key_hash: [u8; 20],
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let path_query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;
        if proved_key_values.len() == 1 {
            let value = &proved_key_values.get(0).unwrap().value;
            let element = Element::deserialize(value)?;
            let identity_id = element.into_item_bytes().map_err(Error::GroveDB)?;
            //todo there shouldn't be a some here
            Ok((
                root_hash,
                Some(identity_id.try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
                })?),
            ))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one identity id",
            )))
        }
    }

    /// Verifies the identity id with a public key hash
    pub fn verify_identity_ids_by_public_key_hashes<
        T: FromIterator<([u8; 20], Option<[u8; 32]>)>,
    >(
        proof: &[u8],
        public_key_hashes: &[[u8; 20]],
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::identity_ids_by_unique_public_key_hash_query(public_key_hashes);
        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;
        if proved_key_values.len() == public_key_hashes.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let key: [u8; 20] = proved_key_value
                        .key
                        .try_into()
                        .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                    let element = Element::deserialize(proved_key_value.value.as_slice())?;
                    let identity_id = element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)?
                        .try_into()
                        .map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
                        })?;
                    Ok((key, Some(identity_id)))
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount(
                "expected one identity id",
            )))
        }
    }
}
