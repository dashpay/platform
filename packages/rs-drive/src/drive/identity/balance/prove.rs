use crate::drive::Drive;
use crate::error::Error;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Proves an Identity's balance from the backing store
    pub fn prove_identity_balance(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let balance_query = Self::balance_for_identity_id_query(identity_id);
        self.grove_get_proved_path_query(&balance_query, transaction, &mut vec![], drive_version)
    }

    /// Proves an Identity's balance and revision from the backing store
    pub fn prove_identity_balance_and_revision(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let balance_query = Self::balance_and_revision_for_identity_id_query(
            identity_id,
            &drive_version.grove_version,
        );
        self.grove_get_proved_path_query(&balance_query, transaction, &mut vec![], drive_version)
    }

    /// Proves multiple Identity balances from the backing store
    pub fn prove_many_identity_balances(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let balance_query =
            Self::balances_for_identity_ids_query(identity_ids, &drive_version.grove_version)?;
        self.grove_get_proved_path_query(&balance_query, transaction, &mut vec![], drive_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::Identity;

    mod prove_identity_balance {
        use super::*;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_prove_a_single_identity_balance() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::first();

            let identity = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a platform identity");

            let identity_id = identity.id().to_buffer();
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
            let proof = drive
                .prove_identity_balance(identity.id().to_buffer(), None, &platform_version.drive)
                .expect("should not error when proving an identity");

            let (_, proved_identity_balance) = Drive::verify_identity_balance_for_identity_id(
                proof.as_slice(),
                identity_id,
                false,
                platform_version,
            )
            .expect("expect that this be verified");

            assert_eq!(proved_identity_balance, Some(identity.balance()));
        }
    }

    mod prove_many_identity_balances {
        use super::*;
        use dpp::fee::Credits;
        use dpp::identity::accessors::IdentityGettersV0;
        use platform_version::version::PlatformVersion;
        use std::collections::BTreeMap;

        #[test]
        fn should_prove_multiple_identity_balances() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();
            let identities: BTreeMap<[u8; 32], Identity> =
                Identity::random_identities(10, 3, Some(14), platform_version)
                    .expect("expected to get random identities")
                    .into_iter()
                    .map(|identity| (identity.id().to_buffer(), identity))
                    .collect();

            for identity in identities.values() {
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
            let identity_ids = identities.keys().copied().collect::<Vec<[u8; 32]>>();
            let identity_balances = identities
                .into_iter()
                .map(|(id, identity)| (id, Some(identity.balance())))
                .collect::<BTreeMap<[u8; 32], Option<Credits>>>();
            let proof = drive
                .prove_many_identity_balances(
                    identity_ids.as_slice(),
                    None,
                    &platform_version.drive,
                )
                .expect("should not error when proving an identity");

            let (_, proved_identity_balances): ([u8; 32], BTreeMap<[u8; 32], Option<Credits>>) =
                Drive::verify_identity_balances_for_identity_ids(
                    proof.as_slice(),
                    false,
                    identity_ids.as_slice(),
                    platform_version,
                )
                .expect("expect that this be verified");

            assert_eq!(proved_identity_balances, identity_balances);
        }
    }
}
