pub mod document;
pub mod random_document;

pub use dpp::data_contract::{
    extra::{DocumentField, DocumentFieldType, DocumentType},
    extra::{Index, IndexProperty},
    DataContract, DataContract as Contract,
};
pub use random_document::CreateRandomDocument;
