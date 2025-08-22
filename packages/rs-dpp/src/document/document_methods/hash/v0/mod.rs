use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::util::hash::hash_double_to_vec;
use crate::version::PlatformVersion;
use crate::ProtocolError;

pub trait DocumentHashV0Method: DocumentPlatformConversionMethodsV0 {
    /// The document is only unique within the contract and document type
    /// Hence we must include contract and document type information to get uniqueness
    fn hash_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = contract.id().to_vec();
        buf.extend(document_type.name().as_bytes()); // TODO: Why we put it here?
        buf.extend(self.serialize(document_type, contract, platform_version)?);
        Ok(hash_double_to_vec(buf))
    }
}
