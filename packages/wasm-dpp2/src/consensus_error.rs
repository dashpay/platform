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

    /// The enum's discriminants are the contract with JS. Anything that
    /// re-numbers them silently breaks every caller's `switch`.
    #[test]
    fn reference_error_codes_round_trip_through_their_discriminants() {
        let cases = [
            (
                40120,
                DocumentReferenceErrorCodeWasm::ReferencedEntityNotFound,
            ),
            (
                40121,
                DocumentReferenceErrorCodeWasm::ReferencedDocumentTypeNotFound,
            ),
            (
                40122,
                DocumentReferenceErrorCodeWasm::ReferencedDocumentTypeDeletable,
            ),
            (
                40123,
                DocumentReferenceErrorCodeWasm::ReferencedIdentityKeyNotFound,
            ),
            (
                40124,
                DocumentReferenceErrorCodeWasm::ReferencedIdentityKeyDisabled,
            ),
            (
                40125,
                DocumentReferenceErrorCodeWasm::ReferencedKeyIdPropertyInvalid,
            ),
        ];

        for (code, expected) in cases {
            assert_eq!(
                DocumentReferenceErrorCodeWasm::from_code(code),
                Some(expected)
            );
            assert_eq!(expected as u32, code);
        }
    }

    #[test]
    fn codes_outside_the_reference_range_are_not_claimed() {
        for code in [40119, 40126, 0, 40200] {
            assert_eq!(DocumentReferenceErrorCodeWasm::from_code(code), None);
        }
    }
}
