//! `#[serde(with = "address_output_map_optional_amount")]` helper.
//!
//! Reshapes `BTreeMap<PlatformAddress, Option<Credits>>` to/from an array of
//! `{ address, amount }` entries (where `amount` may be null) on the JSON /
//! Object wire.

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::serialization::json_safe_fields;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

#[json_safe_fields]
#[derive(Serialize, Deserialize)]
struct AddressOutputOptionalEntry {
    address: PlatformAddress,
    amount: Option<Credits>,
}

pub fn serialize<S>(
    map: &BTreeMap<PlatformAddress, Option<Credits>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(map.len()))?;
    for (address, amount) in map {
        seq.serialize_element(&AddressOutputOptionalEntry {
            address: *address,
            amount: *amount,
        })?;
    }
    seq.end()
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<PlatformAddress, Option<Credits>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(AddressOutputOptionalMapVisitor)
}

struct AddressOutputOptionalMapVisitor;

impl<'de> Visitor<'de> for AddressOutputOptionalMapVisitor {
    type Value = BTreeMap<PlatformAddress, Option<Credits>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of { address, amount } objects (amount may be null)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut map = BTreeMap::new();
        while let Some(entry) = seq.next_element::<AddressOutputOptionalEntry>()? {
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

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrapper(#[serde(with = "super")] BTreeMap<PlatformAddress, Option<Credits>>);

    #[test]
    fn round_trip_with_some_and_none_amounts() {
        let mut map = BTreeMap::new();
        map.insert(p2pkh(1), Some(500u64));
        map.insert(p2pkh(2), None);

        let original = Wrapper(map);
        let value = serde_json::to_value(&original).expect("serialize map");
        let arr = value.as_array().expect("emitted JSON array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["amount"], serde_json::json!(500));
        assert_eq!(arr[1]["amount"], serde_json::Value::Null);

        let restored: Wrapper = serde_json::from_value(value).expect("deserialize map");
        assert_eq!(original, restored);
    }

    #[test]
    fn rejects_duplicate_addresses() {
        let json = serde_json::json!([
            { "address": "00".to_string() + &"01".repeat(20), "amount": 1 },
            { "address": "00".to_string() + &"01".repeat(20), "amount": 2 }
        ]);
        let result: Result<Wrapper, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
