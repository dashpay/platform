//! `#[serde(with = "address_input_map")]` helper.
//!
//! Reshapes `BTreeMap<PlatformAddress, (AddressNonce, Credits)>` to/from an
//! array of `{ address, nonce, amount }` entries on the JSON / Object wire.

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::serialization::json_safe_fields;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

#[json_safe_fields]
#[derive(Serialize, Deserialize)]
struct AddressInputEntry {
    address: PlatformAddress,
    nonce: AddressNonce,
    amount: Credits,
}

pub fn serialize<S>(
    map: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(map.len()))?;
    for (address, (nonce, amount)) in map {
        seq.serialize_element(&AddressInputEntry {
            address: *address,
            nonce: *nonce,
            amount: *amount,
        })?;
    }
    seq.end()
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(AddressInputMapVisitor)
}

struct AddressInputMapVisitor;

impl<'de> Visitor<'de> for AddressInputMapVisitor {
    type Value = BTreeMap<PlatformAddress, (AddressNonce, Credits)>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of { address, nonce, amount } objects")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut map = BTreeMap::new();
        while let Some(entry) = seq.next_element::<AddressInputEntry>()? {
            if map
                .insert(entry.address, (entry.nonce, entry.amount))
                .is_some()
            {
                return Err(de::Error::custom(format!(
                    "duplicate input address: {}",
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

    /// Newtype wrapper so we can drive the helper through a serde derive.
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrapper(#[serde(with = "super")] BTreeMap<PlatformAddress, (AddressNonce, Credits)>);

    #[test]
    fn empty_round_trips() {
        let original = Wrapper(BTreeMap::new());
        let json = serde_json::to_string(&original).expect("serialize empty map");
        assert_eq!(json, "[]");
        let restored: Wrapper = serde_json::from_str(&json).expect("deserialize empty array");
        assert_eq!(original, restored);
    }

    #[test]
    fn single_entry_round_trips() {
        let mut map = BTreeMap::new();
        map.insert(p2pkh(1), (5u32, 1_000u64));
        let original = Wrapper(map);

        let value = serde_json::to_value(&original).expect("serialize single entry");
        assert_eq!(
            value,
            serde_json::json!([
                { "address": "00".to_string() + &"01".repeat(20), "nonce": 5, "amount": 1000 }
            ])
        );

        let restored: Wrapper = serde_json::from_value(value).expect("deserialize single entry");
        assert_eq!(original, restored);
    }

    #[test]
    fn multiple_entries_emit_in_sorted_address_order() {
        let mut map = BTreeMap::new();
        map.insert(p2pkh(2), (2u32, 200u64));
        map.insert(p2pkh(1), (1u32, 100u64));
        map.insert(p2pkh(3), (3u32, 300u64));

        let original = Wrapper(map);
        let value = serde_json::to_value(&original).expect("serialize multi entry");
        let arr = value.as_array().expect("emitted JSON array");
        let nonces: Vec<u64> = arr
            .iter()
            .map(|entry| entry["nonce"].as_u64().expect("nonce as u64"))
            .collect();
        assert_eq!(nonces, vec![1, 2, 3]);

        let restored: Wrapper = serde_json::from_value(value).expect("deserialize multi entry");
        assert_eq!(original, restored);
    }

    #[test]
    fn rejects_duplicate_addresses() {
        let json = serde_json::json!([
            { "address": "00".to_string() + &"01".repeat(20), "nonce": 1, "amount": 100 },
            { "address": "00".to_string() + &"01".repeat(20), "nonce": 2, "amount": 200 }
        ]);
        let result: Result<Wrapper, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
