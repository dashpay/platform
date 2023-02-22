use std::collections::BTreeMap;
use crate::Value;
use ciborium::Value as CborValue;

impl Value {
    pub fn convert_from_cbor_map<I, R>(map : I) -> R
    where I: IntoIterator<Item = (String, CborValue)>,
    R: FromIterator<(String, Value)>
    {
        map.into_iter().map(|(key, cbor_value)| {
            (key, cbor_value.into())
        }).collect()
    }
}

impl From<CborValue> for Value {
    fn from(value: CborValue) -> Self {
        match value {
            CborValue::Integer(integer) => { Self::I128(integer.into())}
            CborValue::Bytes(bytes) => Self::Bytes(bytes),
            CborValue::Float(float) => Self::Float(float),
            CborValue::Text(string) => Self::Text(string),
            CborValue::Bool(value) => Self::Bool(value),
            CborValue::Null => Self::Null,
            CborValue::Tag(int, value) => Self::Tag(int, value.into()),
            CborValue::Array(array) => {
                Self::Array(array.into_iter().map(|v| v.into()).collect())
            }
            CborValue::Map(map) => {
                Self::Map(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
            _ => panic!("unsupported")
        }
    }
}

impl From<Box<CborValue>> for Box<Value> {
    fn from(value: Box<CborValue>) -> Self {
        value.into()
    }
}