pub use document::{property_names, IDENTIFIER_FIELDS};
pub use state_transition::documents_batch_transition::document_transition;
pub use state_transition::documents_batch_transition::validation;
pub use state_transition::documents_batch_transition::DocumentsBatchTransition;

mod document;

mod accessors;
pub mod action_type;
pub mod document_facade;
pub mod document_factory;
mod document_patch;
pub mod errors;
pub mod extended_document;
pub mod fetch_and_validate_data_contract;
pub mod generate_document_id;
mod serde_serialize;
pub mod state_transition;
mod v0;

pub use v0::*;

pub use document::Document;
pub use extended_document::property_names as extended_document_property_names;
pub use extended_document::ExtendedDocument;
pub use extended_document::IDENTIFIER_FIELDS as EXTENDED_DOCUMENT_IDENTIFIER_FIELDS;
