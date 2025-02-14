use crate::error::to_js_error;
use dash_sdk::dpp::identity::KeyID;
use dash_sdk::dpp::serialization::PlatformSerializable;
use dash_sdk::dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dash_sdk::dpp::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use dash_sdk::dpp::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use dash_sdk::dpp::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use dash_sdk::dpp::state_transition::documents_batch_transition::{
    DocumentCreateTransition, DocumentsBatchTransition, DocumentsBatchTransitionV0,
};
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Number, Uint8Array};

#[wasm_bindgen]
pub fn create_document(
    _document: JsValue,
    _identity_contract_nonce: Number,
    signature_public_key_id: Number,
) -> Result<Uint8Array, JsError> {
    // TODO: Extract document fields from JsValue

    let _base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
        id: Default::default(),
        identity_contract_nonce: 1,
        document_type_name: "".to_string(),
        data_contract_id: Default::default(),
    });

    let transition = DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
        base: Default::default(),
        entropy: [0; 32],
        data: Default::default(),
        prefunded_voting_balance: None,
    });

    create_batch_transition(
        vec![DocumentTransition::Create(transition)],
        signature_public_key_id,
    )
}

fn create_batch_transition(
    transitions: Vec<DocumentTransition>,
    signature_public_key_id: Number,
) -> Result<Uint8Array, JsError> {
    let signature_public_key_id = signature_public_key_id
        .as_f64()
        .ok_or_else(|| JsError::new("public_key_id must be a number"))?;

    // boundary checks
    let signature_public_key_id = if signature_public_key_id.is_finite()
        && signature_public_key_id >= KeyID::MIN as f64
        && signature_public_key_id <= (KeyID::MAX as f64)
    {
        signature_public_key_id as KeyID
    } else {
        return Err(JsError::new(&format!(
            "signature_public_key_id {} out of valid range",
            signature_public_key_id
        )));
    };

    let document_batch_transition = DocumentsBatchTransition::V0(DocumentsBatchTransitionV0 {
        owner_id: Default::default(),
        transitions,
        user_fee_increase: 0,
        signature_public_key_id,
        signature: Default::default(),
    });

    document_batch_transition
        .serialize_to_bytes()
        .map_err(to_js_error)
        .map(|bytes| Uint8Array::from(bytes.as_slice()))
}
