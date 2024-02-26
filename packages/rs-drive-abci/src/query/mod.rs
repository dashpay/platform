mod data_contract_based_queries;
mod document_query;
mod identity_based_queries;
mod proofs;
mod response_metadata;
mod system;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::error::execution::ExecutionError;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

#[cfg(test)]
mod tests {
    use crate::error::query::QueryError;
    use crate::platform_types::platform::Platform;
    use crate::query::QueryValidationResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;
    use drive::drive::batch::DataContractOperationType;
    use drive::drive::batch::DriveOperation::DataContractOperation;
    use platform_version::version::PlatformVersion;
    use std::borrow::Cow;

    pub fn setup_platform<'a>() -> (TempPlatform<MockCoreRPCLike>, &'a PlatformVersion) {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = platform
            .platform
            .state
            .read()
            .unwrap()
            .current_platform_version()
            .unwrap();

        (platform, platform_version)
    }

    pub fn store_data_contract(
        platform: &Platform<MockCoreRPCLike>,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) {
        let operation = DataContractOperation(DataContractOperationType::ApplyContract {
            contract: Cow::Owned(data_contract.to_owned()),
            storage_flags: None,
        });

        let block_info = BlockInfo::genesis();

        platform
            .drive
            .apply_drive_operations(vec![operation], true, &block_info, None, platform_version)
            .expect("expected to apply drive operations");
    }

    pub fn assert_invalid_identifier<TData: Clone>(
        validation_result: QueryValidationResult<TData>,
    ) {
        assert!(matches!(
            validation_result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("id must be a valid identifier (32 bytes long)")
        ));
    }
}
