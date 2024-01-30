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

use drive::dpp::identity::{Identity, KeyType, Purpose, SecurityLevel, TimestampMillis};

use crate::platform_types::system_identity_public_keys::v0::SystemIdentityPublicKeysV0Getters;
use crate::platform_types::system_identity_public_keys::SystemIdentityPublicKeys;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::IdentityV0;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use drive::dpp::system_data_contracts::SystemDataContract;
use drive::drive::batch::{
    DataContractOperationType, DocumentOperationType, DriveOperation, IdentityOperationType,
};

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

        let cache = self.drive.cache.read().unwrap();

        let dpns_data_contract = cache.system_data_contracts.dpns.clone();

        let system_data_contract_types = BTreeMap::from_iter([
            (
                SystemDataContract::DPNS,
                (
                    cache.system_data_contracts.dpns.clone(),
                    system_identity_public_keys.dpns_contract_owner(),
                ),
            ),
            (
                SystemDataContract::Withdrawals,
                (
                    cache.system_data_contracts.withdrawals.clone(),
                    system_identity_public_keys.withdrawals_contract_owner(),
                ),
            ),
            // TODO: Do we still need feature flags to change consensus params like timeouts and so on?
            // (
            //     SystemDataContract::FeatureFlags,
            //     (
            //         load_system_data_contract(
            //             SystemDataContract::FeatureFlags,
            //             platform_version.protocol_version,
            //         )?,
            //         system_identity_public_keys.feature_flags_contract_owner(),
            //     ),
            // ),
            (
                SystemDataContract::Dashpay,
                (
                    cache.system_data_contracts.dashpay.clone(),
                    system_identity_public_keys.dashpay_contract_owner(),
                ),
            ),
            (
                SystemDataContract::MasternodeRewards,
                (
                    cache.system_data_contracts.masternode_reward_shares.clone(),
                    system_identity_public_keys.masternode_reward_shares_contract_owner(),
                ),
            ),
        ]);

        drop(cache);

        for (_, (data_contract, identity_public_keys_set)) in system_data_contract_types {
            let public_keys = [
                (
                    0,
                    IdentityPublicKeyV0 {
                        id: 0,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::MASTER,
                        contract_bounds: None,
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
                        contract_bounds: None,
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
            )?;

            self.register_system_identity_operations(identity, &mut operations);
        }

        self.register_dpns_top_level_domain_operations(&dpns_data_contract, &mut operations)?;

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
    ) -> Result<(), Error> {
        let serialization =
            data_contract.serialize_to_bytes_with_platform_version(platform_version)?;
        operations.push(DriveOperation::DataContractOperation(
            //todo: remove cbor
            DataContractOperationType::ApplyContractWithSerialization {
                contract: Cow::Owned(data_contract),
                serialized_contract: serialization,
                storage_flags: None,
            },
        ));
        Ok(())
    }

    fn register_system_identity_operations(
        &self,
        identity: Identity,
        operations: &mut Vec<DriveOperation>,
    ) {
        operations.push(DriveOperation::IdentityOperation(
            IdentityOperationType::AddNewIdentity {
                identity,
                is_masternode_identity: false,
            },
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
            created_at: None,
            updated_at: None,
        }
        .into();

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
        use crate::config::PlatformConfig;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use drive::drive::config::DriveConfig;

        #[test]
        pub fn should_create_genesis_state_deterministically() {
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
                .root_hash(None)
                .unwrap()
                .expect("should obtain root hash");

            assert_eq!(
                root_hash,
                [
                    148, 196, 101, 163, 1, 56, 137, 50, 117, 196, 107, 103, 222, 55, 110, 59, 11,
                    177, 172, 37, 17, 234, 56, 154, 232, 24, 208, 220, 27, 128, 201, 97
                ]
            )
        }
    }
}
