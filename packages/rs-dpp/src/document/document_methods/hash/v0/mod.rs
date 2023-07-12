use serde::Serialize;
use crate::data_contract::DataContract;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{DocumentV0Getters, DocumentV0Setters};
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::ProtocolError;
use crate::util::hash::hash_to_vec;

pub trait DocumentHashV0Method : DocumentPlatformConversionMethodsV0 {
    /// The document is only unique within the contract and document type
    /// Hence we must include contract and document type information to get uniqueness
    fn hash_v0(
        &self,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
    ) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = contract.id().to_vec();
        buf.extend(document_type.name().as_bytes());
        buf.extend(self.serialize(document_type)?);
        Ok(hash_to_vec(buf))
    }
}