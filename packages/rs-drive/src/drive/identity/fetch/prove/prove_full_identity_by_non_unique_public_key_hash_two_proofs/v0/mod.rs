use crate::drive::Drive;

use crate::error::Error;

use crate::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the first index entry after the cursor and, when there is one, that
    /// identity, as two proofs of the same state. Serves request V0; query-only,
    /// never reached from block execution.
    pub(super) fn prove_full_identity_by_non_unique_public_key_hash_two_proofs_v0(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityAndNonUniquePublicKeyHashDoubleProof, Error> {
        let identity_ids = self.fetch_identity_ids_by_non_unique_public_key_hash_operations(
            public_key_hash,
            Some(1),
            after,
            transaction,
            &mut vec![],
            platform_version,
        )?;
        // We only prove the absence of the public key hash
        let mut path_query =
            Self::identity_id_by_non_unique_public_key_hash_query(public_key_hash, after);
        path_query.query.limit = Some(1);
        let identity_id_public_key_hash_proof = self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        let identity_proof = if let Some(identity_id) = identity_ids.first() {
            let full_identity_query =
                Self::full_identity_query(identity_id, &platform_version.drive.grove_version)?;
            Some(self.grove_get_proved_path_query(
                &full_identity_query,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?)
        } else {
            None
        };

        Ok(IdentityAndNonUniquePublicKeyHashDoubleProof {
            identity_proof,
            identity_id_public_key_hash_proof,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;

    /// Request V0 is served at every protocol version, so the two proofs must verify
    /// under the protocol 13 tables (the last without V1) and the latest ones alike.
    fn served_versions() -> [&'static PlatformVersion; 2] {
        [
            PlatformVersion::get(13).expect("protocol 13"),
            PlatformVersion::latest(),
        ]
    }

    fn prove_and_verify(
        drive: &Drive,
        key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Option<Identity> {
        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash_two_proofs(
                key_hash,
                after,
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs(
            &proof.identity_id_public_key_hash_proof,
            proof.identity_proof.as_deref(),
            key_hash,
            after,
            platform_version,
            platform_version,
        )
        .expect("expect that this be verified")
        .1
    }

    #[test]
    fn should_prove_a_single_identity_with_non_unique_key() {
        for platform_version in served_versions() {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a random identity");

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add an identity");

            let first_key_hash = identity
                .public_keys()
                .values()
                .find(|public_key| !public_key.key_type().is_unique_key_type())
                .expect("expected a non-unique key")
                .public_key_hash()
                .expect("expected to hash data");

            assert_eq!(
                prove_and_verify(&drive, first_key_hash, None, platform_version),
                Some(identity),
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    #[test]
    fn should_prove_a_single_identity_with_non_unique_key_when_two_have_same_key() {
        for platform_version in served_versions() {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let mut identity_1 = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a random identity");

            let mut identity_2 = Identity::random_identity(3, Some(15), platform_version)
                .expect("expected a random identity");

            let mut rng = StdRng::seed_from_u64(506);

            let key = IdentityPublicKey::random_voting_key_with_rng(3, &mut rng, platform_version)
                .expect("expected key")
                .0;

            identity_1.add_public_key(key.clone());
            identity_2.add_public_key(key.clone());

            for identity in [&identity_1, &identity_2] {
                drive
                    .add_new_identity(
                        identity.clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add an identity");
            }

            let key_hash = key.public_key_hash().expect("expected key hash");
            let (first, second) = if identity_1.id() < identity_2.id() {
                (&identity_1, &identity_2)
            } else {
                (&identity_2, &identity_1)
            };

            assert_eq!(
                prove_and_verify(&drive, key_hash, None, platform_version).as_ref(),
                Some(first)
            );
            assert_eq!(
                prove_and_verify(
                    &drive,
                    key_hash,
                    Some(first.id().to_buffer()),
                    platform_version
                )
                .as_ref(),
                Some(second)
            );
            assert_eq!(
                prove_and_verify(
                    &drive,
                    key_hash,
                    Some(second.id().to_buffer()),
                    platform_version
                ),
                None
            );
        }
    }
}
