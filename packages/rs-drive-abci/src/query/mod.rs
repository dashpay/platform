mod data_contract_based_queries;
mod document_query;
mod group_queries;
mod identity_based_queries;
mod prefunded_specialized_balances;
mod proofs;
mod response_metadata;
mod service;
mod system;
mod token_queries;
mod validator_queries;
mod voting;

use crate::error::query::QueryError;

use dpp::validation::ValidationResult;

pub use service::QueryService;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

#[cfg(test)]
pub(crate) mod tests {
    use crate::error::query::QueryError;
    use crate::platform_types::platform::Platform;
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::platform_types::platform_state::PlatformState;
    use crate::query::QueryValidationResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;

    use crate::config::PlatformConfig;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::document::Document;
    use dpp::prelude::{CoreBlockHeight, TimestampMillis};
    use drive::util::batch::DriveOperation::{DataContractOperation, DocumentOperation};
    use drive::util::batch::{DataContractOperationType, DocumentOperationType};
    use drive::util::object_size_info::{
        DataContractInfo, DocumentInfo, DocumentTypeInfo, OwnedDocumentInfo,
    };
    use drive::util::storage_flags::StorageFlags;
    use platform_version::version::{PlatformVersion, ProtocolVersion};
    use std::borrow::Cow;
    use std::sync::Arc;

    pub fn setup_platform<'a>(
        with_genesis_state: Option<(TimestampMillis, CoreBlockHeight)>,
        network: Network,
        initial_protocol_version: Option<ProtocolVersion>,
    ) -> (
        TempPlatform<MockCoreRPCLike>,
        Arc<PlatformState>,
        &'a PlatformVersion,
    ) {
        let platform = if let Some((timestamp, activation_core_block_height)) = with_genesis_state {
            let mut platform_builder = TestPlatformBuilder::new()
                .with_config(PlatformConfig::default_for_network(network));

            if let Some(initial_protocol_version) = initial_protocol_version {
                platform_builder =
                    platform_builder.with_initial_protocol_version(initial_protocol_version);
            }

            platform_builder
                .build_with_mock_rpc()
                .set_genesis_state_with_activation_info(timestamp, activation_core_block_height)
        } else {
            let mut platform_builder = TestPlatformBuilder::new()
                .with_config(PlatformConfig::default_for_network(network));

            if let Some(initial_protocol_version) = initial_protocol_version {
                platform_builder =
                    platform_builder.with_initial_protocol_version(initial_protocol_version);
            }

            platform_builder
                .build_with_mock_rpc()
                .set_initial_state_structure()
        };

        // We can't return a reference to Arc (`load` method) so we clone Arc (`load_full`).
        // This is a bit slower, but we don't care since we are in test environment
        let platform_state = platform.platform.state.load_full();

        let platform_version = platform_state.current_platform_version().unwrap();

        (platform, platform_state, platform_version)
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
            .apply_drive_operations(
                vec![operation],
                true,
                &block_info,
                None,
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    pub fn store_document(
        platform: &Platform<MockCoreRPCLike>,
        data_contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        platform_version: &PlatformVersion,
    ) {
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let operation = DocumentOperation(DocumentOperationType::AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentInfo::DocumentRefInfo((document, storage_flags)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(data_contract),
            document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
            override_document: false,
        });

        let block_info = BlockInfo::genesis();

        platform
            .drive
            .apply_drive_operations(
                vec![operation],
                true,
                &block_info,
                None,
                platform_version,
                None,
            )
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
