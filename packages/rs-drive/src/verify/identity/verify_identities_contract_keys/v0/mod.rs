use crate::drive::Drive;

use crate::error::Error;

use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, Purpose};

use crate::error::drive::DriveError;
use dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies the identity keys of a user by their identity ID.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `PartialIdentity`. The `RootHash` represents the root hash of GroveDB,
    /// and the `Option<PartialIdentity>` represents the partial identity of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid partial identity.
    /// - The keys information is missing or incorrect.
    ///
    #[inline(always)]
    pub(crate) fn verify_identities_contract_keys_v0(
        proof: &[u8],
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        is_proof_subset: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, IdentitiesContractKeys), Error> {
        let path_query = Self::identities_contract_keys_query(
            identity_ids,
            contract_id,
            &document_type_name,
            &purposes,
            Some((identity_ids.len() * purposes.len()) as u16),
        );

        let (root_hash, proved_values) = if is_proof_subset {
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

        let mut group_id = contract_id.to_vec();
        if let Some(document_type_name) = document_type_name {
            group_id.extend(document_type_name.as_bytes());
        }

        let mut values = BTreeMap::new();

        for (path, _, maybe_element) in proved_values {
            if let Some(identity_id_bytes) = path.get(1) {
                let identity_id = Identifier::from_vec(identity_id_bytes.to_owned())?;
                // We can use expect here because we have already shown that the path must have
                //  at least 2 sub parts as we get index 1
                let purpose_bytes = path.last().expect("last path element is the purpose");
                if purpose_bytes.len() != 1 {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "purpose for identifier {} at path {} is {}, should be 1 byte",
                        identity_id,
                        path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                        hex::encode(purpose_bytes)
                    ))));
                }

                let purpose_first_byte = purpose_bytes
                    .first()
                    .expect("we have already shown there is 1 byte");

                let purpose = Purpose::try_from(*purpose_first_byte).map_err(|e| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "purpose for identifier {} at path {} has error : {}",
                        identity_id,
                        path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                        e
                    )))
                })?;

                let entry = values.entry(identity_id).or_insert(BTreeMap::new());

                let maybe_item_bytes = maybe_element
                    .as_ref()
                    .map(|element| element.as_item_bytes())
                    .transpose()?;

                let maybe_identity_public_key = maybe_item_bytes
                    .map(IdentityPublicKey::deserialize_from_bytes)
                    .transpose()?;

                entry.insert(purpose, maybe_identity_public_key);
            }
        }

        Ok((root_hash, values))
    }
}
