use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::{Identity, IdentityV0};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches an identity with all its information and
    /// the cost it took from storage.
    pub(super) fn fetch_full_identity_with_costs_v0(
        &self,
        identity_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Identity>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let maybe_identity = self.fetch_full_identity_operations(
            identity_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((maybe_identity, fee))
    }

    // TODO: We deal with it in upcoming PR (Sam!!)
    // /// Fetches identities with all its information from storage.
    // #[deprecated(since = "0.24.0", note = "please use exact fetching")]
    // pub fn fetch_full_identities_efficient(
    //     &self,
    //     identity_ids: Vec<[u8; 32]>,
    //     transaction: TransactionArg,
    // ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
    //     let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
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

    /// Fetches an identity with all its information from storage.
    pub(super) fn fetch_full_identity_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identity_operations(
            identity_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(super) fn fetch_full_identity_operations_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        // let's start by getting the balance
        let balance = self.fetch_identity_balance_operations(
            identity_id,
            true,
            transaction,
            drive_operations,
            platform_version,
        )?;
        if balance.is_none() {
            return Ok(None);
        }
        let balance = balance.unwrap();
        let revision = self
            .fetch_identity_revision_operations(
                identity_id,
                true,
                transaction,
                drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "revision not found on identity".to_string(),
            )))?;

        let public_keys = self.fetch_all_identity_keys_operations(
            identity_id,
            transaction,
            drive_operations,
            platform_version,
        )?;

        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(Some(
                IdentityV0 {
                    id: Identifier::new(identity_id),
                    public_keys,
                    balance,
                    revision,
                }
                .into(),
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identity_operations (for identity structure)".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
