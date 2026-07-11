pub mod v0;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::Document;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::serialization::{PlatformDeserializable, PlatformSerializable};
use crate::voting::contender_structs::contender::v0::ContenderV0;
use crate::voting::contender_structs::ContenderWithSerializedDocumentV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

/// Represents a contender in the contested document vote poll.
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Clone, From)]
pub enum Contender {
    /// V0
    V0(ContenderV0),
}

/// Represents a contender in the contested document vote poll.
/// This is for internal use where the document is in serialized form
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug, PartialEq, Eq, Clone, From, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)]
pub enum ContenderWithSerializedDocument {
    /// V0
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ContenderWithSerializedDocumentV0),
}

impl Contender {
    pub fn identity_id(&self) -> Identifier {
        match self {
            Contender::V0(v0) => v0.identity_id,
        }
    }

    pub fn identity_id_ref(&self) -> &Identifier {
        match self {
            Contender::V0(v0) => &v0.identity_id,
        }
    }

    pub fn document(&self) -> &Option<Document> {
        match self {
            Contender::V0(v0) => &v0.document,
        }
    }

    pub fn take_document(&mut self) -> Option<Document> {
        match self {
            Contender::V0(v0) => v0.document.take(),
        }
    }

    pub fn vote_tally(&self) -> Option<u32> {
        match self {
            Contender::V0(v0) => v0.vote_tally,
        }
    }
}

impl ContenderWithSerializedDocument {
    pub fn identity_id(&self) -> Identifier {
        match self {
            ContenderWithSerializedDocument::V0(v0) => v0.identity_id,
        }
    }

    pub fn identity_id_ref(&self) -> &Identifier {
        match self {
            ContenderWithSerializedDocument::V0(v0) => &v0.identity_id,
        }
    }

    pub fn serialized_document(&self) -> &Option<Vec<u8>> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => &v0.serialized_document,
        }
    }

    pub fn take_serialized_document(&mut self) -> Option<Vec<u8>> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => v0.serialized_document.take(),
        }
    }

    pub fn vote_tally(&self) -> Option<u32> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => v0.vote_tally,
        }
    }
}

impl ContenderWithSerializedDocument {
    pub fn try_into_contender(
        self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Contender, ProtocolError> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => Ok(v0
                .try_into_contender(document_type_ref, platform_version)?
                .into()),
        }
    }

    pub fn try_to_contender(
        &self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Contender, ProtocolError> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => Ok(v0
                .try_to_contender(document_type_ref, platform_version)?
                .into()),
        }
    }
}

impl Contender {
    pub fn try_into_contender_with_serialized_document(
        self,
        document_type_ref: DocumentTypeRef,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderWithSerializedDocument, ProtocolError> {
        match self {
            Contender::V0(v0) => Ok(v0
                .try_into_contender_with_serialized_document(
                    document_type_ref,
                    data_contract,
                    platform_version,
                )?
                .into()),
        }
    }

    pub fn try_to_contender_with_serialized_document(
        &self,
        document_type_ref: DocumentTypeRef,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderWithSerializedDocument, ProtocolError> {
        match self {
            Contender::V0(v0) => Ok(v0
                .try_to_contender_with_serialized_document(
                    document_type_ref,
                    data_contract,
                    platform_version,
                )?
                .into()),
        }
    }

    pub fn serialize(
        &self,
        document_type: DocumentTypeRef,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.try_to_contender_with_serialized_document(
            document_type,
            data_contract,
            platform_version,
        )?
        .serialize_to_bytes()
    }

    pub fn serialize_consume(
        self,
        document_type: DocumentTypeRef,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.try_into_contender_with_serialized_document(
            document_type,
            data_contract,
            platform_version,
        )?
        .serialize_to_bytes()
    }

    pub fn from_bytes(
        serialized_contender: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let serialized_contender =
            ContenderWithSerializedDocument::deserialize_from_bytes(serialized_contender)?;
        serialized_contender.try_into_contender(document_type, platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voting::contender_structs::contender::v0::{
        ContenderV0, ContenderWithSerializedDocumentV0,
    };
    use platform_value::Identifier;

    mod contender_construction {
        use super::*;

        #[test]
        fn contender_v0_default() {
            let contender = ContenderV0::default();
            assert_eq!(contender.identity_id, Identifier::default());
            assert!(contender.document.is_none());
            assert!(contender.vote_tally.is_none());
        }

        #[test]
        fn contender_v0_with_fields() {
            let id = Identifier::new([1u8; 32]);
            let contender = ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: Some(42),
            };
            assert_eq!(contender.identity_id, id);
            assert!(contender.document.is_none());
            assert_eq!(contender.vote_tally, Some(42));
        }

        #[test]
        fn contender_from_v0() {
            let id = Identifier::new([2u8; 32]);
            let v0 = ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: Some(100),
            };
            let contender: Contender = v0.into();
            assert_eq!(contender.identity_id(), id);
            assert_eq!(contender.vote_tally(), Some(100));
        }
    }

    mod contender_accessors {
        use super::*;

        #[test]
        fn identity_id_returns_correct_value() {
            let id = Identifier::new([3u8; 32]);
            let contender = Contender::V0(ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: None,
            });
            assert_eq!(contender.identity_id(), id);
        }

        #[test]
        fn identity_id_ref_returns_reference() {
            let id = Identifier::new([4u8; 32]);
            let contender = Contender::V0(ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: None,
            });
            assert_eq!(*contender.identity_id_ref(), id);
        }

        #[test]
        fn document_returns_none_when_empty() {
            let contender = Contender::V0(ContenderV0::default());
            assert!(contender.document().is_none());
        }

        #[test]
        fn vote_tally_returns_none_when_not_set() {
            let contender = Contender::V0(ContenderV0::default());
            assert!(contender.vote_tally().is_none());
        }

        #[test]
        fn vote_tally_returns_value_when_set() {
            let contender = Contender::V0(ContenderV0 {
                identity_id: Identifier::default(),
                document: None,
                vote_tally: Some(999),
            });
            assert_eq!(contender.vote_tally(), Some(999));
        }

        #[test]
        fn take_document_returns_none_and_leaves_none() {
            let mut contender = Contender::V0(ContenderV0::default());
            let doc = contender.take_document();
            assert!(doc.is_none());
            assert!(contender.document().is_none());
        }
    }

    mod contender_with_serialized_document {
        use super::*;

        #[test]
        fn default_values() {
            let csd = ContenderWithSerializedDocumentV0::default();
            assert_eq!(csd.identity_id, Identifier::default());
            assert!(csd.serialized_document.is_none());
            assert!(csd.vote_tally.is_none());
        }

        #[test]
        fn construction_with_data() {
            let id = Identifier::new([5u8; 32]);
            let doc_bytes = vec![1, 2, 3, 4, 5];
            let csd = ContenderWithSerializedDocumentV0 {
                identity_id: id,
                serialized_document: Some(doc_bytes.clone()),
                vote_tally: Some(50),
            };
            let wrapped = ContenderWithSerializedDocument::V0(csd);
            assert_eq!(wrapped.identity_id(), id);
            assert_eq!(*wrapped.identity_id_ref(), id);
            assert_eq!(wrapped.serialized_document(), &Some(doc_bytes));
            assert_eq!(wrapped.vote_tally(), Some(50));
        }

        #[test]
        fn take_serialized_document() {
            let doc_bytes = vec![10, 20, 30];
            let csd = ContenderWithSerializedDocumentV0 {
                identity_id: Identifier::default(),
                serialized_document: Some(doc_bytes.clone()),
                vote_tally: None,
            };
            let mut wrapped = ContenderWithSerializedDocument::V0(csd);
            let taken = wrapped.take_serialized_document();
            assert_eq!(taken, Some(doc_bytes));
            assert!(wrapped.serialized_document().is_none());
        }

        #[test]
        fn serialization_round_trip() {
            let id = Identifier::new([6u8; 32]);
            let csd = ContenderWithSerializedDocumentV0 {
                identity_id: id,
                serialized_document: Some(vec![0xAA, 0xBB, 0xCC]),
                vote_tally: Some(77),
            };
            let wrapped = ContenderWithSerializedDocument::V0(csd);

            // Serialize to bytes using PlatformSerializable
            let bytes = wrapped
                .serialize_to_bytes()
                .expect("should serialize to bytes");
            assert!(!bytes.is_empty());

            // Deserialize back
            let restored = ContenderWithSerializedDocument::deserialize_from_bytes(&bytes)
                .expect("should deserialize from bytes");

            assert_eq!(wrapped, restored);
        }

        #[test]
        fn serialization_round_trip_with_no_document() {
            let id = Identifier::new([7u8; 32]);
            let csd = ContenderWithSerializedDocumentV0 {
                identity_id: id,
                serialized_document: None,
                vote_tally: None,
            };
            let wrapped = ContenderWithSerializedDocument::V0(csd);

            let bytes = wrapped
                .serialize_to_bytes()
                .expect("should serialize to bytes");
            let restored = ContenderWithSerializedDocument::deserialize_from_bytes(&bytes)
                .expect("should deserialize from bytes");

            assert_eq!(wrapped, restored);
        }
    }

    mod equality {
        use super::*;

        #[test]
        fn equal_contenders() {
            let id = Identifier::new([8u8; 32]);
            let a = Contender::V0(ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: Some(10),
            });
            let b = Contender::V0(ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: Some(10),
            });
            assert_eq!(a, b);
        }

        #[test]
        fn different_vote_tallies_not_equal() {
            let id = Identifier::new([9u8; 32]);
            let a = Contender::V0(ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: Some(10),
            });
            let b = Contender::V0(ContenderV0 {
                identity_id: id,
                document: None,
                vote_tally: Some(20),
            });
            assert_ne!(a, b);
        }

        #[test]
        fn different_identity_ids_not_equal() {
            let a = Contender::V0(ContenderV0 {
                identity_id: Identifier::new([1u8; 32]),
                document: None,
                vote_tally: None,
            });
            let b = Contender::V0(ContenderV0 {
                identity_id: Identifier::new([2u8; 32]),
                document: None,
                vote_tally: None,
            });
            assert_ne!(a, b);
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_contender_with_serialized_document {
    use super::*;
    use platform_value::{platform_value, Identifier, Value};
    use serde_json::json;

    /// Non-default values per field (real identity_id bytes, non-empty
    /// serialized_document, non-zero tally) so the wire-shape assertion
    /// catches silent zero-out / flip on round-trip.
    fn fixture() -> ContenderWithSerializedDocument {
        ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: Identifier::new([0xa1; 32]),
            serialized_document: Some(vec![0xde, 0xad, 0xbe, 0xef]),
            vote_tally: Some(42),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `Identifier` renders as base58 in JSON. `Vec<u8>` (from the default
        // `serialize_seq` in serde_json) renders as an array of numbers — NOT
        // bytes — so each element appears as `Number(...)`. `vote_tally` is
        // `Option<u32>`; JSON erases the size — the value-path assertion uses
        // `42u32` to lock in `Value::U32`.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": "Bswb3UyeD1pUTaGiE6WvqwFpJZsQSEY1xhJePCDTHdvp",
                "serializedDocument": [222, 173, 190, 239],
                "voteTally": 42,
            })
        );
        let recovered = ContenderWithSerializedDocument::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed variants: `Identifier` renders as
        // `Value::Identifier`, `Vec<u8>` renders as `Value::Array([U8(...), ...])`
        // (NOT `Value::Bytes`, because `Vec<u8>` uses the generic `serialize_seq`
        // path), `Option<u32>` becomes `Value::U32`.
        let id = Identifier::new([0xa1; 32]);
        let bytes_array = Value::Array(vec![
            Value::U8(0xde),
            Value::U8(0xad),
            Value::U8(0xbe),
            Value::U8(0xef),
        ]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": id,
                "serializedDocument": bytes_array,
                "voteTally": 42u32,
            })
        );
        let recovered = ContenderWithSerializedDocument::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
