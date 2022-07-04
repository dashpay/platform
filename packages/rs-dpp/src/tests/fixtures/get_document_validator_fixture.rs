use crate::document::document_validator::DocumentValidator;
use std::sync::Arc;

use super::get_protocol_version_validator;

pub fn get_document_validator() -> DocumentValidator {
    let protocol_version_validator = Arc::new(get_protocol_version_validator());
    DocumentValidator::new(protocol_version_validator)
}
