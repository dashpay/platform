pub(in crate::document) mod deserialize;
pub(in crate::document) mod serialize;
mod v0;

use crate::data_contract::document_type::{DecodedValueBudget, DocumentTypeRef};
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0};
#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::{FeatureVersion, PlatformVersion};
pub use v0::*;

impl Document {
    /// Reads only the document `$id` from the header of a serialized document
    /// (formats 0 through 3) without decoding any document properties.
    pub fn id_from_bytes(
        serialized_document: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => DocumentV0::id_from_bytes(serialized_document),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::id_from_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Reads a serialized document while enforcing `budget` on decoded property
    /// values. Client-side proof verifiers share one budget across every
    /// document in a page so zero-width typed-array elements on
    /// protocol-version-14 contracts cannot expand a small response into
    /// unbounded memory.
    pub fn from_bytes_with_budget(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        budget: &mut DecodedValueBudget,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0::from_bytes_with_budget(
                serialized_document,
                document_type,
                budget,
                platform_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl DocumentPlatformConversionMethodsV0 for Document {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize(
        &self,
        document_type: DocumentTypeRef,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                document_v0.serialize(document_type, data_contract, platform_version)
            }
        }
    }

    fn serialize_specific_version(
        &self,
        document_type: DocumentTypeRef,
        data_contract: &DataContract,
        feature_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => document_v0.serialize_specific_version(
                document_type,
                data_contract,
                feature_version,
            ),
        }
    }

    /// Reads a serialized document and creates a Document from it.
    fn from_bytes(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(
                DocumentV0::from_bytes(serialized_document, document_type, platform_version)?
                    .into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "validation")]
    fn from_bytes_in_consensus(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError>
    where
        Self: Sized,
    {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0::from_bytes_in_consensus(
                serialized_document,
                document_type,
                platform_version,
            )?
            .map(|document_v0| document_v0.into())),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes_in_consensus".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::Document;
    use crate::tests::json_document::json_document_to_contract;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_serialization() {
        let platform_version = PlatformVersion::first();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get dashpay contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get profile document type");
        let document = document_type
            .random_document(Some(3333), platform_version)
            .expect("expected to get a random document");

        let serialized_document = document
            .serialize(document_type, &contract, platform_version)
            .expect("expected to serialize");

        let deserialized_document = Document::from_bytes(
            serialized_document.as_slice(),
            document_type,
            platform_version,
        )
        .expect("expected to deserialize a document");
        assert_eq!(document, deserialized_document);
        for _i in 0..10000 {
            let document = document_type
                .random_document(Some(3333), platform_version)
                .expect("expected to get a random document");
            document
                .serialize(document_type, &contract, platform_version)
                .expect("expected to serialize consume");
        }
    }

    #[test]
    fn test_withdrawal_deserialization() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/withdrawals/withdrawals-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get withdrawals contract");

        //  Header (65 bytes)
        //
        //   - 01 - Document Version (1 byte): Value = 1
        //   - 0053626cafc76f47062f936c5938190f5f30aac997b8fc22e81c1d9a7f903bd9 - Document ID (32 bytes)
        //   - fa8696d3f39c518784e53be79ee199e70387f9a7408254de920c1f3779de2856 - Owner ID (32 bytes)
        //
        //   Metadata (19 bytes)
        //
        //   - 01 - Revision (1 byte): Value = 1
        //   - 0003 - Bitwise flags (2 bytes): Binary 0000000000000011
        //     - Bit 0 set: createdAt present
        //     - Bit 1 set: updatedAt present
        //   - 0000019782b96d14 - createdAt timestamp (8 bytes): 1750244879636
        //   - 0000019782b96d14 - updatedAt timestamp (8 bytes): 1750244879636
        //
        //   User Properties (42 bytes)
        //
        //   - 00 - transactionIndex marker (1 byte): 0 = absent
        //   - 00 - transactionSignHeight marker (1 byte): 0 = absent
        //   - 00000002540be400 - amount (8 bytes): 10000000000 duffs (100 DASH)
        //   - 00000001 - coreFeePerByte (4 bytes): 1 duff/byte
        //   - 00 - pooling (1 byte): 0 (Never pool)
        //   - 19 - outputScript length (1 byte varint): 25 bytes
        //   - 76a9149e3292d2612122d81613fdb893dd36a04df3355588ac - outputScript data (25 bytes)
        //     - This is a standard Bitcoin P2PKH script:
        //         - 76 = OP_DUP
        //       - a9 = OP_HASH160
        //       - 14 = Push 20 bytes
        //       - 9e3292d2612122d81613fdb893dd36a04df33555 = recipient's pubkey hash
        //       - 88 = OP_EQUALVERIFY
        //       - ac = OP_CHECKSIG
        //   - 00 - status (1 byte): 0 (QUEUED)

        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("expected to get profile document type");
        let serialized_document = hex::decode("010053626cafc76f47062f936c5938190f5f30aac997b8fc22e81c1d9a7f903bd9fa8696d3f39c518784e53be79ee199e70387f9a7408254de920c1f3779de28560100030000019782b96d140000019782b96d14000000000002540be40000000001001976a9149e3292d2612122d81613fdb893dd36a04df3355588ac00").expect("expected document hex bytes");

        let _deserialized_document = Document::from_bytes(
            serialized_document.as_slice(),
            document_type,
            platform_version,
        )
        .expect("expected to deserialize a document");
    }

    #[test]
    fn should_enforce_a_shared_decoded_value_budget_across_documents_while_keeping_from_bytes_unbounded(
    ) {
        use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
        use crate::data_contract::document_type::DecodedValueBudget;
        use crate::data_contract::DataContract;
        use crate::document::{DocumentV0, DocumentV0Getters};
        use platform_value::{platform_value, Identifier, Value};
        use std::collections::BTreeMap;

        // Register a PV14 contract carrying a zero-width byte-array typed array.
        let pv14 = PlatformVersion::get(14).expect("protocol version 14 exists");
        let latest = PlatformVersion::latest();
        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::new([1; 32]),
                "ownerId": Identifier::new([2; 32]),
                "version": 1u32,
                "documentSchemas": {
                    "doc": {
                        "type": "object",
                        "properties": {
                            "empties": {
                                "type": "array",
                                "maxItems": 32,
                                "items": {
                                    "type": "array",
                                    "byteArray": true,
                                    "minItems": 0,
                                    "maxItems": 0,
                                },
                                "position": 0,
                            },
                        },
                        "additionalProperties": false,
                    }
                }
            }),
            true,
            pv14,
        )
        .expect("PV14 contract registers");
        let document_type = contract.document_type_for_name("doc").expect("doc type");

        let doc = Document::V0(DocumentV0 {
            id: Identifier::new([3; 32]),
            owner_id: Identifier::new([4; 32]),
            revision: Some(1),
            properties: BTreeMap::from([(
                "empties".to_string(),
                Value::Array(vec![Value::Bytes(vec![]); 32]),
            )]),
            ..Default::default()
        });
        let serialized = doc
            .serialize(document_type, &contract, latest)
            .expect("document serializes");

        // Consensus `from_bytes` keeps decoding stored PV14 documents without a budget.
        let decoded = Document::from_bytes(&serialized, document_type, latest)
            .expect("from_bytes decodes stored PV14 document");
        assert_eq!(
            decoded.properties().get("empties"),
            doc.properties().get("empties")
        );

        // `from_bytes_with_budget` charges 1 for the array + 32 for its elements = 33 per doc.
        let mut budget = DecodedValueBudget::new(50);
        let first =
            Document::from_bytes_with_budget(&serialized, document_type, &mut budget, latest)
                .expect("first document fits in budget");
        assert_eq!(
            first.properties().get("empties"),
            doc.properties().get("empties")
        );
        assert_eq!(budget.remaining(), 17);

        let error =
            Document::from_bytes_with_budget(&serialized, document_type, &mut budget, latest)
                .expect_err("second document must exhaust shared budget");
        assert!(
            matches!(
                error,
                crate::ProtocolError::DataContractError(
                    crate::data_contract::errors::DataContractError::DecodedValueBudgetExceeded(_)
                )
            ),
            "unexpected error: {error}"
        );
    }

    /// Format 0 falls back to format 1 for actual format errors (legacy rows written
    /// with sized integers under version 0), but budget exhaustion must be terminal so a
    /// low remaining budget never causes format 1 to reinterpret format-0 i64 bytes as
    /// narrower integers + optional markers and return an altered document.
    #[test]
    fn should_not_fall_back_to_format_1_when_format_0_exhausts_decoded_value_budget() {
        use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
        use crate::data_contract::document_type::DecodedValueBudget;
        use crate::data_contract::errors::DataContractError;
        use crate::data_contract::DataContract;
        use crate::document::serialization_traits::deserialize::v0::DocumentPlatformDeserializationMethodsV0;
        use crate::document::serialization_traits::serialize::v0::DocumentPlatformSerializationMethodsV0;
        use crate::document::{DocumentV0, DocumentV0Getters};
        use crate::ProtocolError;
        use platform_value::{platform_value, Identifier, Value};
        use std::collections::BTreeMap;

        let latest = PlatformVersion::latest();
        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::new([10; 32]),
                "ownerId": Identifier::new([11; 32]),
                "version": 1u32,
                "config": {
                    "$formatVersion": "1",
                    "sizedIntegerTypes": true,
                },
                "documentSchemas": {
                    "item": {
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "integer",
                                "minimum": 0,
                                "maximum": 255,
                                "position": 0,
                            },
                            "note": {
                                "type": "string",
                                "maxLength": 20,
                                "position": 1,
                            },
                        },
                        "required": ["count"],
                        "additionalProperties": false,
                    }
                }
            }),
            true,
            latest,
        )
        .expect("contract registers");
        let document_type = contract.document_type_for_name("item").expect("item type");

        // Serialize in format 0: `count = 0` is 8 zero bytes, followed by optional `note`.
        // Under format 1, byte 0 reads `count = 0u8` and byte 1 (`0`) looks like `note` absent!
        let original = DocumentV0 {
            id: Identifier::new([12; 32]),
            owner_id: Identifier::new([13; 32]),
            revision: Some(1),
            properties: BTreeMap::from([
                ("count".to_string(), Value::I64(0)),
                ("note".to_string(), Value::Text("legacy".to_string())),
            ]),
            ..Default::default()
        };
        let format_0_bytes =
            DocumentPlatformSerializationMethodsV0::serialize_v0(&original, document_type)
                .expect("format 0 serializes");
        assert_eq!(format_0_bytes[0], 0);

        // Confirm format 1 alone would indeed produce a DIFFERENT document (dropping `note`).
        let misparsed = DocumentV0::from_bytes_v1(&format_0_bytes[1..], document_type, latest)
            .expect("format 1 misparses the format-0 zero bytes as an absent optional field");
        assert_eq!(misparsed.properties.get("note"), None);

        // With enough budget (2 values: `count` + `note`), we get the exact historical document.
        let mut sufficient = DecodedValueBudget::new(2);
        let decoded = Document::from_bytes_with_budget(
            &format_0_bytes,
            document_type,
            &mut sufficient,
            latest,
        )
        .expect("sufficient budget decodes historical format-0 document");
        assert_eq!(decoded.properties().get("count"), Some(&Value::I64(0)));
        assert_eq!(
            decoded.properties().get("note"),
            Some(&Value::Text("legacy".to_string()))
        );
        assert_eq!(sufficient.remaining(), 0);

        // With budget = 1 (enough for `count`, short for `note`), format 0 hits budget exhaustion
        // and MUST return `DecodedValueBudgetExceeded`, never falling back to format 1.
        let mut tight = DecodedValueBudget::new(1);
        let error =
            Document::from_bytes_with_budget(&format_0_bytes, document_type, &mut tight, latest)
                .expect_err("tight budget must fail rather than returning an altered document");
        assert!(
            matches!(
                error,
                ProtocolError::DataContractError(DataContractError::DecodedValueBudgetExceeded(_))
            ),
            "expected DecodedValueBudgetExceeded, got: {error}"
        );
    }

    #[test]
    fn should_read_document_id_from_header_across_all_formats_without_decoding_properties() {
        use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
        use crate::data_contract::DataContract;
        use crate::document::serialization_traits::serialize::v0::DocumentPlatformSerializationMethodsV0;
        use crate::document::DocumentV0;
        use platform_value::{platform_value, Identifier, Value};
        use std::collections::BTreeMap;

        let pv14 = PlatformVersion::get(14).expect("PV14 exists");
        let latest = PlatformVersion::latest();
        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::new([20; 32]),
                "ownerId": Identifier::new([21; 32]),
                "version": 1u32,
                "documentSchemas": {
                    "doc": {
                        "type": "object",
                        "properties": {
                            "empties": {
                                "type": "array",
                                "maxItems": 1024,
                                "items": {
                                    "type": "array",
                                    "byteArray": true,
                                    "minItems": 0,
                                    "maxItems": 0,
                                },
                                "position": 0,
                            },
                        },
                        "additionalProperties": false,
                    }
                }
            }),
            true,
            pv14,
        )
        .expect("PV14 contract registers");
        let doc_type = contract.document_type_for_name("doc").expect("doc type");
        let expected_id = Identifier::new([0xAB; 32]);
        let doc = DocumentV0 {
            id: expected_id,
            owner_id: Identifier::new([0xCD; 32]),
            revision: Some(1),
            properties: BTreeMap::from([(
                "empties".to_string(),
                Value::Array(vec![Value::Bytes(vec![]); 1024]),
            )]),
            ..Default::default()
        };

        for format in 0..=3u16 {
            let bytes = match format {
                0 => DocumentPlatformSerializationMethodsV0::serialize_v0(&doc, doc_type),
                1 => DocumentPlatformSerializationMethodsV0::serialize_v1(&doc, doc_type),
                2 => DocumentPlatformSerializationMethodsV0::serialize_v2(&doc, doc_type),
                3 => DocumentPlatformSerializationMethodsV0::serialize_v3(&doc, doc_type),
                _ => unreachable!(),
            }
            .expect("serializes");
            // Works at both PV14 and latest (PV15), and even when bytes past the ID are truncated.
            let header_end = if format == 3 { 34 } else { 33 };
            for pv in [pv14, latest] {
                assert_eq!(
                    Document::id_from_bytes(&bytes, pv).expect("reads id"),
                    expected_id,
                    "format {format} at pv {}",
                    pv.protocol_version
                );
                assert_eq!(
                    Document::id_from_bytes(&bytes[..header_end], pv)
                        .expect("reads id from header prefix only"),
                    expected_id,
                    "header prefix format {format}"
                );
                assert!(Document::id_from_bytes(&bytes[..header_end - 1], pv).is_err());
            }
        }
        assert!(Document::id_from_bytes(&[], latest).is_err());
        assert!(Document::id_from_bytes(&[4, 0], latest).is_err());
    }
}
