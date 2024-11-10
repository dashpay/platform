use crate::drive::{non_unique_key_hashes_sub_tree_path_vec, Drive};

use crate::error::Error;

use crate::query::QueryItem;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// Given public key hash, fetches up to 5 full identities as proofs.
    pub(super) fn prove_full_identities_for_non_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let identity_ids = self.fetch_identity_ids_by_non_unique_public_key_hash(
            public_key_hash,
            limit,
            transaction,
            &platform_version.drive,
        )?;

        let non_unique_key_hashes = non_unique_key_hashes_sub_tree_path_vec(public_key_hash);
        let identity_ids_query = if let Some(last) = identity_ids.last() {
            PathQuery::new_single_query_item(
                non_unique_key_hashes,
                QueryItem::RangeToInclusive(..=last.to_vec()),
            )
        } else {
            Self::identity_ids_for_non_unique_public_key_hash_query(public_key_hash)
        };

        let mut path_queries = identity_ids
            .into_iter()
            .map(|identity_id| {
                Self::full_identity_query(&identity_id, &platform_version.drive.grove_version)
            })
            .collect::<Result<Vec<PathQuery>, Error>>()?;

        path_queries.push(identity_ids_query);

        let path_query = PathQuery::merge(
            path_queries.iter().collect(),
            &platform_version.drive.grove_version,
        )
        .map_err(Error::GroveDB)?;
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use bincode::error::DecodeError;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::IdentityPublicKey;
    use dpp::version::PlatformVersion;
    use grovedb::operations::proof::GroveDBProof;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_multiple_identities() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut identities: BTreeMap<[u8; 32], Identity> =
            Identity::random_identities(2, 3, Some(14), platform_version)
                .expect("expected random identities")
                .into_iter()
                .map(|identity| (identity.id().to_buffer(), identity))
                .collect();

        let mut rng = StdRng::seed_from_u64(394);

        let (public_key, _) = IdentityPublicKey::random_key_with_known_attributes(
            4,
            &mut rng,
            Purpose::AUTHENTICATION,
            SecurityLevel::MEDIUM,
            KeyType::ECDSA_HASH160,
            None,
            platform_version,
        )
        .expect("expected key");

        for identity in identities.values_mut() {
            identity.add_public_key(public_key.clone());
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

        let public_key_hash = public_key
            .public_key_hash()
            .expect("expected public key hash");

        let proof = drive
            .prove_full_identities_for_non_unique_public_key_hash(
                public_key_hash,
                Some(3),
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let (_, proved_identities): ([u8; 32], Vec<Identity>) =
            Drive::verify_full_identities_for_non_unique_public_key_hash(
                proof.as_slice(),
                public_key_hash,
                Some(3),
                platform_version,
            )
            .expect("expect that this be verified");

        assert_eq!(proved_identities.len(), 2);
    }

    #[test]
    fn should_prove_multiple_identities_limit_under_total() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut identities: BTreeMap<[u8; 32], Identity> =
            Identity::random_identities(10, 3, Some(14), platform_version)
                .expect("expected random identities")
                .into_iter()
                .map(|identity| (identity.id().to_buffer(), identity))
                .collect();

        let mut rng = StdRng::seed_from_u64(394);

        let (public_key, _) = IdentityPublicKey::random_key_with_known_attributes(
            4,
            &mut rng,
            Purpose::AUTHENTICATION,
            SecurityLevel::MEDIUM,
            KeyType::ECDSA_HASH160,
            None,
            platform_version,
        )
        .expect("expected key");

        for identity in identities.values_mut() {
            identity.add_public_key(public_key.clone());
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

        let public_key_hash = public_key
            .public_key_hash()
            .expect("expected public key hash");

        let proof = drive
            .prove_full_identities_for_non_unique_public_key_hash(
                public_key_hash,
                Some(3),
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let grovedb_proof: Result<GroveDBProof, DecodeError> =
            bincode::decode_from_slice(&proof, config).map(|(a, _)| a);

        let grovedb_proof_string = match grovedb_proof {
            Ok(proof) => format!("{}", proof),
            Err(_) => "Invalid GroveDBProof".to_string(),
        };
        println!("{}", grovedb_proof_string);

        let (_, proved_identities): ([u8; 32], Vec<Identity>) =
            Drive::verify_full_identities_for_non_unique_public_key_hash(
                proof.as_slice(),
                public_key_hash,
                Some(3),
                platform_version,
            )
            .expect("expect that this be verified");

        assert_eq!(proved_identities.len(), 3);
    }
}
