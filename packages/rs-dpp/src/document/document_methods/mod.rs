use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;

mod get_raw_for_contract;
mod get_raw_for_document_type;
mod hash;
mod is_equal_ignoring_timestamps;

pub(in crate::document) use get_raw_for_contract::*;
pub(in crate::document) use get_raw_for_document_type::*;
pub(in crate::document) use hash::*;
pub(in crate::document) use is_equal_ignoring_timestamps::*;

pub trait DocumentMethodsV0 {
    /// Return a value given the path to its key and the document type for a contract.
    fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError>;

    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError>;

    fn hash(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError>;

    fn increment_revision(&mut self) -> Result<(), ProtocolError>;

    /// Returns if the documents are equal but ignoring the timestamp
    fn is_equal_ignoring_timestamps(
        &self,
        rhs: &Self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError>;
}
