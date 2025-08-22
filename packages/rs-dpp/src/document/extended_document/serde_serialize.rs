use crate::data_contract::DataContract;
use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::{Document, ExtendedDocument};
use platform_value::{Bytes32, Identifier};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

impl Serialize for ExtendedDocument {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(None)?;

        match *self {
            ExtendedDocument::V0(ref v0) => {
                state.serialize_entry("version", &0u16)?;
                state.serialize_entry("$type", &v0.document_type_name)?;
                state.serialize_entry("$dataContractId", &v0.data_contract_id)?;
                state.serialize_entry("document", &v0.document)?;
            }
        }

        state.end()
    }
}

struct ExtendedDocumentVisitor;

impl<'de> Visitor<'de> for ExtendedDocumentVisitor {
    type Value = ExtendedDocument;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map representing an ExtendedDocument")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut version: Option<u16> = None;
        let mut document_type_name: Option<String> = None;
        let mut data_contract_id: Option<Identifier> = None;
        let mut document: Option<Document> = None;
        let data_contract: Option<DataContract> = None;

        while let Some(key) = map.next_key()? {
            match key {
                "$version" => {
                    version = Some(map.next_value()?);
                }
                "$type" => {
                    document_type_name = Some(map.next_value()?);
                }
                "$dataContractId" => {
                    data_contract_id = Some(map.next_value()?);
                }
                "document" => {
                    document = Some(map.next_value()?);
                }
                _ => {}
            }
        }

        let version = version.ok_or_else(|| serde::de::Error::missing_field("$version"))?;
        let document_type_name =
            document_type_name.ok_or_else(|| serde::de::Error::missing_field("$type"))?;
        let data_contract_id =
            data_contract_id.ok_or_else(|| serde::de::Error::missing_field("$dataContractId"))?;
        let data_contract =
            data_contract.ok_or_else(|| serde::de::Error::missing_field("$dataContract"))?;
        let document = document.ok_or_else(|| serde::de::Error::missing_field("document"))?;

        match version {
            0 => Ok(ExtendedDocument::V0(ExtendedDocumentV0 {
                document_type_name,
                data_contract_id,
                document,
                data_contract,
                metadata: None,
                entropy: Bytes32::default(),
                token_payment_info: None,
            })),
            _ => Err(serde::de::Error::unknown_variant(
                &format!("{}", version),
                &[],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for ExtendedDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ExtendedDocumentVisitor)
    }
}
