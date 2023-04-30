use crate::document::{Document, DocumentV0};
use crate::prelude::{Revision, TimestampMillis};
use platform_value::{Identifier, Value};
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::BTreeMap;
use std::fmt;

impl Serialize for Document {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(None)?;

        match *self {
            Document::V0(ref v0) => {
                state.serialize_entry("$version", &0u16)?;
                state.serialize_entry("$id", &v0.id)?;
                state.serialize_entry("$ownerId", &v0.owner_id)?;
                state.serialize_entry("$revision", &v0.revision)?;
                state.serialize_entry("$createdAt", &v0.created_at)?;
                state.serialize_entry("$updatedAt", &v0.updated_at)?;
                for (key, value) in &v0.properties {
                    state.serialize_entry(key, value)?;
                }
            }
        }

        state.end()
    }
}

struct DocumentVisitor;

impl<'de> Visitor<'de> for DocumentVisitor {
    type Value = Document;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map representing a Document")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut version: Option<u16> = None;
        let mut id: Option<Identifier> = None;
        let mut owner_id: Option<Identifier> = None;
        let mut properties: BTreeMap<String, Value> = BTreeMap::new();
        let mut revision: Option<Revision> = None;
        let mut created_at: Option<TimestampMillis> = None;
        let mut updated_at: Option<TimestampMillis> = None;

        while let Some(key) = map.next_key()? {
            match key {
                "version" => {
                    version = Some(map.next_value()?);
                }
                "$id" => {
                    id = Some(map.next_value()?);
                }
                "$ownerId" => {
                    owner_id = Some(map.next_value()?);
                }
                "$revision" => {
                    revision = Some(map.next_value()?);
                }
                "$createdAt" => {
                    created_at = Some(map.next_value()?);
                }
                "$updatedAt" => {
                    updated_at = Some(map.next_value()?);
                }
                key => {
                    let value: Value = map.next_value()?;
                    properties.insert(key.to_string(), value);
                }
            }
        }

        let version = version.ok_or_else(|| serde::de::Error::missing_field("version"))?;
        let id = id.ok_or_else(|| serde::de::Error::missing_field("$id"))?;
        let owner_id = owner_id.ok_or_else(|| serde::de::Error::missing_field("$ownerId"))?;
        let revision = revision;
        let created_at = created_at;
        let updated_at = updated_at;

        match version {
            0 => Ok(Document::V0(DocumentV0 {
                id,
                owner_id,
                properties,
                revision,
                created_at,
                updated_at,
            })),
            _ => Err(serde::de::Error::unknown_variant(
                &format!("{}", version),
                &[],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Document {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DocumentVisitor)
    }
}
