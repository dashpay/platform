use crate::Value;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::fmt::{Display, Formatter};

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string_representation())
    }
}

impl Value {
    pub fn non_qualified_string_representation(&self) -> String {
        match self {
            Value::Bytes(bytes) => format!("bytes {}", hex::encode(bytes)),
            Value::Float(float) => {
                format!("{}", float)
            }
            Value::Text(text) => text.clone(),
            Value::Bool(b) => {
                format!("{}", b)
            }
            Value::Null => "Null".to_string(),
            Value::Array(value) => {
                let inner_values = value
                    .iter()
                    .map(|v| v.string_representation())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("array of [{}]", inner_values)
            }
            Value::Map(map) => {
                let inner_string = map
                    .iter()
                    .map(|(key, value)| format!("{key}: {value}"))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("Map {{ {} }}", inner_string)
            }
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
            Value::Bytes20(bytes20) => {
                format!("bytes20 {}", BASE64_STANDARD.encode(bytes20.as_slice()))
            }
            Value::Bytes32(bytes32) => {
                format!("bytes32 {}", BASE64_STANDARD.encode(bytes32.as_slice()))
            }
            Value::Bytes36(bytes36) => {
                format!("bytes36 {}", BASE64_STANDARD.encode(bytes36.as_slice()))
            }
            Value::Identifier(identifier) => format!(
                "identifier {}",
                bs58::encode(identifier.as_slice()).into_string()
            ),
            Value::EnumU8(_) => "enum u8".to_string(),
            Value::EnumString(_) => "enum string".to_string(),
        }
    }

    fn string_representation(&self) -> String {
        match self {
            Value::Bytes(bytes) => format!("bytes {}", hex::encode(bytes)),
            Value::Float(float) => {
                format!("float {}", float)
            }
            Value::Text(text) => {
                let len = text.len();
                if len > 20 {
                    let first_text = text.split_at(20).0.to_string();
                    format!("string {}[...({})]", first_text, len)
                } else {
                    format!("string {}", text)
                }
            }
            Value::Bool(b) => {
                format!("bool {}", b)
            }
            Value::Null => "Null".to_string(),
            Value::Array(value) => {
                let inner_values = value
                    .iter()
                    .map(|v| v.string_representation())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("array of [{}]", inner_values)
            }
            Value::Map(map) => {
                let inner_string = map
                    .iter()
                    .map(|(key, value)| format!("{key}: {value}"))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("Map {{ {} }}", inner_string)
            }
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
            Value::Bytes20(bytes20) => {
                format!("bytes20 {}", BASE64_STANDARD.encode(bytes20.as_slice()))
            }
            Value::Bytes32(bytes32) => {
                format!("bytes32 {}", BASE64_STANDARD.encode(bytes32.as_slice()))
            }
            Value::Bytes36(bytes36) => {
                format!("bytes36 {}", BASE64_STANDARD.encode(bytes36.as_slice()))
            }
            Value::Identifier(identifier) => format!(
                "identifier {}",
                bs58::encode(identifier.as_slice()).into_string()
            ),
            Value::EnumU8(_) => "enum u8".to_string(),
            Value::EnumString(_) => "enum string".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;
    use base64::Engine;

    // ---- Display (string_representation) tests ----

    #[test]
    fn display_null() {
        assert_eq!(format!("{}", Value::Null), "Null");
    }

    #[test]
    fn display_bool_true() {
        assert_eq!(format!("{}", Value::Bool(true)), "bool true");
    }

    #[test]
    fn display_bool_false() {
        assert_eq!(format!("{}", Value::Bool(false)), "bool false");
    }

    #[test]
    fn display_u128() {
        assert_eq!(
            format!("{}", Value::U128(340282366920938463463374607431768211455)),
            "(u128)340282366920938463463374607431768211455"
        );
    }

    #[test]
    fn display_i128() {
        assert_eq!(format!("{}", Value::I128(-42)), "(i128)-42");
    }

    #[test]
    fn display_u64() {
        assert_eq!(format!("{}", Value::U64(1000)), "(u64)1000");
    }

    #[test]
    fn display_i64() {
        assert_eq!(format!("{}", Value::I64(-999)), "(i64)-999");
    }

    #[test]
    fn display_u32() {
        assert_eq!(format!("{}", Value::U32(42)), "(u32)42");
    }

    #[test]
    fn display_i32() {
        assert_eq!(format!("{}", Value::I32(-1)), "(i32)-1");
    }

    #[test]
    fn display_u16() {
        assert_eq!(format!("{}", Value::U16(65535)), "(u16)65535");
    }

    #[test]
    fn display_i16() {
        assert_eq!(format!("{}", Value::I16(-32768)), "(i16)-32768");
    }

    #[test]
    fn display_u8() {
        assert_eq!(format!("{}", Value::U8(255)), "(u8)255");
    }

    #[test]
    fn display_i8() {
        assert_eq!(format!("{}", Value::I8(-128)), "(i8)-128");
    }

    #[test]
    fn display_float() {
        let s = format!("{}", Value::Float(3.14));
        assert!(s.starts_with("float 3.14"));
    }

    #[test]
    fn display_text_short() {
        let text = "hello";
        assert_eq!(format!("{}", Value::Text(text.to_string())), "string hello");
    }

    #[test]
    fn display_text_exactly_20_chars() {
        let text = "12345678901234567890"; // exactly 20
        assert_eq!(
            format!("{}", Value::Text(text.to_string())),
            format!("string {}", text)
        );
    }

    #[test]
    fn display_text_long_truncated() {
        let text = "123456789012345678901"; // 21 chars
        let result = format!("{}", Value::Text(text.to_string()));
        assert_eq!(result, "string 12345678901234567890[...(21)]");
    }

    #[test]
    fn display_text_long_50_chars() {
        let text = "a".repeat(50);
        let result = format!("{}", Value::Text(text));
        assert_eq!(result, "string aaaaaaaaaaaaaaaaaaaa[...(50)]");
    }

    #[test]
    fn display_bytes_empty() {
        assert_eq!(format!("{}", Value::Bytes(vec![])), "bytes ");
    }

    #[test]
    fn display_bytes_non_empty() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        assert_eq!(format!("{}", Value::Bytes(bytes)), "bytes deadbeef");
    }

    #[test]
    fn display_bytes20() {
        let bytes = [0u8; 20];
        let result = format!("{}", Value::Bytes20(bytes));
        assert!(result.starts_with("bytes20 "));
        // base64 of 20 zero bytes
        let expected_b64 = base64::prelude::BASE64_STANDARD.encode(&bytes);
        assert_eq!(result, format!("bytes20 {}", expected_b64));
    }

    #[test]
    fn display_bytes32() {
        let bytes = [1u8; 32];
        let result = format!("{}", Value::Bytes32(bytes));
        let expected_b64 = base64::prelude::BASE64_STANDARD.encode(&bytes);
        assert_eq!(result, format!("bytes32 {}", expected_b64));
    }

    #[test]
    fn display_bytes36() {
        let bytes = [0xffu8; 36];
        let result = format!("{}", Value::Bytes36(bytes));
        let expected_b64 = base64::prelude::BASE64_STANDARD.encode(&bytes);
        assert_eq!(result, format!("bytes36 {}", expected_b64));
    }

    #[test]
    fn display_identifier() {
        let id = [42u8; 32];
        let result = format!("{}", Value::Identifier(id));
        let expected_b58 = bs58::encode(&id).into_string();
        assert_eq!(result, format!("identifier {}", expected_b58));
    }

    #[test]
    fn display_array_empty() {
        assert_eq!(format!("{}", Value::Array(vec![])), "array of []");
    }

    #[test]
    fn display_array_with_elements() {
        let arr = vec![Value::U8(1), Value::Bool(true)];
        let result = format!("{}", Value::Array(arr));
        assert_eq!(result, "array of [(u8)1, bool true]");
    }

    #[test]
    fn display_map_empty() {
        assert_eq!(format!("{}", Value::Map(vec![])), "Map {  }");
    }

    #[test]
    fn display_map_with_entries() {
        let map = vec![(Value::Text("key".to_string()), Value::U32(99))];
        let result = format!("{}", Value::Map(map));
        assert_eq!(result, "Map { string key: (u32)99 }");
    }

    #[test]
    fn display_enum_u8() {
        assert_eq!(format!("{}", Value::EnumU8(vec![1, 2])), "enum u8");
    }

    #[test]
    fn display_enum_string() {
        assert_eq!(
            format!(
                "{}",
                Value::EnumString(vec!["a".to_string(), "b".to_string()])
            ),
            "enum string"
        );
    }

    // ---- non_qualified_string_representation tests ----

    #[test]
    fn non_qualified_null() {
        assert_eq!(Value::Null.non_qualified_string_representation(), "Null");
    }

    #[test]
    fn non_qualified_bool_true() {
        assert_eq!(
            Value::Bool(true).non_qualified_string_representation(),
            "true"
        );
    }

    #[test]
    fn non_qualified_bool_false() {
        assert_eq!(
            Value::Bool(false).non_qualified_string_representation(),
            "false"
        );
    }

    #[test]
    fn non_qualified_u64() {
        assert_eq!(
            Value::U64(12345).non_qualified_string_representation(),
            "12345"
        );
    }

    #[test]
    fn non_qualified_i64() {
        assert_eq!(Value::I64(-42).non_qualified_string_representation(), "-42");
    }

    #[test]
    fn non_qualified_float() {
        let s = Value::Float(2.5).non_qualified_string_representation();
        assert_eq!(s, "2.5");
    }

    #[test]
    fn non_qualified_text_returns_raw_string() {
        // non_qualified does NOT prepend "string" or truncate
        let text = "a long text that is more than twenty characters";
        assert_eq!(
            Value::Text(text.to_string()).non_qualified_string_representation(),
            text
        );
    }

    #[test]
    fn non_qualified_bytes() {
        let bytes = vec![0xca, 0xfe];
        assert_eq!(
            Value::Bytes(bytes).non_qualified_string_representation(),
            "bytes cafe"
        );
    }

    #[test]
    fn non_qualified_identifier() {
        let id = [7u8; 32];
        let expected_b58 = bs58::encode(&id).into_string();
        assert_eq!(
            Value::Identifier(id).non_qualified_string_representation(),
            format!("identifier {}", expected_b58)
        );
    }

    #[test]
    fn non_qualified_array() {
        let arr = vec![Value::U8(1)];
        let result = Value::Array(arr).non_qualified_string_representation();
        assert_eq!(result, "array of [(u8)1]");
    }

    #[test]
    fn non_qualified_map() {
        let map = vec![(Value::Text("k".to_string()), Value::Null)];
        let result = Value::Map(map).non_qualified_string_representation();
        assert_eq!(result, "Map { string k: Null }");
    }

    #[test]
    fn non_qualified_u128() {
        assert_eq!(
            Value::U128(999).non_qualified_string_representation(),
            "999"
        );
    }

    #[test]
    fn non_qualified_i128() {
        assert_eq!(Value::I128(-1).non_qualified_string_representation(), "-1");
    }

    #[test]
    fn non_qualified_u32() {
        assert_eq!(Value::U32(100).non_qualified_string_representation(), "100");
    }

    #[test]
    fn non_qualified_i32() {
        assert_eq!(
            Value::I32(-100).non_qualified_string_representation(),
            "-100"
        );
    }

    #[test]
    fn non_qualified_u16() {
        assert_eq!(Value::U16(500).non_qualified_string_representation(), "500");
    }

    #[test]
    fn non_qualified_i16() {
        assert_eq!(
            Value::I16(-500).non_qualified_string_representation(),
            "-500"
        );
    }

    #[test]
    fn non_qualified_u8() {
        assert_eq!(Value::U8(7).non_qualified_string_representation(), "7");
    }

    #[test]
    fn non_qualified_i8() {
        assert_eq!(Value::I8(-7).non_qualified_string_representation(), "-7");
    }
}
