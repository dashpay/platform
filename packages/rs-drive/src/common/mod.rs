use crate::contract::Contract;
use crate::drive::Drive;
use byteorder::{BigEndian, WriteBytesExt};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn setup_contract(drive: &Drive, path: &str, transaction: grovedb::TransactionArg) -> Contract {
    let contract_cbor = json_document_to_cbor(path, Some(crate::drive::defaults::PROTOCOL_VERSION));
    let contract = Contract::from_cbor(&contract_cbor).expect("contract should be deserialized");
    drive
        .apply_contract(contract_cbor, 0f64, transaction)
        .expect("contract should be applied");
    contract
}

pub fn setup_contract_from_hex(
    drive: &Drive,
    hex_string: String,
    transaction: grovedb::TransactionArg,
) -> Contract {
    let contract_cbor = cbor_from_hex(hex_string);
    let contract = Contract::from_cbor(&contract_cbor).expect("contract should be deserialized");
    drive
        .apply_contract(contract_cbor, 0f64, transaction)
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
        buffer.write_u32::<BigEndian>(protocol_version);
    }
    ciborium::ser::into_writer(&value, &mut buffer).expect("unable to serialize into cbor");
    buffer
}

pub fn cbor_from_hex(hex_string: String) -> Vec<u8> {
    let decoded = hex::decode(hex_string).expect("Decoding failed");

    decoded
}

pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}
