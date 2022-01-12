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
}

pub fn string_to_field_type(field_type_name: String) -> Option<DocumentFieldType> {
    return match field_type_name.as_str() {
        "integer" => Some(DocumentFieldType::Integer),
        "string" => Some(DocumentFieldType::String),
        "float" => Some(DocumentFieldType::Float),
        "boolean" => Some(DocumentFieldType::Boolean),
        "date" => Some(DocumentFieldType::Date),
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
            let value_as_integer = value.as_integer().ok_or(field_type_match_error)?;
            let value_as_i64: i64 = value_as_integer
                .try_into()
                .map_err(|_| Error::CorruptedData(String::from("expected integer value")))?;

            encode_integer(value_as_i64)
        }
        DocumentFieldType::Float => {
            let value_as_float = value.as_float().ok_or(field_type_match_error)?;
            let value_as_f64 = value_as_float
                .try_into()
                .map_err(|_| Error::CorruptedData(String::from("expected float value")))?;
            encode_float(value_as_f64)
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
    };
}

fn encode_integer(val: i64) -> Result<Option<Vec<u8>>, Error> {
    // Positive integers are represented in binary with the signed bit set to 0
    // Negative integers are represented in 2's complement form

    // Encode the integer in big endian form
    // This ensures that most significant bits are compared first
    // a bigger positive number would be greater than a smaller one
    // and a bigger negative number would be greater than a smaller one
    // maintains sort order for each domain
    let mut wtr = vec![];
    wtr.write_i64::<BigEndian>(val).unwrap();

    // Flip the sign bit
    // to deal with interaction between the domains
    // 2's complement values have the sign bit set to 1
    // this makes them greater than the positive domain in terms of sort order
    // to fix this, we just flip the sign bit
    // so positive integers have the high bit and negative integers have the low bit
    // the relative order of elements in each domain is still maintained, as the
    // change was uniform across all elements
    wtr[0] ^= 0b1000_0000;

    Ok(Some(wtr))
}

fn encode_float(val: f64) -> Result<Option<Vec<u8>>, Error> {
    // Floats are represented based on the  IEEE 754-2008 standard
    // [sign bit] [biased exponent] [mantissa]
    // when comparing floats, the most impactful section is the sign bit
    // positive domain is greater then negative domain all other sections don't matter
    // next is the exponent (this determines the range of a number, numbers in the 100s range
    // are greater than numbers in the 10s range)
    // finally if two numbers are in the same range, then the mantissa would be the determining
    // factor.
    // The standard representation is already setup in this order.
    // Only the sign bit needs to be flipped to so positive domain is before the negative
    // domain.

    // Encode in big endian form, so most significant bits are compared first
    let mut wtr = vec![];
    wtr.write_f64::<BigEndian>(val).unwrap();

    // For positive numbers, the greater the exponent the bigger the number
    if val < 0.0 {
        // Flip all the bits
        wtr = wtr.iter().map(|byte| !byte).collect();
    } else {
        // Just flip the sign bit
        wtr[0] ^= 0b1000_0000;
    }

    Ok(Some(wtr))
}

mod tests {
    use crate::contract::types::{encode_document_field_type, DocumentFieldType};
    use ciborium::value::{Integer, Value};

    #[test]
    fn test_successful_encode() {
        // Constraint: for all types, if a > b then encoding(a) > enconding(b)
        let encode_err_msg = "should encode: valid parameters";

        // Integer encoding
        // Test approach
        // Test positive domain
        // Test negative domain
        // Test against 0
        // Test relationship between positive and negative domain

        // Show that the domain of positive integers maintains sort order after encoding
        let integer1 = Value::Integer(Integer::from(1));
        let integer2 = Value::Integer(Integer::from(600));
        let integer3 = Value::Integer(Integer::from(i64::MAX));

        let encoded_integer1 = encode_document_field_type(&DocumentFieldType::Integer, &integer1)
            .expect(encode_err_msg);
        let encoded_integer2 = encode_document_field_type(&DocumentFieldType::Integer, &integer2)
            .expect(encode_err_msg);
        let encoded_integer3 = encode_document_field_type(&DocumentFieldType::Integer, &integer3)
            .expect(encode_err_msg);

        assert_eq!(encoded_integer1 < encoded_integer2, true);
        assert_eq!(encoded_integer2 < encoded_integer3, true);

        // Show that the domain of negative integers maintain sort order after encoding
        let integer1 = Value::Integer(Integer::from(-1));
        let integer2 = Value::Integer(Integer::from(-600));
        let integer3 = Value::Integer(Integer::from(i64::MIN));

        let encoded_integer1 = encode_document_field_type(&DocumentFieldType::Integer, &integer1)
            .expect(encode_err_msg);
        let encoded_integer2 = encode_document_field_type(&DocumentFieldType::Integer, &integer2)
            .expect(encode_err_msg);
        let encoded_integer3 = encode_document_field_type(&DocumentFieldType::Integer, &integer3)
            .expect(encode_err_msg);

        assert_eq!(encoded_integer1 > encoded_integer2, true);
        assert_eq!(encoded_integer2 > encoded_integer3, true);

        // Show that zero is smack in the middle
        let integer1 = Value::Integer(Integer::from(-1));
        let integer2 = Value::Integer(Integer::from(0));
        let integer3 = Value::Integer(Integer::from(1));

        let encoded_integer1 = encode_document_field_type(&DocumentFieldType::Integer, &integer1)
            .expect(encode_err_msg);
        let encoded_integer2 = encode_document_field_type(&DocumentFieldType::Integer, &integer2)
            .expect(encode_err_msg);
        let encoded_integer3 = encode_document_field_type(&DocumentFieldType::Integer, &integer3)
            .expect(encode_err_msg);

        assert_eq!(encoded_integer2 > encoded_integer1, true);
        assert_eq!(encoded_integer2 < encoded_integer3, true);

        // Test the relationship between positive and negative integers
        // Since it has been shown that positive integers and negative integers maintain sort order
        // If the smallest positive number is greater than the largest negative number
        // then the positive domain is greater than the negative domain
        // Smallest positive integer is 1 and largest negative integer is -1
        assert_eq!(encoded_integer3 > encoded_integer1, true);

        // Float encoding
        // Test approach
        // Test positive domain
        // Test negative domain
        // Test against 0
        // Test relationship between positive and negative domain

        // Show that the domain of positive floats maintains sort order after encoding
        let float1 = Value::Float(1.0);
        let float2 = Value::Float(23.65);
        let float3 = Value::Float(1394.584);
        let float4 = Value::Float(f64::MAX);

        let encoded_float1 =
            encode_document_field_type(&DocumentFieldType::Float, &float1).expect(encode_err_msg);
        let encoded_float2 =
            encode_document_field_type(&DocumentFieldType::Float, &float2).expect(encode_err_msg);
        let encoded_float3 =
            encode_document_field_type(&DocumentFieldType::Float, &float3).expect(encode_err_msg);
        let encoded_float4 =
            encode_document_field_type(&DocumentFieldType::Float, &float4).expect(encode_err_msg);

        assert_eq!(encoded_float1 < encoded_float2, true);
        assert_eq!(encoded_float2 < encoded_float3, true);
        assert_eq!(encoded_float3 < encoded_float4, true);

        // Show that the domain of negative floats maintains sort order after encoding
        let float1 = Value::Float(-0.5);
        let float2 = Value::Float(-23.65);
        let float3 = Value::Float(-1394.584);
        let float4 = Value::Float(f64::MIN);

        let encoded_float1 =
            encode_document_field_type(&DocumentFieldType::Float, &float1).expect(encode_err_msg);
        let encoded_float2 =
            encode_document_field_type(&DocumentFieldType::Float, &float2).expect(encode_err_msg);
        let encoded_float3 =
            encode_document_field_type(&DocumentFieldType::Float, &float3).expect(encode_err_msg);
        let encoded_float4 =
            encode_document_field_type(&DocumentFieldType::Float, &float4).expect(encode_err_msg);

        assert_eq!(encoded_float1 > encoded_float2, true);
        assert_eq!(encoded_float2 > encoded_float3, true);
        assert_eq!(encoded_float3 > encoded_float4, true);

        // Show that 0 is in the middle
        let float1 = Value::Float(-1.0);
        let float2 = Value::Float(0.0);
        let float3 = Value::Float(1.0);

        let encoded_float1 =
            encode_document_field_type(&DocumentFieldType::Float, &float1).expect(encode_err_msg);
        let encoded_float2 =
            encode_document_field_type(&DocumentFieldType::Float, &float2).expect(encode_err_msg);
        let encoded_float3 =
            encode_document_field_type(&DocumentFieldType::Float, &float3).expect(encode_err_msg);

        assert_eq!(encoded_float1 < encoded_float2, true);
        assert_eq!(encoded_float2 < encoded_float3, true);

        // Test the relationship between positive and negative integers
        // Since it has been shown that positive integers and negative integers maintain sort order
        // If the smallest positive number is greater than the largest negative number
        // then the positive domain is greater than the negative domain
        let smallest_positive_float = Value::Float(0.0 + f64::EPSILON);
        let largest_negative_float = Value::Float(0.0 - f64::EPSILON);

        let encoded_float1 =
            encode_document_field_type(&DocumentFieldType::Float, &smallest_positive_float)
                .expect(encode_err_msg);
        let encoded_float2 =
            encode_document_field_type(&DocumentFieldType::Float, &largest_negative_float)
                .expect(encode_err_msg);

        assert_eq!(smallest_positive_float > largest_negative_float, true);
    }
}
