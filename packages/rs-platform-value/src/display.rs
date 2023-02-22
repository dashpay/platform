use crate::Value;

impl Value {
    pub fn string_representation(&self) -> String {
        match self {
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
            Value::U128(i) => format!("{}", i),
            Value::I128(i) => format!("{}", i),
            Value::U64(i) => format!("{}", i),
            Value::I64(i) => format!("{}", i),
            Value::U32(i) => format!("{}", i),
            Value::I32(i) => format!("{}", i),
            Value::U16(i) => format!("{}", i),
            Value::I16(i) => format!("{}", i),
            Value::U8(i) => format!("{}", i),
            Value::I8(i) => format!("{}", i),
        }
    }
}