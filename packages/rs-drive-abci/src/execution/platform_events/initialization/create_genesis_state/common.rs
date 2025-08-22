use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::platform_value::platform_value;
use dpp::ProtocolError;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::system_data_contracts::dpns_contract::DPNS_DASH_TLD_DOCUMENT_ID;
use dpp::version::PlatformVersion;
use drive::dpp::identity::TimestampMillis;
use drive::util::batch::{DataContractOperationType, DocumentOperationType, DriveOperation};
use drive::util::object_size_info::{
    DataContractInfo, DocumentInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use std::borrow::Cow;

impl<C> Platform<C> {
    pub(in crate::execution::platform_events::initialization::create_genesis_state) fn register_system_data_contract_operations<
        'a,
    >(
        &self,
        data_contract: &'a DataContract,
        operations: &mut Vec<DriveOperation<'a>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let serialization =
            data_contract.serialize_to_bytes_with_platform_version(platform_version)?;
        operations.push(DriveOperation::DataContractOperation(
            DataContractOperationType::ApplyContractWithSerialization {
                contract: Cow::Borrowed(data_contract),
                serialized_contract: serialization,
                storage_flags: None,
            },
        ));
        Ok(())
    }

    pub(in crate::execution::platform_events::initialization::create_genesis_state) fn register_dpns_top_level_domain_operations<
        'a,
    >(
        &'a self,
        contract: &'a DataContract,
        genesis_time: TimestampMillis,
        operations: &mut Vec<DriveOperation<'a>>,
    ) -> Result<(), Error> {
        let domain = "dash";

        let document_stub_properties_value = platform_value!({
            "label" : domain,
            "normalizedLabel" : domain,
            "parentDomainName" : "",
            "normalizedParentDomainName" : "",
            "records" : {
                "identity" : contract.owner_id(),
            },
            "subdomainRules": {
                "allowSubdomains": true,
            }
        });

        let document_stub_properties = document_stub_properties_value
            .into_btree_string_map()
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

        let document = DocumentV0 {
            id: DPNS_DASH_TLD_DOCUMENT_ID.into(),
            properties: document_stub_properties,
            owner_id: contract.owner_id(),
            revision: None,
            created_at: Some(genesis_time),
            updated_at: Some(genesis_time),
            transferred_at: Some(genesis_time),
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        }
        .into();

        let document_type = contract.document_type_for_name("domain")?;

        let operation = DriveOperation::DocumentOperation(DocumentOperationType::AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentInfo::DocumentOwnedInfo((document, None)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(contract),
            document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
            override_document: false,
        });

        operations.push(operation);

        Ok(())
    }
}
