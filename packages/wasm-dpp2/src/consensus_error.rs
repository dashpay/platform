use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use dpp::consensus::ConsensusError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::serialization::PlatformDeserializableUntrusted;
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
    /// The referenced identity, contract, token or document (permanent or
    /// deletable) does not exist, or a `listElement` value is not an element
    /// of the list it must be in (or was set while the property finding the
    /// list's document was not).
    ReferencedEntityNotFound = 40120,
    /// A `permanentDocument` or `deletableDocument` reference names a
    /// document type the referenced contract does not define, or the
    /// contract itself is missing.
    ReferencedDocumentTypeNotFound = 40121,
    /// The referenced document type allows deletion. Only types declaring
    /// `canBeDeleted: false` may be the target of a `permanentDocument`
    /// reference — otherwise the reference could be left dangling. A
    /// `deletableDocument` reference is the one for such a type.
    ReferencedDocumentTypeDeletable = 40122,
    /// The referenced identity public key does not exist.
    ReferencedIdentityKeyNotFound = 40123,
    /// The referenced identity public key exists but is disabled.
    ReferencedIdentityKeyDisabled = 40124,
    /// The declaration's `keyIdProperty` is missing from the document type,
    /// or names a property that is not an integer.
    ReferencedKeyIdPropertyInvalid = 40125,
    /// The referenced document type forbids deletion. Only types whose
    /// documents can be deleted may be the target of a `deletableDocument`
    /// reference; a `permanentDocument` reference is the one for a type
    /// declaring `canBeDeleted: false`.
    ReferencedDocumentTypeNotDeletable = 40131,
    /// The referenced contract exists but does not meet what the reference's
    /// `contractRequirements` require of it: elected moderation, its election
    /// open, a minimum age or a minimum time since its last update at the block
    /// time of the write, an owner relation to the writer of the referring
    /// document, or a config flag (read-only, keeping history, the owner
    /// protection of its elected moderation declaration).
    ReferencedContractRequirementNotMet = 40135,
    /// The referenced identity public key exists and is enabled but does not
    /// meet what the reference's `keyRequirements` require of it: its
    /// purpose, or a binding to a document type of the declaring contract.
    ReferencedIdentityKeyRequirementNotMet = 40136,
    /// A `refersTo` lookup into a document type of another contract cannot
    /// resolve there, reported at contract registration: the named index is
    /// missing or not unique, the keys do not cover it exactly, or a source
    /// holds a different kind of value than its index property. (A lookup into
    /// the declaring contract is refused by the contract parse instead.)
    ReferencedDocumentLookupInvalid = 40137,
    /// A `refersTo: listElement` whose list lives in a document type of
    /// another contract cannot be served by it, reported at contract
    /// registration: that type's documents can be deleted, the list is not a
    /// stored typed array of identifiers of it, or a replace could change the
    /// list. (A list in the declaring contract is refused by the contract parse
    /// instead.)
    ReferencedDocumentListInvalid = 40138,
}

impl DocumentReferenceErrorCodeWasm {
    /// The reference-validation error a code names, or `None` when the code
    /// is not in the 40120-40125 range, 40131 or 40135-40138.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            40120 => Some(Self::ReferencedEntityNotFound),
            40121 => Some(Self::ReferencedDocumentTypeNotFound),
            40122 => Some(Self::ReferencedDocumentTypeDeletable),
            40123 => Some(Self::ReferencedIdentityKeyNotFound),
            40124 => Some(Self::ReferencedIdentityKeyDisabled),
            40125 => Some(Self::ReferencedKeyIdPropertyInvalid),
            40131 => Some(Self::ReferencedDocumentTypeNotDeletable),
            40135 => Some(Self::ReferencedContractRequirementNotMet),
            40136 => Some(Self::ReferencedIdentityKeyRequirementNotMet),
            40137 => Some(Self::ReferencedDocumentLookupInvalid),
            40138 => Some(Self::ReferencedDocumentListInvalid),
            _ => None,
        }
    }
}

/// Consensus error codes emitted by the immutable-property check on document
/// replaces (`immutable` / `immutableAllowSetting`, protocol version 14+).
///
/// Branch on an error's `code` against this instead of matching its message:
///
/// ```js
/// try {
///   await sdk.documents.replace({ document, identityKey, signer });
/// } catch (e) {
///   if (e.code === DocumentImmutabilityErrorCode.DocumentImmutablePropertyChanged) {
///     // the replace touched a property the document type freezes
///   }
/// }
/// ```
#[wasm_bindgen(js_name = "DocumentImmutabilityErrorCode")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DocumentImmutabilityErrorCodeWasm {
    /// The replace changed, added or removed a property the document type
    /// lists under `immutable`, and the change was not the one first-time
    /// set `immutableAllowSetting` permits.
    DocumentImmutablePropertyChanged = 40128,
}

impl DocumentImmutabilityErrorCodeWasm {
    /// The immutability error a code names, or `None` for any other code.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            40128 => Some(Self::DocumentImmutablePropertyChanged),
            _ => None,
        }
    }
}

/// Consensus error codes emitted by the `distinctFrom` check on document
/// creates and replaces (protocol version 14+).
///
/// Branch on an error's `code` against this instead of matching its message:
///
/// ```js
/// try {
///   await sdk.documents.create({ document, identityKey, signer });
/// } catch (e) {
///   if (e.code === DocumentDistinctFromErrorCode.DocumentPropertyNotDistinct) {
///     // an identifier property equals the value it must differ from
///   }
/// }
/// ```
#[wasm_bindgen(js_name = "DocumentDistinctFromErrorCode")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DocumentDistinctFromErrorCodeWasm {
    /// A `distinctFrom` identifier property of the written document equals
    /// the value it must differ from: the document's `$ownerId`, or the named
    /// property of the same document.
    DocumentPropertyNotDistinct = 10419,
}

impl DocumentDistinctFromErrorCodeWasm {
    /// The distinctFrom error a code names, or `None` for any other code.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            10419 => Some(Self::DocumentPropertyNotDistinct),
            _ => None,
        }
    }
}

/// Consensus error codes emitted by `encryptedFor` validation, which runs
/// from protocol version 14 onward on every document create and replace.
///
/// Branch on an error's `code` against these instead of matching its
/// message:
///
/// ```js
/// try {
///   await sdk.documents.create({ document, identityKey, signer });
/// } catch (e) {
///   if (e.code === DocumentEncryptionErrorCode.InvalidEncryptedPropertyShape) {
///     // the bytes are not the shape the declared scheme produces
///   }
/// }
/// ```
#[wasm_bindgen(js_name = "DocumentEncryptionErrorCode")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DocumentEncryptionErrorCodeWasm {
    /// A property the document type declares `encryptedFor` was supplied
    /// with bytes that are not a ciphertext of the declared scheme: shorter
    /// than the IV plus one block, or not a multiple of the block length.
    /// The shape is all consensus checks about a ciphertext.
    InvalidEncryptedPropertyShape = 10420,
}

impl DocumentEncryptionErrorCodeWasm {
    /// The encryption error a code names, or `None` for any other code.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            10420 => Some(Self::InvalidEncryptedPropertyShape),
            _ => None,
        }
    }
}

/// Consensus error codes emitted by the `maxBytes` check, which runs from
/// protocol version 14 onward wherever a document is validated, on every
/// create and replace included.
///
/// Branch on an error's `code` against these instead of matching its
/// message:
///
/// ```js
/// try {
///   await sdk.documents.create({ document, identityKey, signer });
/// } catch (e) {
///   if (e.code === DocumentMaxBytesErrorCode.MaxBytesExceeded) {
///     // a string is longer in UTF-8 bytes than its property's maxBytes
///   }
/// }
/// ```
#[wasm_bindgen(js_name = "DocumentMaxBytesErrorCode")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DocumentMaxBytesErrorCodeWasm {
    /// A string, or an element of a typed array of strings, is longer in
    /// UTF-8 bytes than the `maxBytes` its property declares. `maxLength`
    /// counts characters, which are up to four bytes each.
    MaxBytesExceeded = 10421,
}

impl DocumentMaxBytesErrorCodeWasm {
    /// The maxBytes error a code names, or `None` for any other code.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            10421 => Some(Self::MaxBytesExceeded),
            _ => None,
        }
    }
}

/// Consensus error codes emitted by the `propertyConstraints` check, which
/// runs from protocol version 14 onward wherever a document is validated, on
/// every create and replace included.
///
/// Branch on an error's `code` against this instead of matching its message:
///
/// ```js
/// try {
///   await sdk.documents.create({ document, identityKey, signer });
/// } catch (e) {
///   if (e.code === DocumentPropertyConstraintErrorCode.DocumentPropertyConstraintViolated) {
///     // the document breaks one of its type's rules; the message names it
///   }
/// }
/// ```
#[wasm_bindgen(js_name = "DocumentPropertyConstraintErrorCode")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DocumentPropertyConstraintErrorCodeWasm {
    /// The written document breaks a rule of its document type's
    /// `propertyConstraints`: the comparison does not hold, or evaluating it
    /// overflowed, divided by zero, raised to a negative power or read a value
    /// that is not an integer.
    DocumentPropertyConstraintViolated = 10422,
}

impl DocumentPropertyConstraintErrorCodeWasm {
    /// The propertyConstraints error a code names, or `None` for any other code.
    fn from_code(code: u32) -> Option<Self> {
        match code {
            10422 => Some(Self::DocumentPropertyConstraintViolated),
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
        Ok(ConsensusErrorWasm(
            ConsensusError::deserialize_from_bytes_untrusted(error.as_slice())?,
        ))
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
    /// not one of codes 40120-40125, 40131, 40135 and 40136.
    #[wasm_bindgen(getter = "documentReferenceErrorCode")]
    pub fn document_reference_error_code(&self) -> Option<DocumentReferenceErrorCodeWasm> {
        DocumentReferenceErrorCodeWasm::from_code(self.0.code())
    }

    /// The immutable-property error this is, or `undefined` when it is not
    /// code 40128.
    #[wasm_bindgen(getter = "documentImmutabilityErrorCode")]
    pub fn document_immutability_error_code(&self) -> Option<DocumentImmutabilityErrorCodeWasm> {
        DocumentImmutabilityErrorCodeWasm::from_code(self.0.code())
    }

    /// The distinctFrom error this is, or `undefined` when it is not code
    /// 10419.
    #[wasm_bindgen(getter = "documentDistinctFromErrorCode")]
    pub fn document_distinct_from_error_code(&self) -> Option<DocumentDistinctFromErrorCodeWasm> {
        DocumentDistinctFromErrorCodeWasm::from_code(self.0.code())
    }
    /// The encrypted-property error this is, or `undefined` when it is not
    /// code 10420.
    #[wasm_bindgen(getter = "documentEncryptionErrorCode")]
    pub fn document_encryption_error_code(&self) -> Option<DocumentEncryptionErrorCodeWasm> {
        DocumentEncryptionErrorCodeWasm::from_code(self.0.code())
    }
    /// The maxBytes error this is, or `undefined` when it is not code 10421.
    #[wasm_bindgen(getter = "documentMaxBytesErrorCode")]
    pub fn document_max_bytes_error_code(&self) -> Option<DocumentMaxBytesErrorCodeWasm> {
        DocumentMaxBytesErrorCodeWasm::from_code(self.0.code())
    }

    /// The propertyConstraints error this is, or `undefined` when it is not
    /// code 10422.
    #[wasm_bindgen(getter = "documentPropertyConstraintErrorCode")]
    pub fn document_property_constraint_error_code(
        &self,
    ) -> Option<DocumentPropertyConstraintErrorCodeWasm> {
        DocumentPropertyConstraintErrorCodeWasm::from_code(self.0.code())
    }
}

impl_wasm_type_info!(ConsensusErrorWasm, ConsensusError);

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::state::document::referenced_contract_requirement_not_met_error::ReferencedContractRequirementNotMetError;
    use dpp::consensus::state::document::referenced_document_list_invalid_error::ReferencedDocumentListInvalidError;
    use dpp::consensus::state::document::referenced_document_lookup_invalid_error::ReferencedDocumentLookupInvalidError;
    use dpp::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
    use dpp::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
    use dpp::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
    use dpp::consensus::state::document::referenced_identity_key_disabled_error::ReferencedIdentityKeyDisabledError;
    use dpp::consensus::state::document::referenced_identity_key_not_found_error::ReferencedIdentityKeyNotFoundError;
    use dpp::consensus::state::document::referenced_identity_key_requirement_not_met_error::ReferencedIdentityKeyRequirementNotMetError;
    use dpp::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::data_contract::document_type::DocumentPropertyReferenceTarget;
    use dpp::prelude::Identifier;

    fn id() -> Identifier {
        Identifier::from([1u8; 32])
    }

    /// Built from the real DPP error rather than a code literal, for the
    /// same reason as `cases()` below: the code comes back through
    /// [`ErrorWithCode`], which is the source of truth the JS enum mirrors.
    #[test]
    fn immutability_error_code_mirrors_the_dpp_error() {
        use dpp::consensus::state::document::document_immutable_property_changed_error::DocumentImmutablePropertyChangedError;

        let error: ConsensusError = StateError::DocumentImmutablePropertyChangedError(
            DocumentImmutablePropertyChangedError::new(
                id(),
                "post".to_string(),
                "author".to_string(),
            ),
        )
        .into();

        assert_eq!(
            DocumentImmutabilityErrorCodeWasm::from_code(error.code()),
            Some(DocumentImmutabilityErrorCodeWasm::DocumentImmutablePropertyChanged)
        );
        assert_eq!(
            DocumentImmutabilityErrorCodeWasm::DocumentImmutablePropertyChanged as u32,
            error.code()
        );
        assert_eq!(
            ConsensusErrorWasm(error).document_immutability_error_code(),
            Some(DocumentImmutabilityErrorCodeWasm::DocumentImmutablePropertyChanged)
        );
        // A neighbouring code is not claimed.
        assert_eq!(DocumentImmutabilityErrorCodeWasm::from_code(40127), None);
    }

    /// Built from the real DPP error, for the same reason as the test above.
    #[test]
    fn distinct_from_error_code_mirrors_the_dpp_error() {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::basic::document::DocumentPropertyNotDistinctError;

        let error: ConsensusError =
            BasicError::DocumentPropertyNotDistinctError(DocumentPropertyNotDistinctError::new(
                "delegation".to_string(),
                "delegateId".to_string(),
                "$ownerId".to_string(),
            ))
            .into();

        assert_eq!(
            DocumentDistinctFromErrorCodeWasm::from_code(error.code()),
            Some(DocumentDistinctFromErrorCodeWasm::DocumentPropertyNotDistinct)
        );
        assert_eq!(
            DocumentDistinctFromErrorCodeWasm::DocumentPropertyNotDistinct as u32,
            error.code()
        );
        assert_eq!(
            ConsensusErrorWasm(error).document_distinct_from_error_code(),
            Some(DocumentDistinctFromErrorCodeWasm::DocumentPropertyNotDistinct)
        );
        // A neighbouring code is not claimed.
        assert_eq!(DocumentDistinctFromErrorCodeWasm::from_code(10418), None);
    }

    /// Built from the real DPP error rather than a code literal, like the
    /// immutability test above.
    #[test]
    fn encryption_error_code_mirrors_the_dpp_error() {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::basic::document::InvalidEncryptedPropertyShapeError;

        let error: ConsensusError = BasicError::InvalidEncryptedPropertyShapeError(
            InvalidEncryptedPropertyShapeError::new(
                "encryptedMessage".to_string(),
                "ecdh-secp256k1-aes256-cbc".to_string(),
                47,
                32,
                16,
            ),
        )
        .into();

        assert_eq!(
            DocumentEncryptionErrorCodeWasm::from_code(error.code()),
            Some(DocumentEncryptionErrorCodeWasm::InvalidEncryptedPropertyShape)
        );
        assert_eq!(
            DocumentEncryptionErrorCodeWasm::InvalidEncryptedPropertyShape as u32,
            error.code()
        );
        assert_eq!(
            ConsensusErrorWasm(error).document_encryption_error_code(),
            Some(DocumentEncryptionErrorCodeWasm::InvalidEncryptedPropertyShape)
        );
        // A neighbouring code is not claimed.
        assert_eq!(DocumentEncryptionErrorCodeWasm::from_code(10419), None);
    }

    /// Built from the real DPP error rather than a code literal, like the
    /// encryption test above.
    #[test]
    fn should_mirror_the_dpp_error_in_the_max_bytes_error_code() {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::basic::document::DocumentPropertyMaxBytesExceededError;

        let error = ConsensusError::from(BasicError::DocumentPropertyMaxBytesExceededError(
            DocumentPropertyMaxBytesExceededError::new("description".to_string(), 4098, 4096),
        ));

        assert_eq!(
            DocumentMaxBytesErrorCodeWasm::MaxBytesExceeded as u32,
            error.code()
        );
        assert_eq!(
            ConsensusErrorWasm(error).document_max_bytes_error_code(),
            Some(DocumentMaxBytesErrorCodeWasm::MaxBytesExceeded)
        );
        // A neighbouring code is not claimed.
        assert_eq!(DocumentMaxBytesErrorCodeWasm::from_code(10420), None);
    }

    /// Built from the real DPP error rather than a code literal, like the
    /// encryption test above.
    #[test]
    fn should_mirror_the_dpp_property_constraint_error_code() {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::basic::document::{
            DocumentPropertyConstraintViolatedError, PropertyConstraintViolation,
        };

        let error: ConsensusError = BasicError::DocumentPropertyConstraintViolatedError(
            DocumentPropertyConstraintViolatedError::new(
                "order".to_string(),
                "depositCoversOrder".to_string(),
                PropertyConstraintViolation::NotMet,
            ),
        )
        .into();

        assert_eq!(
            DocumentPropertyConstraintErrorCodeWasm::from_code(error.code()),
            Some(DocumentPropertyConstraintErrorCodeWasm::DocumentPropertyConstraintViolated)
        );
        assert_eq!(
            DocumentPropertyConstraintErrorCodeWasm::DocumentPropertyConstraintViolated as u32,
            error.code()
        );
        assert_eq!(
            ConsensusErrorWasm(error).document_property_constraint_error_code(),
            Some(DocumentPropertyConstraintErrorCodeWasm::DocumentPropertyConstraintViolated)
        );
        // A neighbouring code is not claimed.
        assert_eq!(
            DocumentPropertyConstraintErrorCodeWasm::from_code(10420),
            None
        );
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
                StateError::ReferencedContractRequirementNotMetError(
                    ReferencedContractRequirementNotMetError::new(
                        id(),
                        "moderation".to_string(),
                        "elected".to_string(),
                        "targetContractId".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedContractRequirementNotMet,
            ),
            (
                StateError::ReferencedIdentityKeyRequirementNotMetError(
                    ReferencedIdentityKeyRequirementNotMetError::new(
                        "joinRequest".to_string(),
                        "recipientId".to_string(),
                        id(),
                        3,
                        "purpose".to_string(),
                        "decryption".to_string(),
                        "encryption".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedIdentityKeyRequirementNotMet,
            ),
            (
                StateError::ReferencedDocumentLookupInvalidError(
                    ReferencedDocumentLookupInvalidError::new(
                        "electedCharter.members".to_string(),
                        "bySubmittedCharter".to_string(),
                        "is not unique".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedDocumentLookupInvalid,
            ),
            (
                StateError::ReferencedDocumentListInvalidError(
                    ReferencedDocumentListInvalidError::new(
                        "resignation.memberId".to_string(),
                        "members".to_string(),
                        "is not a typed array of identifiers".to_string(),
                    ),
                )
                .into(),
                DocumentReferenceErrorCodeWasm::ReferencedDocumentListInvalid,
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
        for code in [40119, 40126, 40139, 0, 40200] {
            assert_eq!(DocumentReferenceErrorCodeWasm::from_code(code), None);
        }
    }
}
