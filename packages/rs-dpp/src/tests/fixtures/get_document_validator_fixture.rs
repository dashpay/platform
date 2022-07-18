use crate::document::document_validator::DocumentValidator;
use std::sync::Arc;

use super::get_protocol_version_validator_fixture;

pub fn get_document_validator_fixture() -> DocumentValidator {
    let protocol_version_validator = Arc::new(get_protocol_version_validator_fixture());
    DocumentValidator::new(protocol_version_validator)
}
