pub mod array_field;
pub mod document_field;
pub mod document_type;
pub mod index;
pub mod random_document;

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
    pub const REF: &str = "$ref";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
    pub const MIN_ITEMS: &str = "minItems";
    pub const MAX_ITEMS: &str = "maxItems";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const BYTE_ARRAY: &str = "byteArray";
}
