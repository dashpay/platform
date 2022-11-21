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

use std::collections::BTreeMap;

use ciborium::value::Value;
use rs_drive::dpp::identity::Identity;

use rs_drive::common::helpers::identities::create_test_identity;
use rs_drive::contract::document::Document;
use rs_drive::contract::Contract;
use rs_drive::dpp::data_contract::extra::DriveContractExt;
use rs_drive::drive::block_info::BlockInfo;
use rs_drive::drive::flags::StorageFlags;
use rs_drive::drive::object_size_info::DocumentAndContractInfo;
use rs_drive::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use rs_drive::drive::Drive;
use rs_drive::grovedb::TransactionArg;

use crate::contracts::reward_shares::MN_REWARD_SHARES_DOCUMENT_TYPE;
use crate::error::Error;

/// A function which creates a test MN_REWARD_SHARES_DOCUMENT_TYPE document.
fn create_test_mn_share_document(
    drive: &Drive,
    contract: &Contract,
    identity: &Identity,
    pay_to_identity: &Identity,
    percentage: u16,
    transaction: TransactionArg,
) -> Document {
    let id = rand::random::<[u8; 32]>();

    let mut properties: BTreeMap<String, Value> = BTreeMap::new();

    properties.insert(
        String::from("payToId"),
        Value::Bytes(pay_to_identity.id.buffer.to_vec()),
    );
    properties.insert(String::from("percentage"), percentage.into());

    let document = Document {
        id,
        properties,
        owner_id: identity.id.buffer,
    };

    let document_type = contract
        .document_type_for_name(MN_REWARD_SHARES_DOCUMENT_TYPE)
        .expect("expected to get a document type");

    let storage_flags = Some(StorageFlags::SingleEpoch(0));

    let document_cbor = document.to_cbor();

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                document_info: DocumentRefAndSerialization((
                    &document,
                    &document_cbor,
                    storage_flags.as_ref(),
                )),
                contract,
                document_type,
                owner_id: None,
            },
            false,
            BlockInfo::genesis(),
            true,
            transaction,
        )
        .expect("expected to insert a document successfully");

    document
}

/// A function which creates a vector of test identities with
/// a test MN_REWARD_SHARES_DOCUMENT_TYPE document for each.
pub fn create_test_masternode_share_identities_and_documents(
    drive: &Drive,
    contract: &Contract,
    pro_tx_hashes: &Vec<[u8; 32]>,
    transaction: TransactionArg,
) -> Vec<(Identity, Document)> {
    drive
        .fetch_identities(pro_tx_hashes, transaction)
        .expect("expected to fetch identities")
        .iter()
        .map(|mn_identity| {
            let id: [u8; 32] = rand::random();
            let identity = create_test_identity(drive, id, transaction);
            let document = create_test_mn_share_document(
                drive,
                contract,
                mn_identity,
                &identity,
                5000,
                transaction,
            );

            (identity, document)
        })
        .collect()
}

/// A function for refetching identities.
///
/// Takes a list of identities, queries the database for them, and returns the query result as a list of identities.
pub fn refetch_identities(
    drive: &Drive,
    identities: Vec<&Identity>,
    transaction: TransactionArg,
) -> Result<Vec<Identity>, Error> {
    let ids = identities
        .into_iter()
        .map(|identity| identity.id.buffer)
        .collect();

    drive
        .fetch_identities(&ids, transaction)
        .map_err(Error::Drive)
}
