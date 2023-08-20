mod v0;

use crate::document::{Document, DocumentV0};
use crate::util::deserializer;
use crate::util::deserializer::SplitFeatureVersionOutcome;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use ciborium::Value as CborValue;
pub use v0::*;

impl DocumentCborMethodsV0 for Document {
    fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let SplitFeatureVersionOutcome {
            main_message_bytes: read_document_cbor,
            feature_version,
            ..
        } = deserializer::split_cbor_feature_version(document_cbor)?;

        if !platform_version
            .dpp
            .document_versions
            .document_cbor_serialization_version
            .check_version(feature_version)
        {
            return Err(ProtocolError::UnsupportedVersionMismatch {
                method: "Document::from_cbor (for document structure)".to_string(),
                allowed_versions: vec![0],
                received: feature_version,
            });
        }

        match feature_version {
            0 => DocumentV0::from_cbor(read_document_cbor, document_id, owner_id, platform_version)
                .map(|document| document.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_cbor (for document structure)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_cbor_value(),
        }
    }

    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_cbor(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentCborMethodsV0;
    use crate::tests::json_document::json_document_to_contract;
    use platform_version::version::LATEST_PLATFORM_VERSION;

    #[test]
    fn test_document_cbor_serialization() {
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected to get cbor contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type
            .random_document(Some(3333), LATEST_PLATFORM_VERSION)
            .expect("expected to get a random document");

        let document_cbor = document.to_cbor().expect("expected to encode to cbor");

        let recovered_document = Document::from_cbor(
            document_cbor.as_slice(),
            None,
            None,
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected to get document");

        assert_eq!(recovered_document, document);
    }
}
