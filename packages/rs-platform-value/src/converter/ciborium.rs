use crate::{Error, Value};
use ciborium::value::Integer;
use ciborium::Value as CborValue;

impl Value {
    pub fn convert_from_cbor_map<I, R>(map: I) -> R
    where
        I: IntoIterator<Item = (String, CborValue)>,
        R: FromIterator<(String, Value)>,
    {
        map.into_iter()
            .map(|(key, cbor_value)| (key, cbor_value.into()))
            .collect()
    }

    pub fn convert_to_cbor_map<I, R>(map: I) -> Result<R, Error>
    where
        I: IntoIterator<Item = (String, Value)>,
        R: FromIterator<(String, CborValue)>,
    {
        map.into_iter()
            .map(|(key, value)| Ok((key, value.try_into()?)))
            .collect()
    }
}

impl From<CborValue> for Value {
    fn from(value: CborValue) -> Self {
        match value {
            CborValue::Integer(integer) => Self::I128(integer.into()),
            CborValue::Bytes(bytes) => Self::Bytes(bytes),
            CborValue::Float(float) => Self::Float(float),
            CborValue::Text(string) => Self::Text(string),
            CborValue::Bool(value) => Self::Bool(value),
            CborValue::Null => Self::Null,
            CborValue::Tag(int, value) => Self::Tag(int, value.into()),
            CborValue::Array(array) => {
                if !array.is_empty()
                    && array.iter().all(|v| {
                        let Some(int) = v.as_integer() else {
                        return false;
                    };
                        int.le(&Integer::from(u8::MAX)) && int.ge(&Integer::from(0))
                    })
                {
                    //this is an array of bytes
                    Self::Bytes(
                        array
                            .into_iter()
                            .map(|v| v.into_integer().unwrap().try_into().unwrap())
                            .collect(),
                    )
                } else {
                    Self::Array(array.into_iter().map(|v| v.into()).collect())
                }
            }
            CborValue::Map(map) => {
                Self::Map(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
            _ => panic!("unsupported"),
        }
    }
}

impl From<Box<CborValue>> for Box<Value> {
    fn from(value: Box<CborValue>) -> Self {
        value.into()
    }
}

impl TryInto<CborValue> for Value {
    type Error = Error;

    fn try_into(self) -> Result<CborValue, Self::Error> {
        Ok(match self {
            Value::U128(i) => CborValue::Integer((i as u64).into()),
            Value::I128(i) => CborValue::Integer((i as i64).into()),
            Value::U64(i) => CborValue::Integer(i.into()),
            Value::I64(i) => CborValue::Integer(i.into()),
            Value::U32(i) => CborValue::Integer(i.into()),
            Value::I32(i) => CborValue::Integer(i.into()),
            Value::U16(i) => CborValue::Integer(i.into()),
            Value::I16(i) => CborValue::Integer(i.into()),
            Value::U8(i) => CborValue::Integer(i.into()),
            Value::I8(i) => CborValue::Integer(i.into()),
            Value::Bytes(bytes) => CborValue::Bytes(bytes),
            Value::Float(float) => CborValue::Float(float),
            Value::Text(string) => CborValue::Text(string),
            Value::Bool(value) => CborValue::Bool(value),
            Value::Null => CborValue::Null,
            Value::Tag(i, v) => CborValue::Tag(i, v.try_into()?),
            Value::Array(array) => CborValue::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<CborValue>, Error>>()?,
            ),
            Value::Map(map) => CborValue::Map(
                map.into_iter()
                    .map(|(k, v)| Ok((k.try_into()?, v.try_into()?)))
                    .collect::<Result<Vec<(CborValue, CborValue)>, Error>>()?,
            ),
        })
    }
}

impl TryInto<Box<CborValue>> for Box<Value> {
    type Error = Error;
    fn try_into(self) -> Result<Box<CborValue>, Self::Error> {
        self.try_into()
    }
}
