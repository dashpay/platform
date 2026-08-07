//! Transport-free helpers for document create/replace transitions.
//!
//! `dash-sdk`'s `PutDocument` broadcast path calls these; embedders that
//! assemble their own transitions share the same preparation and
//! entropy/id consistency check.

use crate::Error;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::Identifier;

/// Returns a copy of `document` with its properties sanitized for the given
/// document type (e.g. integer arrays coerced back into byte arrays after a
/// WASM boundary crossing), leaving the caller's document untouched.
pub fn prepare_document_for_transition(
    document: &Document,
    document_type: &DocumentType,
) -> Document {
    let mut document = document.clone();
    document_type
        .as_ref()
        .sanitize_document_properties(document.properties_mut());
    document
}

/// Ensures a caller-supplied `entropy` derives the same document id already set
/// on a create document.
///
/// A document-create state transition carries both the document id and the
/// entropy, and Drive recomputes the id from the entropy during
/// `advanced_structure` validation, rejecting the transition with
/// `InvalidDocumentTransitionIdError` when they disagree. Because the
/// broadcast path trusts the caller's id verbatim when entropy is supplied,
/// a two-phase caller whose id and entropy have drifted would only discover
/// the mismatch after paying (a bumped identity-contract nonce). This check
/// surfaces the mismatch locally before broadcasting.
pub fn ensure_entropy_matches_document_id(
    contract_id: &Identifier,
    owner_id: &Identifier,
    document_type_name: &str,
    entropy: &[u8; 32],
    document_id: Identifier,
) -> Result<(), Error> {
    let expected_id = Document::generate_document_id_v0(
        contract_id,
        owner_id,
        document_type_name,
        entropy.as_slice(),
    );
    if expected_id != document_id {
        return Err(Error::InvalidInput(format!(
            "document id {document_id} does not match the id {expected_id} derived from the \
             supplied entropy; the entropy must be the one used to generate the document id"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::document::{DocumentV0, INITIAL_REVISION};
    use dpp::platform_value::{platform_value, Value};
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn contract_id() -> Identifier {
        Identifier::from([1u8; 32])
    }

    fn owner_id() -> Identifier {
        Identifier::from([2u8; 32])
    }

    #[test]
    fn matching_entropy_and_id_pass() {
        let entropy = [7u8; 32];
        let id = Document::generate_document_id_v0(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            entropy.as_slice(),
        );

        ensure_entropy_matches_document_id(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            &entropy,
            id,
        )
        .expect("id derived from the supplied entropy must be accepted");
    }

    #[test]
    fn mismatched_entropy_and_id_error_before_broadcast() {
        // The id was derived from E1, but the caller passes E2 != E1 (mirroring
        // the very drift consensus rejects with InvalidDocumentTransitionIdError).
        let entropy_used = [1u8; 32];
        let id = Document::generate_document_id_v0(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            entropy_used.as_slice(),
        );

        let different_entropy = [2u8; 32];
        let result = ensure_entropy_matches_document_id(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            &different_entropy,
            id,
        );

        assert!(
            matches!(result, Err(Error::InvalidInput(_))),
            "a document id derived from a different entropy must be rejected locally"
        );
    }

    #[test]
    fn should_normalize_wasm_uint8_array_property_without_mutating_caller_document() {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create default data contract config");
        let document_type = DocumentType::try_from_schema(
            contract_id(),
            1,
            config.version(),
            "preorder",
            platform_value!({
                "type": "object",
                "properties": {
                    "saltedDomainHash": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32_u32,
                        "maxItems": 32_u32,
                        "position": 0
                    }
                },
                "required": ["saltedDomainHash"],
                "additionalProperties": false,
            }),
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("should create DPNS-like document type");
        let integer_array = Value::Array(vec![Value::U64(7); 32]);
        let document = Document::V0(DocumentV0 {
            id: Identifier::new([3; 32]),
            owner_id: owner_id(),
            properties: BTreeMap::from([("saltedDomainHash".to_string(), integer_array.clone())]),
            revision: Some(INITIAL_REVISION),
            ..Default::default()
        });

        let prepared = prepare_document_for_transition(&document, &document_type);

        assert_eq!(
            prepared.properties().get("saltedDomainHash"),
            Some(&Value::Bytes32([7; 32]))
        );
        assert_eq!(
            document.properties().get("saltedDomainHash"),
            Some(&integer_array)
        );
    }
}
