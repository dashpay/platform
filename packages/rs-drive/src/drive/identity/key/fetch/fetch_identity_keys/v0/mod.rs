use crate::drive::identity::key::fetch::KeyRequestType::{
    AllKeys, ContractBoundKey, ContractDocumentTypeBoundKey, SearchKey, SpecificKeys,
};
use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, IdentityPublicKeyResult, KeyKindRequestType,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::TransactionArg;

impl Drive {
    /// Fetch keys matching the request for a specific Identity
    pub(super) fn fetch_identity_keys_v0<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<T, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_keys_operations(
            key_request,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Operations for fetching keys matching the request for a specific Identity
    pub(super) fn fetch_identity_keys_operations_v0<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<T, Error> {
        match &key_request.request_type {
            AllKeys => {
                let path_query = key_request.into_path_query();

                let (result, _) = self.grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryPathKeyElementTrioResultType,
                    drive_operations,
                    &platform_version.drive,
                )?;

                T::try_from_query_results(result, platform_version)
            }
            SpecificKeys(_) => {
                let path_query = key_request.into_path_query();

                let result = self.grove_get_raw_path_query_with_optional(
                    &path_query,
                    false,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;

                T::try_from_path_key_optional(result, platform_version)
            }
            SearchKey(_)
            | ContractBoundKey(_, _, KeyKindRequestType::CurrentKeyOfKindRequest)
            | ContractDocumentTypeBoundKey(_, _, _, KeyKindRequestType::CurrentKeyOfKindRequest) => {
                let path_query = key_request.into_path_query();

                let result = self.grove_get_path_query_with_optional(
                    &path_query,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;

                T::try_from_path_key_optional(result, platform_version)
            }
            ContractBoundKey(_, _, KeyKindRequestType::AllKeysOfKindRequest)
            | ContractDocumentTypeBoundKey(_, _, _, KeyKindRequestType::AllKeysOfKindRequest) => {
                let path_query = key_request.into_path_query();

                let (result, _) = self.grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryPathKeyElementTrioResultType,
                    drive_operations,
                    &platform_version.drive,
                )?;

                T::try_from_query_results(result, platform_version)
            }
        }
    }
}
