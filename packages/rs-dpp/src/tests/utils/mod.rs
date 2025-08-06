use anyhow::Result;
use dashcore::block::Version;
use dashcore::hashes::Hash;
use dashcore::{Block, BlockHash, CompactTarget, Header, TxMerkleNode};
use platform_value::Value;
#[cfg(test)]
use serde_json::Value as JsonValue;

use crate::prelude::Identifier;

#[cfg(test)]
#[macro_export]
macro_rules! assert_error_contains {
    ($result:ident, $contains:expr) => {
        match $result {
            Ok(o) => {
                panic!("expected error, but returned: {:?}", o);
            }
            Err(e) => {
                let string_error = e.to_string();
                if !string_error.contains($contains) {
                    panic!(
                        "assertion error: '{}' hasn't been found in '{}'",
                        $contains, string_error
                    );
                }
            }
        }
    };
}

/// Sets a key value pair in serde_json object, returns the modified object
pub fn serde_set<T, S>(mut object: serde_json::Value, key: T, value: S) -> serde_json::Value
where
    T: Into<String>,
    S: Into<serde_json::Value>,
    serde_json::Value: From<S>,
{
    let map = object
        .as_object_mut()
        .expect("Expected value to be an JSON object");
    map.insert(key.into(), serde_json::Value::from(value));

    object
}

/// Sets a key value pair in serde_json object, returns the modified object
pub fn platform_value_set_ref<T, S>(object: &mut Value, key: T, value: S)
where
    T: Into<Value>,
    S: Into<Value>,
    Value: From<S>,
{
    let map = object
        .as_map_mut()
        .expect("Expected value to be an JSON object");
    map.push((key.into(), value.into()));
}

pub fn generate_random_identifier_struct() -> Identifier {
    let mut buffer = [0u8; 32];
    getrandom::fill(&mut buffer).unwrap();
    Identifier::from_bytes(&buffer).unwrap()
}

pub fn get_data_from_file(file_path: &str) -> Result<String> {
    let current_dir = std::env::current_dir()?;
    let file_path = format!("{}/{}", current_dir.display(), file_path);
    let d = std::fs::read_to_string(file_path)?;
    Ok(d)
}

#[cfg(test)]
pub trait SerdeTestExtension {
    fn remove_key(&mut self, key: impl Into<String>);
    fn set_key_value<T, S>(&mut self, key: T, value: S)
    where
        T: Into<String>,
        S: Into<serde_json::Value>,
        serde_json::Value: From<S>;
    fn get_value(&self, key: impl Into<String>) -> &serde_json::Value;
    fn get_value_mut(&mut self, key: impl Into<String>) -> &mut serde_json::Value;
}

#[cfg(test)]
impl SerdeTestExtension for serde_json::Value {
    fn remove_key(&mut self, key: impl Into<String>) {
        self.as_object_mut()
            .expect("Expected value to be an JSON object")
            .remove(&key.into());
    }

    fn set_key_value<T, S>(&mut self, key: T, value: S)
    where
        T: Into<String>,
        S: Into<JsonValue>,
        JsonValue: From<S>,
    {
        let map = self
            .as_object_mut()
            .expect("Expected value to be an JSON object");
        map.insert(key.into(), serde_json::Value::from(value));
    }

    fn get_value(&self, key: impl Into<String>) -> &JsonValue {
        self.as_object()
            .expect("Expected key to exist")
            .get(&key.into())
            .expect("Expected key to exist")
    }

    fn get_value_mut(&mut self, key: impl Into<String>) -> &mut JsonValue {
        self.as_object_mut()
            .expect("Expected key to exist")
            .get_mut(&key.into())
            .expect("Expected key to exist")
    }
}

// fn byte_to_hex(byte: &u8) -> String {
//     format!("{:02x}", byte)
// }
//
// /// Serializes bytes into a hex string
// pub fn encode_hex<T: Clone + Into<Vec<u8>>>(bytes: &T) -> String {
//     let hex_vec: Vec<String> = bytes.clone().into().iter().map(byte_to_hex).collect();
//
//     hex_vec.join("")
// }

/// Assert that all validation error belong to a certain enum variant and
/// extracts all the errors from enum to a vector
#[macro_export]
macro_rules! assert_consensus_errors {
    ($validation_result: expr, $variant: path, $expected_errors_count: expr) => {{
        if $validation_result.errors.len() != $expected_errors_count {
            for error in $validation_result.errors.iter() {
                println!("{:?}", error);
            }
        }

        assert_eq!($validation_result.errors.len(), $expected_errors_count);

        let mut errors = Vec::new();

        for error in &$validation_result.errors {
            match error {
                $variant(err) => errors.push(err),
                err => {
                    panic!("Got error that differs from what was expected: {:?}", err)
                }
            }
        }

        errors
    }};
}

/// Assert that all validation error belong to a certain enum variant of basic consensus errors
/// and extracts all the errors from enum to a vector
#[macro_export]
macro_rules! assert_basic_consensus_errors {
    ($validation_result: expr, $variant: path, $expected_errors_count: expr) => {{
        if $validation_result.errors.len() != $expected_errors_count {
            for error in $validation_result.errors.iter() {
                println!("{:?}", error);
            }
        }

        assert_eq!($validation_result.errors.len(), $expected_errors_count);

        let mut errors = Vec::new();

        for error in &$validation_result.errors {
            match error {
                ConsensusError::BasicError($variant(err)) => errors.push(err),
                err => {
                    panic!("Got error that differs from what was expected: {:?}", err)
                }
            }
        }

        errors
    }};
}

/// Assert that all validation error belong to a certain enum variant of state consensus errors
/// and extracts all the errors from enum to a vector
#[macro_export]
macro_rules! assert_state_consensus_errors {
    ($validation_result: expr, $variant: path, $expected_errors_count: expr) => {{
        if $validation_result.errors.len() != $expected_errors_count {
            for error in $validation_result.errors.iter() {
                println!("{:?}", error);
            }
        }

        assert_eq!($validation_result.errors.len(), $expected_errors_count);

        let mut errors = Vec::new();

        for error in &$validation_result.errors {
            match error {
                ConsensusError::StateError($variant(err)) => errors.push(err),
                err => {
                    panic!("Got error that differs from what was expected: {:?}", err)
                }
            }
        }

        errors
    }};
}

pub fn create_empty_block(timestamp_secs: Option<u32>) -> Block {
    Block {
        txdata: vec![],
        header: new_block_header(timestamp_secs),
    }
}

pub fn new_block_header(timestamp_secs: Option<u32>) -> Header {
    Header {
        bits: CompactTarget::default(),
        nonce: 0,
        merkle_root: TxMerkleNode::all_zeros(),
        prev_blockhash: BlockHash::all_zeros(),
        version: Version::default(),
        time: timestamp_secs.unwrap_or_default(),
    }
}
