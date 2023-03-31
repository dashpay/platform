pub use document::{property_names, IDENTIFIER_FIELDS};
pub use state_transition::documents_batch_transition::document_transition;
pub use state_transition::documents_batch_transition::validation;
pub use state_transition::documents_batch_transition::DocumentsBatchTransition;

mod document;

pub mod document_facade;
pub mod document_factory;
mod document_patch;
pub mod document_validator;
pub mod errors;
pub mod extended_document;
pub mod fetch_and_validate_data_contract;
pub mod generate_document_id;
pub mod serialize;
pub mod state_transition;

pub use document::Document;
pub use extended_document::property_names as extended_document_property_names;
pub use extended_document::ExtendedDocument;
pub use extended_document::IDENTIFIER_FIELDS as EXTENDED_DOCUMENT_IDENTIFIER_FIELDS;
