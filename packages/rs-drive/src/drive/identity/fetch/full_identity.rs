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

    /// The query getting all keys and balance and revision
    pub fn full_identity_query(identity_id: [u8; 32]) -> Result<PathQuery, Error> {
        let balance_query = Self::identity_balance_query(identity_id);
        let revision_query = Self::identity_revision_query(identity_id);
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id);
        let all_keys_query = key_request.into_path_query();
        PathQuery::merge(vec![&balance_query, &revision_query, &all_keys_query])
            .map_err(Error::GroveDB)
    }

    /// The query getting all keys and balance and revision
    pub fn full_identities_query(identity_ids: Vec<[u8; 32]>) -> Result<PathQuery, Error> {
        let path_queries: Vec<PathQuery> = identity_ids
            .into_iter()
            .map(|identity_id| Self::full_identity_query(identity_id))
            .collect::<Result<Vec<PathQuery>, Error>>()?;
        PathQuery::merge(path_queries.iter().map(|query| query).collect()).map_err(Error::GroveDB)
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_proved_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identity_query(identity_id)?;
        let result = self.grove_get_proved_path_query(&query, transaction, &mut drive_operations);
        match result {
            Ok(r) => Ok(Some(r)),
            Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathNotFound(_))) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_proved_full_identities(
        &self,
        identity_ids: Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identities_query(identity_ids)?;
        let result = self.grove_get_proved_path_query(&query, transaction, &mut drive_operations);
        match result {
            Ok(r) => Ok(Some(r)),
            Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathNotFound(_))) => Ok(None),
            Err(e) => Err(e),
        }
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
