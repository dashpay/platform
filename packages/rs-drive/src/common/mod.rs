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

//! Common functions
//!
//! This module defines general, commonly used functions in Drive.
//!

#[cfg(any(feature = "full", feature = "verify"))]
pub mod encode;
#[cfg(feature = "full")]
/// Helpers module
pub mod helpers;

#[cfg(feature = "full")]
use std::fs::File;
#[cfg(feature = "full")]
use std::io;
#[cfg(feature = "full")]
use std::io::BufRead;
#[cfg(feature = "full")]
use std::option::Option::None;
#[cfg(feature = "full")]
use std::path::Path;

#[cfg(feature = "full")]
use ciborium::value::Value;

#[cfg(feature = "full")]
use grovedb::TransactionArg;

#[cfg(feature = "full")]
use crate::contract::Contract;
#[cfg(feature = "full")]
use crate::drive::Drive;

#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::extra::common::json_document_to_contract_with_ids;
use dpp::prelude::Identifier;

#[cfg(feature = "full")]
/// Serializes to CBOR and applies to Drive a JSON contract from the file system.
pub fn setup_contract(
    drive: &Drive,
    path: &str,
    contract_id: Option<[u8; 32]>,
    transaction: TransactionArg,
) -> Contract {
    let contract =
        json_document_to_contract_with_ids(path, contract_id.map(|i| Identifier::from(i)), None)
            .expect("expected to get cbor contract");

    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, transaction)
        .expect("contract should be applied");
    contract
}

#[cfg(feature = "full")]
/// Serializes to CBOR and applies to Drive a contract from hex string format.
pub fn setup_contract_from_cbor_hex(
    drive: &Drive,
    hex_string: String,
    transaction: TransactionArg,
) -> Contract {
    let contract_cbor = cbor_from_hex(hex_string);
    let contract = Contract::from_cbor(&contract_cbor).expect("contract should be deserialized");
    drive
        .apply_contract_cbor(
            contract_cbor,
            None,
            BlockInfo::default(),
            true,
            None,
            transaction,
        )
        .expect("contract should be applied");
    contract
}

#[cfg(feature = "full")]
/// Serializes a hex string to CBOR.
pub fn cbor_from_hex(hex_string: String) -> Vec<u8> {
    hex::decode(hex_string).expect("Decoding failed")
}

#[cfg(feature = "full")]
/// Takes a file and returns the lines as a list of strings.
pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}

#[cfg(feature = "full")]
/// Retrieves the value of a key from a CBOR map.
pub fn get_key_from_cbor_map<'a>(
    cbor_map: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Value> {
    for (cbor_key, cbor_value) in cbor_map.iter() {
        if !cbor_key.is_text() {
            continue;
        }

        if cbor_key.as_text().expect("confirmed as text") == key {
            return Some(cbor_value);
        }
    }
    None
}

#[cfg(feature = "full")]
/// Retrieves the value of a key from a CBOR map if it's a map itself.
pub fn cbor_inner_map_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Vec<(Value, Value)>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Map(map_value) = key_value {
        return Some(map_value);
    }
    None
}
