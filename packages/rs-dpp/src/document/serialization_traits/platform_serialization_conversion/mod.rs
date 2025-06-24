pub(in crate::document) mod deserialize;
pub(in crate::document) mod serialize;
mod v0;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0};
#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};
pub use v0::*;

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
}
