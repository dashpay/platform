// use serde::{Deserialize, Deserializer, Serialize, Serializer};
// use crate::{Error, Value};
// use ciborium::Value as CborValue;
//
// //todo: fix this
// impl Serialize for Value {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//         let cbor_value : CborValue = self.try_into()?;
//         cbor_value.serialize(serializer)
//     }
// }
//
// impl <'de> Deserialize<'de> for Value {
//     fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where D: Deserializer<'de> {
//         CborValue::deserialize(deserializer).into()
//     }
// }
