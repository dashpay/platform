use crate::drive::balances::balance_path;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;
use dpp::identifier::Identifier;
use dpp::identity::Identity;
use dpp::prelude::Revision;
use grovedb::query_result_type::Path;
use grovedb::{PathQuery, TransactionArg};
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// Fetches an identity with all its information and
    /// the cost it took from storage.
    pub fn fetch_full_identity_with_costs(
        &self,
        identity_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
    ) -> Result<(Option<Identity>, FeeResult), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let maybe_identity =
            self.fetch_full_identity_operations(identity_id, transaction, &mut drive_operations)?;
        let fee = calculate_fee(None, Some(drive_operations), epoch)?;
        Ok((maybe_identity, fee))
    }

    // /// Fetches identities with all its information from storage.
    // #[deprecated(since = "0.24.0", note = "please use exact fetching")]
    // pub fn fetch_full_identities_efficient(
    //     &self,
    //     identity_ids: Vec<[u8; 32]>,
    //     transaction: TransactionArg,
    // ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
    //     let mut drive_operations: Vec<DriveOperation> = vec![];
    //     let query = Self::full_identities_query(identity_ids)?;
    //     let result =
    //         self.grove_get_path_query_with_optional(&query, transaction, &mut drive_operations)?;
    //
    //     let balances_path = balance_path();
    //     // Let's do a first pass to get identities from balances
    //     let mut identities :BTreeMap<[u8; 32], Option<Identity>> = result.iter().filter_map(
    //         |(path, key, element)| {
    //             if path == balances_path {
    //                 let identity_id = key.try_into().map_err(|_| Error::Drive(DriveError::CorruptedDriveState("balance key not 32 bytes".to_string())))?;
    //                 if &Some(balance) = element {
    //                     Some((identity_id, Identity {
    //                         protocol_version: PROTOCOL_VERSION,
    //                         id: identity_id,
    //                         public_keys: Default::default(),
    //                         balance,
    //                         revision: Revision::MAX,
    //                         asset_lock_proof: None,
    //                         metadata: None,
    //                     }))
    //                 } else {
    //                     Some((identity_id, None))
    //                 }
    //             } else {
    //                 None
    //             }
    //         }
    //     ).collect()?;
    //
    //     result.into_iter().try_for_each(
    //         |(path, key, element)| {
    //             if path != balances_path {
    //                 if let Some(element) = element {
    //                     // we need to get the identity_id from the path which will be the second item (1)
    //                     // of the path
    //                     let identity_id: [u8;32] = path.get(1).ok_or(Error::Drive(DriveError::CorruptedDriveState("path much contain identity id".to_string())))?.try_into()
    //                         .try_into().map_err(|_| Error::Drive(DriveError::CorruptedDriveState("identity id not 32 bytes".to_string())))?;
    //                     let mut identity = identities.get_mut(&identity_id).ok_or();
    //                 }
    //             }
    //         }
    //     )?;
    //     Ok(identities)
    // }

    /// Fetches identities with all its information from storage.
    #[deprecated(since = "0.24.0", note = "please use exact fetching")]
    pub fn fetch_full_identities(
        &self,
        identity_ids: Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
        identity_ids
            .into_iter()
            .map(|identity_id| {
                Ok((
                    identity_id,
                    self.fetch_full_identity(identity_id, transaction)?,
                ))
            })
            .collect()
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_full_identity_operations(identity_id, transaction, &mut drive_operations)
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_full_identity_operations(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Identity>, Error> {
        // let's start by getting the balance
        let balance = self.fetch_identity_balance_operations(
            identity_id,
            true,
            transaction,
            drive_operations,
        )?;
        if balance.is_none() {
            return Ok(None);
        }
        let balance = balance.unwrap();
        let revision = self
            .fetch_identity_revision_operations(identity_id, true, transaction, drive_operations)?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "revision not found on identity".to_string(),
            )))?;

        let public_keys =
            self.fetch_all_identity_keys_operations(identity_id, transaction, drive_operations)?;
        Ok(Some(Identity {
            protocol_version: PROTOCOL_VERSION,
            id: Identifier::new(identity_id),
            public_keys,
            balance,
            revision,
            asset_lock_proof: None,
            metadata: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use grovedb::GroveDb;

    mod fetch_full_identities {
        use super::*;
        use crate::drive::block_info::BlockInfo;

        #[test]
        fn should_get_full_identities() {
            let drive = setup_drive_with_initial_state_structure();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(10, 3, Some(14))
                    .into_iter()
                    .map(|identity| (identity.id.to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        &BlockInfo::default(),
                        true,
                        None,
                    )
                    .expect("expected to add an identity");
            }
            let fetched_identities = drive
                .fetch_full_identities(identities.keys().copied().collect(), None)
                .expect("should get identities");

            assert_eq!(identities, fetched_identities);
        }
    }

    mod fetch_full_identity {
        use super::*;
        use crate::drive::block_info::BlockInfo;

        #[test]
        fn should_return_none_if_identity_is_not_present() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = drive
                .fetch_full_identity(
                    [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0,
                    ],
                    None,
                )
                .expect("should return none");

            assert!(identity.is_none());
        }

        #[test]
        fn should_get_a_full_identity() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(3, Some(14));

            let identity_id = identity.id.to_buffer();
            drive
                .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                .expect("expected to add an identity");
            let fetched_identity = drive
                .fetch_full_identity(identity_id, None)
                .expect("should not error when fetching an identity")
                .expect("should find an identity");

            assert_eq!(identity, fetched_identity);
        }
    }
}
