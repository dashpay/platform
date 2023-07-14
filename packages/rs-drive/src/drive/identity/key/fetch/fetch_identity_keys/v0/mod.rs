use crate::drive::identity::key::fetch::KeyRequestType::{SearchKey, SpecificKeys};
use crate::drive::identity::key::fetch::{IdentityKeysRequest, IdentityPublicKeyResult};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::TransactionArg;

impl Drive {
    /// Fetch keys matching the request for a specific Identity
    pub(super) fn fetch_identity_keys_v0<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<T, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_keys_operations(
            key_request,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }

    /// Operations for fetching keys matching the request for a specific Identity
    pub(super) fn fetch_identity_keys_operations_v0<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<T, Error> {
        match &key_request.request_type {
            AllKeys => {
                let path_query = key_request.into_path_query();

                let (result, _) = self.grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryPathKeyElementTrioResultType,
                    drive_operations,
                    drive_version,
                )?;

                T::try_from_query_results(result)
            }
            SpecificKeys(_) => {
                let path_query = key_request.into_path_query();

                let result = self.grove_get_raw_path_query_with_optional(
                    &path_query,
                    transaction,
                    drive_operations,
                    drive_version,
                )?;

                T::try_from_path_key_optional(result)
            }
            SearchKey(_) => {
                let path_query = key_request.into_path_query();

                let result = self.grove_get_path_query_with_optional(
                    &path_query,
                    transaction,
                    drive_operations,
                    drive_version,
                )?;

                T::try_from_path_key_optional(result)
            }
        }
    }
}
