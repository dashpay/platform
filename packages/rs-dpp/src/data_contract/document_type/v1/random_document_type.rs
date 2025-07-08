use crate::data_contract::document_type::v0::random_document_type::RandomDocumentTypeParameters;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Identifier;
use rand::rngs::StdRng;

impl DocumentTypeV1 {
    pub fn random_document_type(
        parameters: RandomDocumentTypeParameters,
        data_contract_id: Identifier,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentTypeV0::random_document_type(
            parameters,
            data_contract_id,
            rng,
            platform_version,
        )?
        .into())
    }

    /// This is used to create an invalid random document type, often for testing
    pub fn invalid_random_document_type(
        parameters: RandomDocumentTypeParameters,
        data_contract_id: Identifier,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentTypeV0::invalid_random_document_type(
            parameters,
            data_contract_id,
            rng,
            platform_version,
        )?
        .into())
    }
}
