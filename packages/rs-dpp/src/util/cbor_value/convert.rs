use std::convert::TryInto;

use ciborium::value::Value as CborValue;

#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    ArrayInt,
    Bytes,
    StringBase64,
    StringBase58,
}

pub(super) fn convert_to(
    cbor_value: &CborValue,
    from: FieldType,
    to: FieldType,
) -> Option<CborValue> {
    let data_bytes = match from {
        FieldType::ArrayInt => {
            let array: Vec<u8> = cbor_value
                .as_array()?
                .iter()
                .map(|v| {
                    if let Some(int) = v.as_integer() {
                        let byte: Option<u8> = int.try_into().ok();
                        byte
                    } else {
                        None
                    }
                })
                .collect::<Option<_>>()?;
            array
        }
        FieldType::Bytes => cbor_value.as_bytes()?.to_owned(),

        FieldType::StringBase58 => {
            let text = cbor_value.as_text()?;
            bs58::decode(text.as_bytes()).into_vec().ok()?
        }

        FieldType::StringBase64 => {
            let text = cbor_value.as_text()?;
            base64::decode(text).ok()?
        }
    };

    let converted = match to {
        FieldType::ArrayInt => {
            unimplemented!("this use case has no use for dpp so far")
        }
        FieldType::Bytes => CborValue::Bytes(data_bytes),

        FieldType::StringBase58 => {
            let encoded = bs58::encode(data_bytes).into_string();
            CborValue::Text(encoded)
        }

        FieldType::StringBase64 => {
            let encoded = base64::encode(data_bytes);
            CborValue::Text(encoded)
        }
    };

    Some(converted)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn convert_array_to_bytes() {
        let cbor_value = CborValue::serialized(&vec![0_u8; 32]).expect("no error");

        let result =
            convert_to(&cbor_value, FieldType::ArrayInt, FieldType::Bytes).expect("no error");
        assert_eq!(CborValue::Bytes(vec![0_u8; 32]), result);
    }

    #[test]
    fn convert_array_to_base58() {
        let cbor_value = CborValue::serialized(&vec![0_u8; 32]).expect("no error");

        let result = convert_to(&cbor_value, FieldType::ArrayInt, FieldType::StringBase58)
            .expect("no error");
        assert_eq!(
            CborValue::Text(bs58::encode(vec![0_u8; 32]).into_string()),
            result
        );
    }

    #[test]
    fn convert_array_to_base64() {
        let cbor_value = CborValue::serialized(&vec![0_u8; 32]).expect("no error");

        let result = convert_to(&cbor_value, FieldType::ArrayInt, FieldType::StringBase64)
            .expect("no error");
        assert_eq!(CborValue::Text(base64::encode(vec![0_u8; 32])), result);
    }
}
