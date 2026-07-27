//! `#[serde(with = "address_output_map_required_amount")]` helper.
//!
//! Reshapes `BTreeMap<PlatformAddress, Credits>` to/from an array of
//! `{ address, amount }` entries on the JSON / Object wire. The `amount` field
//! is always required and never serializes as `null`.

use super::AddressOutputEntry;
use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::collections::BTreeMap;
use std::fmt;

pub fn serialize<S>(
    map: &BTreeMap<PlatformAddress, Credits>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(map.len()))?;
    for (address, amount) in map {
        seq.serialize_element(&AddressOutputEntry {
            address: *address,
            amount: *amount,
        })?;
    }
    seq.end()
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<PlatformAddress, Credits>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(AddressOutputMapVisitor)
}

struct AddressOutputMapVisitor;

impl<'de> Visitor<'de> for AddressOutputMapVisitor {
    type Value = BTreeMap<PlatformAddress, Credits>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of { address, amount } objects with required amount")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut map = BTreeMap::new();
        while let Some(entry) = seq.next_element::<AddressOutputEntry>()? {
            if map.insert(entry.address, entry.amount).is_some() {
                return Err(de::Error::custom(format!(
                    "duplicate output address: {}",
                    hex::encode(entry.address.to_bytes())
                )));
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrapper(#[serde(with = "super")] BTreeMap<PlatformAddress, Credits>);

    #[test]
    fn round_trip_required_amount() {
        let mut map = BTreeMap::new();
        map.insert(p2pkh(1), 100u64);
        map.insert(p2pkh(2), 200u64);

        let original = Wrapper(map);
        let value = serde_json::to_value(&original).expect("serialize map");
        let arr = value.as_array().expect("emitted JSON array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["amount"], serde_json::json!(100));
        assert_eq!(arr[1]["amount"], serde_json::json!(200));

        let restored: Wrapper = serde_json::from_value(value).expect("deserialize map");
        assert_eq!(original, restored);
    }

    #[test]
    fn rejects_missing_amount() {
        let json = serde_json::json!([
            { "address": "00".to_string() + &"01".repeat(20) }
        ]);
        let result: Result<Wrapper, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
