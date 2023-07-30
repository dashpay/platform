use crate::drive::balances::balance_path;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{identity_key_tree_path, identity_path};
use crate::drive::{unique_key_hashes_tree_path_vec, Drive};

use crate::error::proof::ProofError;
use crate::error::Error;
use dpp::fee::Credits;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
pub use dpp::prelude::{Identity, Revision};
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies the full identities of multiple users by their public key hashes.
    ///
    /// This function is a generalization of `verify_full_identity_by_public_key_hash`,
    /// which works with a slice of public key hashes instead of a single hash.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the users.
    /// - `public_key_hashes`: A reference to a slice of 20-byte arrays, each representing
    ///    a hash of a public key of a user.
    ///
    /// # Generic Parameters
    ///
    /// - `T`: The type of the collection to hold the results, which must be constructible
    ///    from an iterator of tuples of a 20-byte array and an optional `Identity`.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and `T`.
    /// The `RootHash` represents the root hash of GroveDB, and `T` represents
    /// the collection of the public key hash and associated identity (if it exists) for each user.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - Any of the public key hashes do not correspond to a valid identity ID.
    /// - Any of the identity IDs do not correspond to a valid full identity.
    ///
    pub(super) fn verify_full_identities_by_public_key_hashes_v0<
        T: FromIterator<([u8; 20], Option<Identity>)>,
    >(
        proof: &[u8],
        public_key_hashes: &[[u8; 20]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let (root_hash, identity_ids_by_key_hashes) =
            Self::verify_identity_ids_by_public_key_hashes::<Vec<(_, _)>>(
                proof,
                true,
                public_key_hashes,
                platform_version,
            )?;
        let maybe_identity = identity_ids_by_key_hashes
            .into_iter()
            .map(|(key_hash, identity_id)| match identity_id {
                None => Ok((key_hash, None)),
                Some(identity_id) => {
                    let identity = Self::verify_full_identity_by_identity_id(
                        proof,
                        true,
                        identity_id,
                        platform_version,
                    )
                    .map(|(_, maybe_identity)| maybe_identity)?;
                    let identity = identity.ok_or(Error::Proof(ProofError::IncompleteProof(
                        "proof returned an identity id without identity information",
                    )))?;
                    Ok((key_hash, Some(identity)))
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, maybe_identity))
    }
}
