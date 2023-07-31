// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::platform_value::{platform_value, BinaryData};
use dpp::ProtocolError;
use drive::dpp::document::Document;
use drive::dpp::identity::{
    Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel, TimestampMillis,
};

use crate::platform_types::system_identity_public_keys::v0::SystemIdentityPublicKeysV0Getters;
use crate::platform_types::system_identity_public_keys::SystemIdentityPublicKeys;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::base::DataContractBaseMethodsV0;
use dpp::data_contract::DataContract;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::IdentityV0;
use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use dpp::version::PlatformVersion;
use drive::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use drive::drive::batch::{
    DataContractOperationType, DocumentOperationType, DriveOperation, IdentityOperationType,
};
use drive::drive::defaults::PROTOCOL_VERSION;
use drive::drive::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use drive::query::TransactionArg;
use std::borrow::Cow;
use std::collections::BTreeMap;

const DPNS_DASH_TLD_DOCUMENT_ID: [u8; 32] = [
    215, 242, 197, 63, 70, 169, 23, 171, 110, 91, 57, 162, 215, 188, 38, 11, 100, 146, 137, 69, 55,
    68, 209, 224, 212, 242, 106, 141, 142, 255, 55, 207,
];
const DPNS_DASH_TLD_PREORDER_SALT: [u8; 32] = [
    224, 181, 8, 197, 163, 104, 37, 162, 6, 105, 58, 31, 65, 74, 161, 62, 219, 236, 244, 60, 65,
    227, 199, 153, 234, 158, 115, 123, 79, 154, 162, 38,
];

impl<C> Platform<C> {
    /// Creates trees and populates them with necessary identities, contracts and documents
    pub fn create_genesis_state_v0(
        &self,
        genesis_time: TimestampMillis,
        system_identity_public_keys: SystemIdentityPublicKeys,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        //versioned call
        self.drive
            .create_initial_state_structure(transaction, platform_version)
            .map_err(Error::Drive)?;

        let mut operations = vec![];

        // Create system identities and contracts

        let dpns_contract =
            load_system_data_contract(SystemDataContract::DPNS, platform_version.protocol_version)?;

        let system_data_contract_types = BTreeMap::from_iter([
            (
                SystemDataContract::DPNS,
                (
                    dpns_contract.clone(),
                    system_identity_public_keys.dpns_contract_owner(),
                ),
            ),
            (
                SystemDataContract::Withdrawals,
                (
                    load_system_data_contract(
                        SystemDataContract::Withdrawals,
                        platform_version.protocol_version,
                    )?,
                    system_identity_public_keys.withdrawals_contract_owner(),
                ),
            ),
            (
                SystemDataContract::FeatureFlags,
                (
                    load_system_data_contract(
                        SystemDataContract::FeatureFlags,
                        platform_version.protocol_version,
                    )?,
                    system_identity_public_keys.feature_flags_contract_owner(),
                ),
            ),
            (
                SystemDataContract::Dashpay,
                (
                    load_system_data_contract(
                        SystemDataContract::Dashpay,
                        platform_version.protocol_version,
                    )?,
                    system_identity_public_keys.dashpay_contract_owner(),
                ),
            ),
            (
                SystemDataContract::MasternodeRewards,
                (
                    load_system_data_contract(
                        SystemDataContract::MasternodeRewards,
                        platform_version.protocol_version,
                    )?,
                    system_identity_public_keys.masternode_reward_shares_contract_owner(),
                ),
            ),
        ]);

        for (_, (data_contract, identity_public_keys_set)) in system_data_contract_types {
            let public_keys = [
                (
                    0,
                    IdentityPublicKeyV0 {
                        id: 0,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::MASTER,
                        key_type: KeyType::ECDSA_SECP256K1,
                        read_only: false,
                        data: identity_public_keys_set.master.clone().into(),
                        disabled_at: None,
                    }
                    .into(),
                ),
                (
                    1,
                    IdentityPublicKeyV0 {
                        id: 1,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::HIGH,
                        key_type: KeyType::ECDSA_SECP256K1,
                        read_only: false,
                        data: identity_public_keys_set.high.clone().into(),
                        disabled_at: None,
                    }
                    .into(),
                ),
            ];

            let identity = IdentityV0 {
                id: data_contract.owner_id(),
                public_keys: BTreeMap::from(public_keys),
                balance: 0,
                revision: 0,
            }
            .into();

            self.register_system_data_contract_operations(
                data_contract,
                &mut operations,
                platform_version,
            );

            self.register_system_identity_operations(identity, &mut operations);
        }

        self.register_dpns_top_level_domain_operations(&dpns_contract, &mut operations)?;

        let block_info = BlockInfo::default_with_time(genesis_time);

        self.drive.apply_drive_operations(
            operations,
            true,
            &block_info,
            transaction,
            platform_version,
        )?;

        Ok(())
    }

    fn register_system_data_contract_operations(
        &self,
        data_contract: DataContract,
        operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) {
        let serialization = data_contract.serialize_with_platform_version().unwrap();
        operations.push(DriveOperation::DataContractOperation(
            //todo: remove cbor
            ContractOperationType::ApplyContractWithSerialization {
                contract: Cow::Owned(data_contract),
                serialized_contract: serialization,
                storage_flags: None,
            },
        ))
    }

    fn register_system_identity_operations(
        &self,
        identity: Identity,
        operations: &mut Vec<DriveOperation>,
    ) {
        operations.push(DriveOperation::IdentityOperation(
            IdentityOperationType::AddNewIdentity { identity },
        ))
    }

    fn register_dpns_top_level_domain_operations<'a>(
        &'a self,
        contract: &'a DataContract,
        operations: &mut Vec<DriveOperation<'a>>,
    ) -> Result<(), Error> {
        let domain = "dash";

        let document_stub_properties_value = platform_value!({
            "label" : domain,
            "normalizedLabel" : domain,
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

        let document = Document {
            id: DPNS_DASH_TLD_DOCUMENT_ID.into(),
            properties: document_stub_properties,
            owner_id: contract.owner_id(),
            revision: None,
            created_at: None,
            updated_at: None,
        };

        let document_type = contract.document_type_for_name("domain")?;

        let operation =
            DriveOperation::DocumentOperation(DocumentOperationType::AddDocumentForContract {
                document_and_contract_info: DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentOwnedInfo((document, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                override_document: false,
            });

        operations.push(operation);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod create_genesis_state {
        use crate::test::helpers::setup::TestPlatformBuilder;

        #[test]
        pub fn should_create_genesis_state_deterministically() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let root_hash = platform
                .drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should obtain root hash");

            assert_eq!(
                root_hash,
                [
                    160, 90, 213, 23, 20, 159, 230, 205, 37, 87, 220, 50, 254, 118, 219, 31, 151,
                    34, 100, 176, 59, 217, 113, 231, 25, 48, 128, 28, 200, 22, 113, 88
                ]
            )
        }
    }
}
