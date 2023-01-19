use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Identity, Revision};
use grovedb::operations::proof::verify::ProvedKeyValue;
use grovedb::{Element, GroveDb};
use std::collections::BTreeMap;

pub type RootHash = [u8; 32];

impl Drive {
    /// Verifies the identity with a public key hash
    pub fn verify_full_identity_by_public_key_hash(
        proof: &[u8],
        public_key_hash: [u8; 20],
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let (root_hash, identity_id) =
            Self::verify_identity_id_by_public_key_hash(proof, public_key_hash)?;
        let maybe_identity = identity_id
            .map(|identity_id| {
                Self::verify_full_identity_by_identity_id(proof, identity_id)
                    .map(|(_, maybe_identity)| maybe_identity)
            })
            .transpose()?
            .flatten();
        Ok((root_hash, maybe_identity))
    }

    /// Verifies the identity with its identity id
    pub fn verify_full_identity_by_identity_id(
        proof: &[u8],
        identity_id: [u8; 32],
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let path_query = Self::full_identity_query(&identity_id)?;
        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;
        let mut balance = None;
        let mut revision = None;
        let mut keys = BTreeMap::<KeyID, IdentityPublicKey>::new();
        for proved_key_value in proved_key_values {
            let ProvedKeyValue { key, value, .. } = proved_key_value;
            let element = Element::deserialize(&value)?;

            if key == identity_id {
                //this is the balance
                let signed_balance = element.as_sum_item_value().map_err(Error::GroveDB)?;
                if signed_balance < 0 {
                    return Err(Error::Proof(ProofError::Overflow(
                        "balance can't be negative",
                    )));
                }
                balance = Some(signed_balance as u64);
                continue;
            }
            let item_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;

            if key == vec![IdentityTreeRevision as u8] && item_bytes.len() == 8 {
                //this is the revision
                revision = Some(Revision::from_be_bytes(item_bytes.try_into().map_err(
                    |_| Error::Proof(ProofError::IncorrectValueSize("revision should be 8 bytes")),
                )?));
                continue;
            }

            let key = IdentityPublicKey::deserialize(&item_bytes)?;
            keys.insert(key.id, key);
        }
        let maybe_identity = if balance.is_none() && revision.is_none() && keys.is_empty() {
            Ok(None)
        } else if balance.is_none() || revision.is_none() || keys.is_empty() {
            // that means that one has stuff and the others don't
            // this is an error
            Err(Error::Proof(ProofError::IncompleteProof(
                "identity proof is incomplete",
            )))
        } else {
            Ok(Some(Identity {
                protocol_version: PROTOCOL_VERSION,
                id: Identifier::from(identity_id),
                public_keys: keys,
                balance: balance.unwrap(),
                revision: revision.unwrap(),
                asset_lock_proof: None,
                metadata: None,
            }))
        }?;
        Ok((root_hash, maybe_identity))
    }

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

    /// Verifies the identity's balance with a identity id
    pub fn verify_identity_balance_for_identity_id(
        proof: &[u8],
        identity_id: [u8; 32],
    ) -> Result<(RootHash, Option<u64>), Error> {
        let path_query = Self::identity_balance_query(&identity_id);
        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;
        if proved_key_values.len() == 1 {
            let value = &proved_key_values.get(0).unwrap().value;
            let element = Element::deserialize(value)?;
            let signed_balance = element.as_sum_item_value().map_err(Error::GroveDB)?;
            if signed_balance < 0 {
                return Err(Error::Proof(ProofError::Overflow(
                    "balance can't be negative",
                )));
            }
            //todo there shouldn't be a some here
            Ok((
                root_hash,
                Some(signed_balance as u64),
            ))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one identity balance",
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
