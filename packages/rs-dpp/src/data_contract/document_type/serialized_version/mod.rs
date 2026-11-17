//! The stored form of a document type: what parsing its schema decided, so that a contract
//! stored in format version 2 is read back without parsing its schemas again, and keeps the
//! meaning it was registered with whatever parser a later protocol version brings.
//!
//! The form carries its own version tag: the variant decides how it is read back, whatever
//! version reads it. Which generation is written is the protocol version's choice
//! (`contract_storage_document_type_version`). Generation 0 is in `v0`.

pub mod v0;

use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::DefinitionName;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserializeUntrusted, PlatformSerialize};
use platform_value::string_encoding::{decode, encode, Encoding};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
pub use v0::DocumentTypeInSerializationFormatV0;

/// A document type, compiled. Serde reads and writes it as its bincode bytes (base64 in a
/// human-readable format): it is a stored form, which only consensus writes, not a shape
/// clients edit. Its bytes are those of the platform bincode configuration, decoded with the
/// untrusted decoder, nothing left over ([`PlatformSerializable`],
/// [`PlatformDeserializableUntrusted`]).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    DecodeUntrusted,
    From,
    PlatformSerialize,
    PlatformDeserializeUntrusted,
)]
#[platform_serialize(unversioned)]
pub enum DocumentTypeInSerializationFormat {
    V0(DocumentTypeInSerializationFormatV0),
}

impl DocumentTypeInSerializationFormat {
    /// `document_type`, of a contract whose `$defs` are `schema_defs`, compiled in the
    /// generation of the stored form `platform_version` selects
    /// (`contract_storage_document_type_version`), with what its documents are checked against:
    /// the typed validation schema it was read back with, or the one the builder
    /// `platform_version` selects builds.
    pub fn try_from_document_type(
        document_type: DocumentTypeRef,
        schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_storage_document_type_version
        {
            0 => Ok(DocumentTypeInSerializationFormatV0::try_from_document_type(
                document_type,
                schema_defs,
                platform_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentTypeInSerializationFormat::try_from_document_type".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Whether this stored form is what `document_type`, of a contract whose `$defs` are
    /// `schema_defs`, compiles to at `platform_version` in this form's own generation, what its
    /// documents are checked against included.
    pub(crate) fn is_compiled_from(
        &self,
        document_type: DocumentTypeRef,
        schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match self {
            DocumentTypeInSerializationFormat::V0(v0) => {
                v0.is_compiled_from(document_type, schema_defs, platform_version)
            }
        }
    }

    /// The document type `name` of the contract `data_contract_id`, read back from this stored
    /// form, with `schema`, the schema it was compiled from.
    pub fn into_document_type(
        self,
        name: &str,
        data_contract_id: Identifier,
        schema: Value,
    ) -> Result<DocumentType, ProtocolError> {
        match self {
            DocumentTypeInSerializationFormat::V0(v0) => {
                v0.into_document_type(name, data_contract_id, schema)
            }
        }
    }
}

impl Serialize for DocumentTypeInSerializationFormat {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = self
            .serialize_to_bytes()
            .map_err(serde::ser::Error::custom)?;
        if serializer.is_human_readable() {
            serializer.serialize_str(&encode(&bytes, Encoding::Base64))
        } else {
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> Deserialize<'de> for DocumentTypeInSerializationFormat {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("the bytes of a stored document type")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                decode(value, Encoding::Base64).map_err(E::custom)
            }

            fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
                Ok(value.to_vec())
            }

            fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                Ok(value)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(4096));
                while let Some(byte) = seq.next_element::<u8>()? {
                    bytes.push(byte);
                }
                Ok(bytes)
            }
        }

        // Every format reaching here describes itself (JSON, platform values, CBOR, JavaScript
        // objects), and a converted value may hold the bytes in any shape: base64 text, as JSON
        // carries them, bytes, or a list of them
        let bytes = deserializer.deserialize_any(BytesVisitor)?;
        Self::deserialize_from_bytes_untrusted_exact(&bytes).map_err(D::Error::custom)
    }
}

/// Serialized as a byte string, so no integer of it reaches JSON as a number.
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for DocumentTypeInSerializationFormat {}
