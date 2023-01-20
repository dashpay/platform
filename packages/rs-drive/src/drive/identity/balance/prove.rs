use crate::drive::balances::balance_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::credits::Credits;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an Identity's balance from the backing store
    pub fn prove_identity_balance(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let balance_query = Self::balance_for_identity_id_query(identity_id);
        self.grove_get_proved_path_query(&balance_query, transaction, &mut vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::drive::block_info::BlockInfo;
    use dpp::identity::Identity;
    use grovedb::GroveDb;
    use std::borrow::Borrow;

    mod prove_identity_ids {
        use super::*;
        use std::collections::BTreeMap;

        #[test]
        fn should_prove_a_single_identity_balance() {
            let drive = setup_drive_with_initial_state_structure();
            let identity = Identity::random_identity(3, Some(14));

            let identity_id = identity.id.to_buffer();
            drive
                .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                .expect("expected to add an identity");
            let proof = drive
                .prove_identity_balance(identity.id.to_buffer(), None)
                .expect("should not error when proving an identity");

            let (_, proved_identity_balance) =
                Drive::verify_identity_balance_for_identity_id(proof.as_slice(), identity_id)
                    .expect("expect that this be verified");

            assert_eq!(proved_identity_balance, Some(identity.balance));
        }

        // #[test]
        // fn should_prove_multiple_identity_balances() {
        //     let drive = setup_drive_with_initial_state_structure();
        //     let identities: BTreeMap<[u8; 32], Identity> =
        //         Identity::random_identities(10, 3, Some(14))
        //             .into_iter()
        //             .map(|identity| (identity.id.to_buffer(), identity))
        //             .collect();
        //
        //     for identity in identities.values() {
        //         drive
        //             .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
        //             .expect("expected to add an identity");
        //     }
        //
        //     let key_hashes_to_identity_ids = identities
        //         .values()
        //         .into_iter()
        //         .map(|identity| {
        //             (
        //                 identity
        //                     .public_keys
        //                     .first_key_value()
        //                     .expect("expected a key")
        //                     .1
        //                     .hash()
        //                     .expect("expected to hash first_key")
        //                     .try_into()
        //                     .expect("expected to be 20 bytes"),
        //                 Some(identity.id.to_buffer()),
        //             )
        //         })
        //         .collect::<BTreeMap<[u8; 20], Option<[u8; 32]>>>();
        //     let proof = drive
        //         .prove_identity_balance(identity.id.to_buffer(), None)
        //         .expect("should not error when proving an identity");
        //
        //     let (_, proved_identity_balance) =
        //         Drive::verify_identity_balance_for_identity_id(proof.as_slice(), identity_id)
        //             .expect("expect that this be verified");
        //
        //     assert_eq!(proved_identity_balance, Some(identity.balance));
        // }

        // #[test]
        // fn should_prove_multiple_identity_ids() {
        //     let drive = setup_drive_with_initial_state_structure();
        //
        //     let identities: BTreeMap<[u8; 32], Identity> =
        //         Identity::random_identities(10, 3, Some(14))
        //             .into_iter()
        //             .map(|identity| (identity.id.to_buffer(), identity))
        //             .collect();
        //
        //     for identity in identities.values() {
        //         drive
        //             .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
        //             .expect("expected to add an identity");
        //     }
        //
        //     let key_hashes_to_identity_ids = identities
        //         .values()
        //         .into_iter()
        //         .map(|identity| {
        //             (
        //                 identity
        //                     .public_keys
        //                     .first_key_value()
        //                     .expect("expected a key")
        //                     .1
        //                     .hash()
        //                     .expect("expected to hash first_key")
        //                     .try_into()
        //                     .expect("expected to be 20 bytes"),
        //                 Some(identity.id.to_buffer()),
        //             )
        //         })
        //         .collect::<BTreeMap<[u8; 20], Option<[u8; 32]>>>();
        //
        //     let key_hashes = key_hashes_to_identity_ids
        //         .keys()
        //         .copied()
        //         .collect::<Vec<[u8; 20]>>();
        //
        //     let proof = drive
        //         .prove_identity_ids_by_unique_public_key_hashes(&key_hashes, None)
        //         .expect("should not error when proving an identity");
        //
        //     let (_, proved_identity_id): ([u8; 32], BTreeMap<[u8; 20], Option<[u8; 32]>>) =
        //         Drive::verify_identity_ids_by_public_key_hashes(proof.as_slice(), &key_hashes)
        //             .expect("expect that this be verified");
        //
        //     assert_eq!(proved_identity_id, key_hashes_to_identity_ids);
        // }
    }
}
