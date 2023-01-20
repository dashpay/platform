pub mod document_type;
pub mod document_field;
pub mod array_field;
pub mod index;
pub mod mutability;

use super::errors::DataContractError;

pub use {
    array_field::ArrayFieldType,
    document_field::{
        DocumentField, DocumentFieldType, encode_float, encode_signed_integer,
        encode_unsigned_integer,
    },
    document_type::{DocumentType, IndexLevel},
    drive_api::{DriveContractExt, DriveEncoding},
    errors::{ContractError, StructureError},
    index::{Index, IndexProperty},
    mutability::ContractConfig,
    root_tree::RootTree,
};

pub(self) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";
    pub const INDICES: &str = "indices";
    pub const PROPERTIES: &str = "properties";
    pub const REQUIRED: &str = "required";
    pub const TYPE: &str = "type";
}
