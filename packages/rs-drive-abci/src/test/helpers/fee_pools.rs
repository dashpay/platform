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
//

//! Fee pool helper functions.
//!
//! This module defines helper functions related to fee distribution pools.
//!

use std::borrow::Cow;
use std::collections::BTreeMap;

use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use drive::dpp::identity::Identity;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::data_contract::DataContract;
use dpp::document::{DocumentV0, INITIAL_REVISION};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::system_data_contracts::masternode_reward_shares_contract::v1::document_types;
use dpp::version::PlatformVersion;
use drive::common::identities::create_test_identity_with_rng;
use drive::dpp::document::Document;
use drive::drive::flags::StorageFlags;
use drive::drive::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

/// A function which creates a test MN_REWARD_SHARES_DOCUMENT_TYPE document.
fn create_test_mn_share_document(
    drive: &Drive,
    contract: &DataContract,
    identity_id: Identifier,
    pay_to_identity: &Identity,
    percentage: u16,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Document {
    let id = Identifier::random();

    let mut properties: BTreeMap<String, Value> = BTreeMap::new();

    properties.insert(
        String::from("payToId"),
        Value::Bytes(pay_to_identity.id().to_buffer().to_vec()),
    );
    properties.insert(String::from("percentage"), percentage.into());

    let document = DocumentV0 {
        id,
        properties,
        owner_id: identity_id,
        revision: Some(INITIAL_REVISION),
        created_at: None,
        updated_at: None,
    }
    .into();

    let document_type = contract
        .document_type_for_name(document_types::reward_share::NAME)
        .expect("expected to get a document type");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, storage_flags)),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::genesis(),
            true,
            transaction,
            platform_version,
        )
        .expect("expected to insert a document successfully");

    document
}

/// A function which creates a vector of test identities with
/// a test MN_REWARD_SHARES_DOCUMENT_TYPE document for each.
pub fn create_test_masternode_share_identities_and_documents(
    drive: &Drive,
    contract: &DataContract,
    pro_tx_hashes: &Vec<[u8; 32]>,
    seed: Option<u64>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Vec<(Identity, Document)> {
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };
    let all_exist = drive
        .verify_all_identities_exist(pro_tx_hashes, transaction)
        .expect("expected that all identities existed");
    if all_exist {
        pro_tx_hashes
            .iter()
            .map(|mn_identity| {
                let id = rng.gen::<[u8; 32]>();
                let identity = create_test_identity_with_rng(
                    drive,
                    id,
                    &mut rng,
                    transaction,
                    platform_version,
                )
                .expect("expected to create a test identity");
                let document = create_test_mn_share_document(
                    drive,
                    contract,
                    Identifier::new(*mn_identity),
                    &identity,
                    5000,
                    transaction,
                    platform_version,
                );
                (identity, document)
            })
            .collect()
    } else {
        panic!("all identities didn't exist")
    }
}
