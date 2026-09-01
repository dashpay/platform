use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use dpp::consensus::ConsensusError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::serialization::PlatformDeserializable;
use wasm_bindgen::prelude::wasm_bindgen;

/// Consensus error codes emitted by `refersTo` reference validation, which
/// runs from protocol version 14 onward.
///
/// Branch on an error's `code` against these instead of matching its
/// message. Both directions work — `DocumentReferenceErrorCode[40123]` is
/// `"ReferencedIdentityKeyNotFound"`.
///
/// These reach JS on the state-transition broadcast path, where the
/// consensus code is carried through to `WasmSdkError.code`:
///
/// ```js
/// try {
///   await sdk.documents.create({ document, identityKey, signer });
/// } catch (e) {
///   if (e.code === DocumentReferenceErrorCode.ReferencedIdentityKeyDisabled) {
///     // the referenced key exists but was disabled
///   }
/// }
/// ```
#[wasm_bindgen(js_name = "DocumentReferenceErrorCode")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DocumentReferenceErrorCodeWasm {
    /// The referenced identity, contract, token or permanent document does
    /// not exist.
    ReferencedEntityNotFound = 40120,
    /// A `permanentDocument` reference names a document type the referenced
    /// contract does not define, or the contract itself is missing.
    ReferencedDocumentTypeNotFound = 40121,
    /// The referenced document type allows deletion. Only types declaring
    /// `canBeDeleted: false` may be the target of a `permanentDocument`
    /// reference — otherwise the reference could be left dangling.
    ReferencedDocumentTypeDeletable = 40122,
    /// The referenced identity public key does not exist.
    ReferencedIdentityKeyNotFound = 40123,
    /// The referenced identity public key exists but is disabled.
    ReferencedIdentityKeyDisabled = 40124,
    /// The declaration's `keyIdProperty` is missing from the document type,
    /// or names a property that is not an integer.
    ReferencedKeyIdPropertyInvalid = 40125,
}

impl DocumentReferenceErrorCodeWasm {
    /// The reference-validation error a code names, or `None` when the code
    /// is outside the 40120-40125 range.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            40120 => Some(Self::ReferencedEntityNotFound),
            40121 => Some(Self::ReferencedDocumentTypeNotFound),
            40122 => Some(Self::ReferencedDocumentTypeDeletable),
            40123 => Some(Self::ReferencedIdentityKeyNotFound),
            40124 => Some(Self::ReferencedIdentityKeyDisabled),
            40125 => Some(Self::ReferencedKeyIdPropertyInvalid),
            _ => None,
        }
    }
}

#[wasm_bindgen(js_name = "ConsensusError")]
pub struct ConsensusErrorWasm(ConsensusError);

#[wasm_bindgen(js_class = ConsensusError)]
impl ConsensusErrorWasm {
    #[wasm_bindgen(js_name = "deserialize")]
    pub fn deserialize(error: Vec<u8>) -> WasmDppResult<Self> {
        Ok(ConsensusErrorWasm(ConsensusError::deserialize_from_bytes(
            error.as_slice(),
        )?))
    }

    #[wasm_bindgen(getter = "message")]
    pub fn message(&self) -> String {
        self.0.to_string()
    }

    /// The consensus error code.
    ///
    /// This is the same number that reaches JS as `WasmSdkError.code` when
    /// a state transition is rejected. See [`DocumentReferenceErrorCodeWasm`]
    /// for the reference-validation range.
    #[wasm_bindgen(getter = "code")]
    pub fn code(&self) -> u32 {
        self.0.code()
    }

    /// The reference-validation error this is, or `undefined` when it is
    /// not one of codes 40120-40125.
    #[wasm_bindgen(getter = "documentReferenceErrorCode")]
    pub fn document_reference_error_code(&self) -> Option<DocumentReferenceErrorCodeWasm> {
        DocumentReferenceErrorCodeWasm::from_code(self.0.code())
    }
}

impl_wasm_type_info!(ConsensusErrorWasm, ConsensusError);

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
    use dpp::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
    use dpp::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
    use dpp::consensus::state::document::referenced_identity_key_disabled_error::ReferencedIdentityKeyDisabledError;
    use dpp::consensus::state::document::referenced_identity_key_not_found_error::ReferencedIdentityKeyNotFoundError;
    use dpp::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::data_contract::document_type::DocumentPropertyReferenceTarget;
    use dpp::prelude::Identifier;

    fn id() -> Identifier {
        Identifier::from([1u8; 32])
    }

    /// The six reference-validation errors, paired with the JS enum variant
    /// each is advertised to be.
    ///
    /// Deliberately built from the real DPP errors rather than from code
    /// literals: the codes come back through [`ErrorWithCode`], which is the
    /// source of truth the JS enum claims to mirror. Asserting against
    /// literals would keep passing if two names were consistently assigned
    /// each other's protocol code.
    fn cases() -> Vec<(ConsensusError, DocumentReferenceErrorCodeWasm)> {
        vec![
            (
                StateError::ReferencedEntityNotFoundError(ReferencedEntityNotFoundError::new(
                    id(),
                    DocumentPropertyReferenceTarget::Identity,
                    "author".to_string(),
                ))
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedEntityNotFound,
            ),
            (
                StateError::ReferencedDocumentTypeNotFoundError(
                    ReferencedDocumentTypeNotFoundError::new(
                        id(),
                        "note".to_string(),
                        "parentNoteId".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedDocumentTypeNotFound,
            ),
            (
                StateError::ReferencedDocumentTypeDeletableError(
                    ReferencedDocumentTypeDeletableError::new(
                        id(),
                        "note".to_string(),
                        "parentNoteId".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedDocumentTypeDeletable,
            ),
            (
                StateError::ReferencedIdentityKeyNotFoundError(
                    ReferencedIdentityKeyNotFoundError::new(id(), 3, "signerKey".to_string()),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedIdentityKeyNotFound,
            ),
            (
                StateError::ReferencedIdentityKeyDisabledError(
                    ReferencedIdentityKeyDisabledError::new(id(), 3, "signerKey".to_string()),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedIdentityKeyDisabled,
            ),
            (
                StateError::ReferencedKeyIdPropertyInvalidError(
                    ReferencedKeyIdPropertyInvalidError::new(
                        "signerKeyId".to_string(),
                        "signerKey".to_string(),
                        "not an integer".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedKeyIdPropertyInvalid,
            ),
        ]
    }

    /// Every JS enum variant must carry the code DPP actually emits for the
    /// error it names. This is the assertion that makes the enum a mirror of
    /// the protocol rather than a second, independent list of numbers.
    #[test]
    fn each_variant_carries_the_code_dpp_emits_for_that_error() {
        for (error, expected) in cases() {
            let canonical = error.code();

            assert_eq!(
                expected as u32, canonical,
                "{expected:?} is advertised for an error DPP codes as {canonical}"
            );
            assert_eq!(
                DocumentReferenceErrorCodeWasm::from_code(canonical),
                Some(expected),
                "code {canonical} should resolve back to {expected:?}"
            );
        }
    }

    /// A `ConsensusError` crossing to JS reports the same code, so
    /// `error.code` and the enum are comparable without a message regex.
    #[test]
    fn the_wasm_getter_reports_the_canonical_code() {
        for (error, expected) in cases() {
            let canonical = error.code();
            let wrapped = ConsensusErrorWasm(error);

            assert_eq!(wrapped.code(), canonical);
            assert_eq!(wrapped.document_reference_error_code(), Some(expected));
        }
    }

    /// The six codes are distinct — a copy-paste that gave two variants the
    /// same discriminant would otherwise slip past the pairwise checks.
    #[test]
    fn the_reference_codes_are_distinct() {
        let mut codes: Vec<u32> = cases().into_iter().map(|(error, _)| error.code()).collect();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();

        assert_eq!(codes.len(), total, "reference error codes must be distinct");
    }

    #[test]
    fn codes_outside_the_reference_range_are_not_claimed() {
        for code in [40119, 40126, 0, 40200] {
            assert_eq!(DocumentReferenceErrorCodeWasm::from_code(code), None);
        }
    }
}
