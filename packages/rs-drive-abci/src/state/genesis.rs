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

use crate::abci::messages::SystemIdentityPublicKeys;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use ciborium::{cbor, Value as CborValue};
use dpp::platform_value::Value;
use dpp::ProtocolError;
use drive::contract::DataContract;
use drive::dpp::data_contract::DriveContractExt;
use drive::dpp::document::Document;
use drive::dpp::document::ExtendedDocument;
use drive::dpp::identity::{
    Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel, TimestampMillis,
};
use drive::dpp::prelude::Identifier;
use drive::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use drive::dpp::util::string_encoding::{encode, Encoding};
use drive::drive::batch::{
    ContractOperationType, DocumentOperationType, DriveOperationType, IdentityOperationType,
};
use drive::drive::block_info::BlockInfo;
use drive::drive::defaults::PROTOCOL_VERSION;
use drive::drive::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use drive::query::TransactionArg;
use serde_json::json;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

const DPNS_DASH_TLD_DOCUMENT_ID: [u8; 32] = [
    215, 242, 197, 63, 70, 169, 23, 171, 110, 91, 57, 162, 215, 188, 38, 11, 100, 146, 137, 69, 55,
    68, 209, 224, 212, 242, 106, 141, 142, 255, 55, 207,
];
const DPNS_DASH_TLD_PREORDER_SALT: [u8; 32] = [
    224, 181, 8, 197, 163, 104, 37, 162, 6, 105, 58, 31, 65, 74, 161, 62, 219, 236, 244, 60, 65,
    227, 199, 153, 234, 158, 115, 123, 79, 154, 162, 38,
];

impl Platform {
    /// Creates trees and populates them with necessary identities, contracts and documents
    pub fn create_genesis_state(
        &self,
        genesis_time: TimestampMillis,
        system_identity_public_keys: SystemIdentityPublicKeys,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.drive
            .create_initial_state_structure(transaction)
            .map_err(Error::Drive)?;

        let mut operations = vec![];

        // Create system identities and contracts

        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS)?;

        let system_data_contract_types = BTreeMap::from_iter([
            (
                SystemDataContract::DPNS,
                (
                    dpns_contract.clone(),
                    system_identity_public_keys.dpns_contract_owner,
                ),
            ),
            (
                SystemDataContract::Withdrawals,
                (
                    load_system_data_contract(SystemDataContract::Withdrawals)?,
                    system_identity_public_keys.withdrawals_contract_owner,
                ),
            ),
            (
                SystemDataContract::FeatureFlags,
                (
                    load_system_data_contract(SystemDataContract::FeatureFlags)?,
                    system_identity_public_keys.feature_flags_contract_owner,
                ),
            ),
            (
                SystemDataContract::Dashpay,
                (
                    load_system_data_contract(SystemDataContract::Dashpay)?,
                    system_identity_public_keys.dashpay_contract_owner,
                ),
            ),
            (
                SystemDataContract::MasternodeRewards,
                (
                    load_system_data_contract(SystemDataContract::MasternodeRewards)?,
                    system_identity_public_keys.masternode_reward_shares_contract_owner,
                ),
            ),
        ]);

        for (_, (data_contract, identity_public_keys_set)) in system_data_contract_types {
            let public_keys = BTreeSet::from_iter([
                IdentityPublicKey {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: identity_public_keys_set.master,
                    disabled_at: None,
                },
                IdentityPublicKey {
                    id: 1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: identity_public_keys_set.high,
                    disabled_at: None,
                },
            ]);

            let identity = Identity {
                protocol_version: PROTOCOL_VERSION,
                id: data_contract.owner_id,
                // TODO: It super inconvenient to have this boilerplate everywhere and there is no
                //  way to control consistency. BTreeMap must be internal structure of IdentityPublicKey
                public_keys: public_keys.into_iter().map(|pk| (pk.id, pk)).collect(),
                balance: 0,
                revision: 0,
                asset_lock_proof: None,
                metadata: None,
            };

            self.register_system_data_contract_operations(data_contract, &mut operations);

            self.register_system_identity_operations(identity, &mut operations);
        }

        self.register_dpns_top_level_domain_operations(&dpns_contract, &mut operations)?;

        let block_info = BlockInfo::default_with_time(genesis_time);

        self.drive
            .apply_drive_operations(operations, true, &block_info, transaction)?;

        Ok(())
    }

    fn register_system_data_contract_operations(
        &self,
        data_contract: DataContract,
        operations: &mut Vec<DriveOperationType>,
    ) {
        let serialization = data_contract.to_cbor().unwrap();
        operations.push(DriveOperationType::ContractOperation(
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
        operations: &mut Vec<DriveOperationType>,
    ) {
        operations.push(DriveOperationType::IdentityOperation(
            IdentityOperationType::AddNewIdentity { identity },
        ))
    }

    fn register_dpns_top_level_domain_operations<'a>(
        &self,
        contract: &'a DataContract,
        operations: &mut Vec<DriveOperationType<'a>>,
    ) -> Result<(), Error> {
        let domain = "dash";

        let preorder_salt_string = encode(&DPNS_DASH_TLD_PREORDER_SALT, Encoding::Base64);
        let alias_identity_id = encode(&contract.owner_id.to_buffer(), Encoding::Base58);

        // TODO: Add created and updated at to DPNS contract

        let document = ExtendedDocument {
            protocol_version: PROTOCOL_VERSION,
            id: Identifier::new(DPNS_DASH_TLD_DOCUMENT_ID),
            document_type: "domain".to_string(),
            revision: 0,
            data_contract_id: contract.id,
            owner_id: contract.owner_id,
            created_at: None,
            updated_at: None,
            data: json!({
                "label": domain,
                "normalizedLabel": domain,
                "normalizedParentDomainName": "",
                "preorderSalt": preorder_salt_string,
                "records": {
                    "dashAliasIdentityId": alias_identity_id,
                },
                "subdomainRules": {
                    "allowSubdomains": true,
                }
            }),
            data_contract: contract.clone(),
            metadata: None,
            entropy: [0; 32],
        };

        let document_stub_properties_value: Value = cbor!({
            "label" => domain,
            "normalizedLabel" => domain,
            "normalizedParentDomainName" => "",
            "preorderSalt" => CborValue::Bytes(DPNS_DASH_TLD_PREORDER_SALT.to_vec()),
            "records" => {
                "dashAliasIdentityId" => CborValue::Bytes(contract.owner_id.to_buffer_vec()),
            },
        })
        .map_err(|_| {
            // TODO: Can't pass original error because the error expecting String
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "can't create cbor for dpns tld",
            ))
        })?
        .into();

        let document_stub_properties = document_stub_properties_value
            .into_btree_map()
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

        let document_cbor = document.to_buffer()?;

        let document = Document {
            id: DPNS_DASH_TLD_DOCUMENT_ID,
            properties: document_stub_properties,
            owner_id: contract.owner_id.to_buffer(),
            revision: None,
            created_at: None,
            updated_at: None,
        };

        let document_type = contract.document_type_for_name("domain")?;

        let operation =
            DriveOperationType::DocumentOperation(DocumentOperationType::AddDocumentForContract {
                document_and_contract_info: DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        //todo: remove cbor and use DocumentInfo::DocumentWithoutSerialization((document, None))
                        document_info: DocumentInfo::DocumentAndSerialization((
                            document,
                            document_cbor,
                            None,
                        )),
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
        use crate::test::helpers::setup::setup_platform_with_genesis_state;

        #[test]
        pub fn should_create_genesis_state_deterministically() {
            let platform = setup_platform_with_genesis_state(None);

            let root_hash = platform
                .drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should obtain root hash");

            assert_eq!(
                root_hash,
                [
                    59, 16, 30, 145, 9, 47, 66, 85, 133, 88, 194, 109, 241, 15, 226, 214, 163, 196,
                    146, 107, 122, 145, 111, 45, 251, 242, 250, 157, 153, 43, 219, 184
                ]
            )
        }
    }
}
