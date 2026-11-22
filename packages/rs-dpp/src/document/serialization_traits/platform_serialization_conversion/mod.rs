pub(in crate::document) mod deserialize;
pub(in crate::document) mod serialize;
mod v0;

use crate::data_contract::document_type::{DecodedValueBudget, DocumentTypeRef};
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0};
#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};
pub use v0::*;

impl Document {
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
            error.to_string().contains("decoded value budget exhausted"),
            "unexpected error: {error}"
        );
    }
}
