pub mod accessors;
pub mod transition;

mod property_names {
    pub const AMOUNT: &str = "amount";
}

/// The Identifier fields in [`TokenBurnTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
