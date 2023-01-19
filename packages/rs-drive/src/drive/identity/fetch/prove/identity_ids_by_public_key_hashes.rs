use crate::drive::grove_operations::DirectQueryType::StatefulDirectQuery;
use crate::drive::{
    non_unique_key_hashes_sub_tree_path, non_unique_key_hashes_tree_path,
    unique_key_hashes_tree_path, unique_key_hashes_tree_path_vec, Drive,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use dpp::identity::Identity;
use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    /// Proves an identity id against a public key hash.
    pub fn prove_identity_id_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        self.grove_get_proved_path_query(&path_query, transaction, &mut vec![])
    }

    /// Proves identity ids against public key hashes.
    pub fn prove_identity_ids_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::identity_ids_by_unique_public_key_hash_query(public_key_hashes);
        self.grove_get_proved_path_query(&path_query, transaction, &mut vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use grovedb::GroveDb;

    mod prove_identity_ids {
        use super::*;
        use crate::drive::block_info::BlockInfo;
        use std::borrow::Borrow;

        #[test]
        fn should_prove_a_single_identity_id() {
            let drive = setup_drive_with_initial_state_structure();
            let identity = Identity::random_identity(3, Some(14));

            let identity_id = identity.id.to_buffer();
            drive
                .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                .expect("expected to add an identity");
            let first_key_hash = identity
                .public_keys
                .first_key_value()
                .expect("expected a key")
                .1
                .hash()
                .expect("expected to hash first_key")
                .try_into()
                .expect("expected to be 20 bytes");
            let proof = drive
                .prove_identity_id_by_unique_public_key_hash(first_key_hash, None)
                .expect("should not error when proving an identity");

            let (_, proved_identity_id) =
                Drive::verify_identity_id_by_public_key_hash(proof.as_slice(), first_key_hash)
                    .expect("expect that this be verified");

            assert_eq!(proved_identity_id, Some(identity_id));
        }

        #[test]
        fn should_prove_multiple_identity_ids() {
            let drive = setup_drive_with_initial_state_structure();

            let identities: BTreeMap<[u8; 32], Identity> =
                Identity::random_identities(10, 3, Some(14))
                    .into_iter()
                    .map(|identity| (identity.id.to_buffer(), identity))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                    .expect("expected to add an identity");
            }

            let key_hashes_to_identity_ids = identities
                .values()
                .into_iter()
                .map(|identity| {
                    (
                        identity
                            .public_keys
                            .first_key_value()
                            .expect("expected a key")
                            .1
                            .hash()
                            .expect("expected to hash first_key")
                            .try_into()
                            .expect("expected to be 20 bytes"),
                        Some(identity.id.to_buffer()),
                    )
                })
                .collect::<BTreeMap<[u8; 20], Option<[u8; 32]>>>();

            let key_hashes = key_hashes_to_identity_ids
                .keys()
                .copied()
                .collect::<Vec<[u8; 20]>>();

            let proof = drive
                .prove_identity_ids_by_unique_public_key_hashes(&key_hashes, None)
                .expect("should not error when proving an identity");

            let (_, proved_identity_id): ([u8; 32], BTreeMap<[u8; 20], Option<[u8; 32]>>) =
                Drive::verify_identity_ids_by_public_key_hashes(proof.as_slice(), &key_hashes)
                    .expect("expect that this be verified");

            assert_eq!(proved_identity_id, key_hashes_to_identity_ids);
        }
    }
}
