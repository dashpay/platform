use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::platform_value::{platform_value, BinaryData};
use dpp::ProtocolError;

use drive::dpp::identity::TimestampMillis;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use drive::dpp::system_data_contracts::SystemDataContract;
use drive::util::batch::{DataContractOperationType, DocumentOperationType, DriveOperation};

use dpp::prelude::CoreBlockHeight;
use dpp::system_data_contracts::dpns_contract::{
    DPNS_DASH_TLD_DOCUMENT_ID, DPNS_DASH_TLD_PREORDER_SALT,
};
use drive::query::TransactionArg;
use drive::util::object_size_info::{
    DataContractInfo, DocumentInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use std::borrow::Cow;
use std::collections::BTreeMap;

impl<C> Platform<C> {
    /// Creates trees and populates them with necessary identities, contracts and documents
    #[inline(always)]
    pub(super) fn create_genesis_state_v0(
        &self,
        genesis_core_height: CoreBlockHeight,
        genesis_time: TimestampMillis,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        //versioned call
        self.drive
            .create_initial_state_structure(transaction, platform_version)?;

        self.drive
            .store_genesis_core_height(genesis_core_height, transaction, platform_version)?;

        let mut operations = vec![];

        // Create system identities and contracts

        let system_data_contracts = &self.drive.cache.system_data_contracts;

        let system_data_contract_types = BTreeMap::from_iter([
            (SystemDataContract::DPNS, system_data_contracts.load_dpns()),
            (
                SystemDataContract::Withdrawals,
                system_data_contracts.load_withdrawals(),
            ),
            (
                SystemDataContract::Dashpay,
                system_data_contracts.load_dashpay(),
            ),
            (
                SystemDataContract::MasternodeRewards,
                system_data_contracts.load_masternode_reward_shares(),
            ),
        ]);

        for data_contract in system_data_contract_types.values() {
            self.register_system_data_contract_operations(
                data_contract,
                &mut operations,
                platform_version,
            )?;
        }

        let dpns_contract = system_data_contracts.load_dpns();

        self.register_dpns_top_level_domain_operations(
            &dpns_contract,
            genesis_time,
            &mut operations,
        )?;

        let block_info = BlockInfo::default_with_time(genesis_time);

        self.drive.apply_drive_operations(
            operations,
            true,
            &block_info,
            transaction,
            platform_version,
            None, // No previous_fee_versions needed for genesis state creation
        )?;

        Ok(())
    }

    fn register_system_data_contract_operations<'a>(
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

    fn register_dpns_top_level_domain_operations<'a>(
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
            "preorderSalt" : BinaryData::new(DPNS_DASH_TLD_PREORDER_SALT.to_vec()),
            "records" : {
                "dashAliasIdentityId" : contract.owner_id(),
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

#[cfg(test)]
mod tests {
    mod create_genesis_state {
        use crate::config::PlatformConfig;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use drive::config::DriveConfig;
        use platform_version::version::PlatformVersion;

        #[test]
        pub fn should_create_genesis_state_deterministically() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .with_config(PlatformConfig {
                    drive: DriveConfig {
                        epochs_per_era: 20,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .build_with_mock_rpc()
                .set_genesis_state();

            let root_hash = platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should obtain root hash");

            assert_eq!(
                hex::encode(root_hash),
                "8163884a9eef0d3b306bd6f426806e7ff41d7b09f030c4ff2b79b3b4c646dfca"
            )
        }
    }
}
