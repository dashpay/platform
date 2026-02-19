/// Custom serde serialization for BTreeMap<PlatformAddress, V>
/// Converts PlatformAddress keys to strings using Display when serializing to JSON
use crate::address_funds::PlatformAddress;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

pub fn serialize<S, V>(
    map: &BTreeMap<PlatformAddress, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    V: Serialize,
{
    let mut ser_map = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        // Convert PlatformAddress to string using Display
        ser_map.serialize_entry(&k.to_string(), v)?;
    }
    ser_map.end()
}

pub fn deserialize<'de, D, V>(deserializer: D) -> Result<BTreeMap<PlatformAddress, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct MapVisitor<V> {
        marker: PhantomData<V>,
    }

    impl<'de, V: Deserialize<'de>> Visitor<'de> for MapVisitor<V> {
        type Value = BTreeMap<PlatformAddress, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map with PlatformAddress keys")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut result = BTreeMap::new();
            while let Some((key_str, value)) = map.next_entry::<String, V>()? {
                let key = PlatformAddress::from_str(&key_str)
                    .map_err(|e| serde::de::Error::custom(format!("{}", e)))?;
                result.insert(key, value);
            }
            Ok(result)
        }
    }

    deserializer.deserialize_map(MapVisitor {
        marker: PhantomData,
    })
}


