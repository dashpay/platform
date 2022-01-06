use ciborium::value::{Integer, Value};
use grovedb::Error;
use std::borrow::Borrow;

enum DocumentFieldType {
    Integer,
    String,
    Float,
}

// What kind of error should be returned if something goes wrong
// THe value might not match the document field type is that corrupted data??
// Yeah corrupted data
// If the value match then encoding should work, so only one type of error really
fn encode_document_field_type(
    field_type: DocumentFieldType,
    value: &Value,
) -> Result<Vec<u8>, Error> {
    let field_type_match_error = Error::CorruptedData(String::from(
        "document field type doesn't match document value",
    ));

    return match field_type {
        DocumentFieldType::String => {
            let value_as_text = value.as_text().ok_or(field_type_match_error)?;
            Ok(value_as_text.as_bytes().to_vec())
        }
        DocumentFieldType::Integer => {
            let value_as_integer = value.as_integer().ok_or(field_type_match_error)?;
            let value_as_u64: u64 = value_as_integer.try_into().unwrap();
            Ok(value_as_u64.to_be_bytes().to_vec())
        }
        DocumentFieldType::Float => {
            let value_as_float = value.as_float().ok_or(field_type_match_error)?;
            Ok(value_as_float.to_be_bytes().to_vec())
        }
    };
}

mod tests {
    use crate::contract::types::{encode_document_field_type, DocumentFieldType};
    use ciborium::value::Value;

    #[test]
    fn test_successful_encode() {
        // TODO: Add more edge cases
        // Constraint: for all types, if a > b then encoding(a) > enconding(b)

        // String encoding
        let string1 = Value::Text(String::from("a"));
        let string2 = Value::Text(String::from("b"));

        let encoded_string1 = encode_document_field_type(DocumentFieldType::String, &string1)
            .expect("should encode: valid parameters");
        let encoded_string2 = encode_document_field_type(DocumentFieldType::String, &string2)
            .expect("should encode: valid parameters");

        assert_eq!(string1 > string2, encoded_string1 > encoded_string2);

        // Float encoding
        let float1 = Value::Float(11.0);
        let float2 = Value::Float(121.1);
        let float3 = Value::Float(13.0);

        let encoded_float1 = encode_document_field_type(DocumentFieldType::Float, &float1)
            .expect("should encode: valid parameters");
        let encoded_float2 = encode_document_field_type(DocumentFieldType::Float, &float2)
            .expect("should encode: valid parameters");
        let encoded_float3 = encode_document_field_type(DocumentFieldType::Float, &float3)
            .expect("should encode: valid parameters");

        // 11.0 < 121.1
        assert_eq!(encoded_float1 < encoded_float2, true);
        // 121.1 > 13.0
        assert_eq!(encoded_float2 > encoded_float3, true);
        // 13.0 > 11.0
        assert_eq!(encoded_float3 > encoded_float1, true);
    }
}
