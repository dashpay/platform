/// Document module
pub mod document;

/// Random document module
pub mod random_document;

/// Import from dpp
pub use dpp::data_contract::{
    extra::{DocumentField, DocumentFieldType, DocumentType},
    extra::{Index, IndexProperty},
    DataContract, DataContract as Contract,
};

/// Import from random_document
pub use random_document::CreateRandomDocument;
