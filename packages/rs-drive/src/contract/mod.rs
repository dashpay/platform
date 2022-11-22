pub mod document;
pub mod random_document;

use ciborium::value::Value;

/// Import from dpp
pub use dpp::data_contract::{
    extra::{DocumentField, DocumentFieldType, DocumentType},
    extra::{Index, IndexProperty},
    DataContract, DataContract as Contract,
};

/// Import from random_document
pub use random_document::CreateRandomDocument;

fn reduced_value_string_representation(value: &Value) -> String {
    match value {
        Value::Integer(integer) => {
            let i: i128 = (*integer).try_into().unwrap();
            format!("{}", i)
        }
        Value::Bytes(bytes) => hex::encode(bytes),
        Value::Float(float) => {
            format!("{}", float)
        }
        Value::Text(text) => {
            let len = text.len();
            if len > 20 {
                let first_text = text.split_at(20).0.to_string();
                format!("{}[...({})]", first_text, len)
            } else {
                text.clone()
            }
        }
        Value::Bool(b) => {
            format!("{}", b)
        }
        Value::Null => "None".to_string(),
        Value::Tag(_, _) => "Tag".to_string(),
        Value::Array(_) => "Array".to_string(),
        Value::Map(_) => "Map".to_string(),
        _ => "".to_string(),
    }
}
