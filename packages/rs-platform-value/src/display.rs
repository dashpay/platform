use crate::Value;
use std::fmt::{Display, Formatter};

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string_representation())
    }
}

impl Value {
    fn string_representation(&self) -> String {
        match self {
            Value::Bytes(bytes) => format!("bytes {}", hex::encode(bytes)),
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
            Value::Null => "Null".to_string(),
            Value::Tag(_, _) => "Tag".to_string(),
            Value::Array(value) => {
                let inner_values = value
                    .iter()
                    .map(|v| v.string_representation())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("array of [{}]", inner_values)
            }
            Value::Map(_) => "Map".to_string(),
            Value::U128(i) => format!("(u128){}", i),
            Value::I128(i) => format!("(i128){}", i),
            Value::U64(i) => format!("(u64){}", i),
            Value::I64(i) => format!("(i64){}", i),
            Value::U32(i) => format!("(u32){}", i),
            Value::I32(i) => format!("(i32){}", i),
            Value::U16(i) => format!("(u16){}", i),
            Value::I16(i) => format!("(i16){}", i),
            Value::U8(i) => format!("(u8){}", i),
            Value::I8(i) => format!("(i8){}", i),
            Value::Bytes32(bytes32) => format!(
                "bytes32 {}",
                base64::encode(bytes32.as_slice())
            ),
            Value::Identifier(identifier) => format!(
                "identifier {}",
                bs58::encode(identifier.as_slice()).into_string()
            ),
        }
    }
}
