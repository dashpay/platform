use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use ciborium::value::{Integer, Value};
use grovedb::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum DocumentFieldType {
    Integer,
    String,
    Float,
    ByteArray,
    Boolean,
    Date,
    Object,
}

pub fn string_to_field_type(field_type_name: String) -> Option<DocumentFieldType> {
    return match field_type_name.as_str() {
        "integer" => Some(DocumentFieldType::Integer),
        "string" => Some(DocumentFieldType::String),
        "float" => Some(DocumentFieldType::Float),
        "boolean" => Some(DocumentFieldType::Boolean),
        "object" => Some(DocumentFieldType::Object),
        _ => None,
    };
}

// Given a field type and a value this function chooses and executes the right encoding method
pub fn encode_document_field_type(
    field_type: &DocumentFieldType,
    value: &Value,
) -> Result<Option<Vec<u8>>, Error> {
    let field_type_match_error = Error::CorruptedData(String::from(
        "document field type doesn't match document value",
    ));

    return match field_type {
        DocumentFieldType::String => {
            let value_as_text = value.as_text().ok_or(field_type_match_error)?;
            Ok(Some(value_as_text.as_bytes().to_vec()))
        }
        DocumentFieldType::Integer => {
            // Direct integer to byte encoding doesn't take into account the signed bit
            // for negative and positive integers
            let value_as_integer = value.as_integer().ok_or(field_type_match_error)?;
            let value_as_i64: i64 = value_as_integer
                .try_into()
                .map_err(|_| Error::CorruptedData(String::from("expected integer value")))?;

            encode_integer(value_as_i64)
        }
        DocumentFieldType::Float => {
            let value_as_float = value.as_float().ok_or(field_type_match_error)?;
            Ok(Some(value_as_float.to_be_bytes().to_vec()))
        }
        DocumentFieldType::ByteArray => {
            // Byte array could either be raw bytes or encoded as a base64 string
            if value.is_text() {
                // Decode base64 string
                let base64_value = value.as_text().expect("confirmed as text");
                let value_as_bytes = base64::decode(base64_value).map_err(|_| {
                    Error::CorruptedData(String::from("bytearray: invalid base64 value"))
                })?;
                Ok(Some(value_as_bytes))
            } else {
                let value_as_bytes = value.as_bytes().ok_or(field_type_match_error)?;
                Ok(Some(value_as_bytes.clone()))
            }
        }
        DocumentFieldType::Boolean => {
            let value_as_boolean = value.as_bool().ok_or(field_type_match_error)?;
            if value_as_boolean == true {
                Ok(Some(vec![1]))
            } else {
                Ok(Some(vec![0]))
            }
        }
        DocumentFieldType::Date => {
            let date_string = value.as_text().ok_or(field_type_match_error)?;
            let date_as_integer: i64 = date_string
                .parse()
                .map_err(|_| Error::CorruptedData(String::from("invalid integer string")))?;
            encode_integer(date_as_integer)
        }
        DocumentFieldType::Object => Ok(Some(Vec::new())),
    };
}

fn encode_integer(val: i64) -> Result<Option<Vec<u8>>, Error> {
    let mut wtr = vec![];
    wtr.write_i64::<BigEndian>(val).unwrap();

    // Flip the sign bit
    wtr[0] = wtr[0] ^ 0b1000_0000;

    Ok(Some(wtr))
}

mod tests {
    use crate::contract::types::{encode_document_field_type, DocumentFieldType};
    use ciborium::value::{Integer, Value};

    #[test]
    fn test_successful_encode() {
        // TODO: Add more edge cases
        // Constraint: for all types, if a > b then encoding(a) > enconding(b)

        // String encoding
        let string1 = Value::Text(String::from("a"));
        let string2 = Value::Text(String::from("b"));

        let encoded_string1 = encode_document_field_type(&DocumentFieldType::String, &string1)
            .expect("should encode: valid parameters");
        let encoded_string2 = encode_document_field_type(&DocumentFieldType::String, &string2)
            .expect("should encode: valid parameters");

        assert_eq!(string1 > string2, encoded_string1 > encoded_string2);

        // Float encoding
        let float1 = Value::Float(11.0);
        let float2 = Value::Float(121.1);
        let float3 = Value::Float(13.0);

        let encoded_float1 = encode_document_field_type(&DocumentFieldType::Float, &float1)
            .expect("should encode: valid parameters");
        let encoded_float2 = encode_document_field_type(&DocumentFieldType::Float, &float2)
            .expect("should encode: valid parameters");
        let encoded_float3 = encode_document_field_type(&DocumentFieldType::Float, &float3)
            .expect("should encode: valid parameters");

        // 11.0 < 121.1
        assert_eq!(encoded_float1 < encoded_float2, true);
        // 121.1 > 13.0
        assert_eq!(encoded_float2 > encoded_float3, true);
        // 13.0 > 11.0
        assert_eq!(encoded_float3 > encoded_float1, true);

        // Integer encoding
        let integer1 = Value::Integer(Integer::from(1));
        let integer2 = Value::Integer(Integer::from(600));
        let integer3 = Value::Integer(Integer::from(60));
        let integer4 = Value::Integer(Integer::from(-10));

        let encoded_integer1 = encode_document_field_type(&DocumentFieldType::Integer, &integer1)
            .expect("should encode: valid parameters");
        let encoded_integer2 = encode_document_field_type(&DocumentFieldType::Integer, &integer2)
            .expect("should encode: valid parameters");
        let encoded_integer3 = encode_document_field_type(&DocumentFieldType::Integer, &integer3)
            .expect("should encode: valid parameters");
        let encoded_integer4 = encode_document_field_type(&DocumentFieldType::Integer, &integer4)
            .expect("should encode: valid parameters");

        dbg!(&encoded_integer2);
        dbg!(&encoded_integer4);

        assert_eq!(encoded_integer1 < encoded_integer2, true);
        assert_eq!(encoded_integer2 > encoded_integer4, true);
    }
}
