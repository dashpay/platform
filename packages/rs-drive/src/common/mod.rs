use crate::contract::Contract;
use crate::drive::Drive;
use byteorder::{BigEndian, WriteBytesExt};
use grovedb::TransactionArg;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::option::Option::None;
use std::path::Path;

pub fn setup_contract(
    drive: &Drive,
    path: &str,
    contract_id: Option<[u8; 32]>,
    transaction: TransactionArg,
) -> Contract {
    let contract_cbor = json_document_to_cbor(path, Some(crate::drive::defaults::PROTOCOL_VERSION));
    let contract =
        Contract::from_cbor(&contract_cbor, contract_id).expect("contract should be deserialized");
    drive
        .apply_contract_cbor(contract_cbor, contract_id, 0f64, transaction)
        .expect("contract should be applied");
    contract
}

pub fn setup_contract_from_hex(
    drive: &Drive,
    hex_string: String,
    transaction: TransactionArg,
) -> Contract {
    let contract_cbor = cbor_from_hex(hex_string);
    let contract =
        Contract::from_cbor(&contract_cbor, None).expect("contract should be deserialized");
    drive
        .apply_contract_cbor(contract_cbor, None, 0f64, transaction)
        .expect("contract should be applied");
    contract
}

pub fn json_document_to_cbor(path: impl AsRef<Path>, protocol_version: Option<u32>) -> Vec<u8> {
    let file = File::open(path).expect("file not found");
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).expect("expected a valid json");
    value_to_cbor(json, protocol_version)
}

pub fn value_to_cbor(value: serde_json::Value, protocol_version: Option<u32>) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    if let Some(protocol_version) = protocol_version {
        buffer
            .write_u32::<BigEndian>(protocol_version)
            .expect("writing protocol version caused error");
    }
    ciborium::ser::into_writer(&value, &mut buffer).expect("unable to serialize into cbor");
    buffer
}

pub fn cbor_from_hex(hex_string: String) -> Vec<u8> {
    hex::decode(hex_string).expect("Decoding failed")
}

pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}
