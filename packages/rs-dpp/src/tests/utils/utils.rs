use anyhow::Result;
use dashcore::{Block, BlockHeader};
use getrandom::getrandom;
use serde_json::Value;

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

pub fn generate_random_identifier() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    buffer
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
pub fn serde_set_ref<T, S>(object: &mut Value, key: T, value: S)
where
    T: Into<String>,
    S: Into<serde_json::Value>,
    serde_json::Value: From<S>,
{
    let map = object
        .as_object_mut()
        .expect("Expected value to be an JSON object");
    map.insert(key.into(), serde_json::Value::from(value));
}

/// Removes a key value pair in serde_json object, returns the modified object
pub fn serde_remove<T>(mut object: serde_json::Value, key: T) -> serde_json::Value
where
    T: Into<String>,
{
    let map = object
        .as_object_mut()
        .expect("Expected value to be an JSON object");
    map.remove(&key.into());

    object
}

/// Removes a key value pair in serde_json object, returns the modified object
pub fn serde_remove_ref<T>(object: &mut Value, key: T)
where
    T: Into<String>,
{
    object
        .as_object_mut()
        .expect("Expected value to be an JSON object")
        .remove(&key.into());
}

pub fn generate_random_identifier_struct() -> Identifier {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    return Identifier::from_bytes(&buffer).unwrap();
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
    fn get_value(&self, key: impl Into<String>) -> &Value;
    fn get_value_mut(&mut self, key: impl Into<String>) -> &mut Value;
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
        S: Into<Value>,
        Value: From<S>,
    {
        let map = self
            .as_object_mut()
            .expect("Expected value to be an JSON object");
        map.insert(key.into(), serde_json::Value::from(value));
    }

    fn get_value(&self, key: impl Into<String>) -> &Value {
        self.as_object()
            .expect("Expected key to exist")
            .get(&key.into())
            .expect("Expected key to exist")
    }

    fn get_value_mut(&mut self, key: impl Into<String>) -> &mut Value {
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
        if $validation_result.errors().len() != $expected_errors_count {
            for error in $validation_result.errors().iter() {
                println!("{:?}", error);
            }
        }

        assert_eq!($validation_result.errors().len(), $expected_errors_count);

        let mut errors = Vec::new();

        for error in $validation_result.errors() {
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

pub fn create_empty_block(timestamp_secs: Option<u32>) -> Block {
    Block {
        txdata: vec![],
        header: new_block_header(timestamp_secs),
    }
}

pub fn new_block_header(timestamp_secs: Option<u32>) -> BlockHeader {
    BlockHeader {
        bits: 0,
        nonce: 0,
        merkle_root: Default::default(),
        prev_blockhash: Default::default(),
        version: 0,
        time: timestamp_secs.unwrap_or_default(),
    }
}
