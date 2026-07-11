//! `#[serde(with = "address_output_singular")]` helper.
//!
//! Reshapes `Option<(PlatformAddress, Credits)>` to/from a single
//! `{ address, amount }` object (or `null`) on the JSON / Object wire.

use super::AddressOutputEntry;
use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(
    value: &Option<(PlatformAddress, Credits)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some((address, amount)) => serializer.serialize_some(&AddressOutputEntry {
            address: *address,
            amount: *amount,
        }),
        None => serializer.serialize_none(),
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<(PlatformAddress, Credits)>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<AddressOutputEntry> = Option::deserialize(deserializer)?;
    Ok(opt.map(|entry| (entry.address, entry.amount)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrapper(#[serde(with = "super")] Option<(PlatformAddress, Credits)>);

    #[test]
    fn none_serializes_to_null() {
        let original = Wrapper(None);
        let value = serde_json::to_value(&original).expect("serialize None");
        assert_eq!(value, serde_json::Value::Null);

        let restored: Wrapper = serde_json::from_value(value).expect("deserialize null");
        assert_eq!(original, restored);
    }

    #[test]
    fn some_round_trips_as_object() {
        let original = Wrapper(Some((p2pkh(7), 42u64)));
        let value = serde_json::to_value(&original).expect("serialize Some");
        assert_eq!(value["amount"], serde_json::json!(42));
        let address_hex = value["address"].as_str().expect("address as hex string");
        assert!(address_hex.starts_with("00")); // P2pkh discriminant
        assert_eq!(address_hex.len(), 42); // 21 bytes hex-encoded

        let restored: Wrapper = serde_json::from_value(value).expect("deserialize object");
        assert_eq!(original, restored);
    }
}
