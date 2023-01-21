pub mod array_field;
pub mod document_field;
pub mod document_type;
pub mod index;

use super::errors::DataContractError;

pub use {
    array_field::ArrayFieldType,
    document_field::{
        encode_float, encode_signed_integer, encode_unsigned_integer, DocumentField,
        DocumentFieldType,
    },
    document_type::{DocumentType, IndexLevel},
    index::{Index, IndexProperty},
};

pub(self) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";
    pub const INDICES: &str = "indices";
    pub const PROPERTIES: &str = "properties";
    pub const REQUIRED: &str = "required";
    pub const TYPE: &str = "type";
}
