// mod validation;

// pub use validation::*;

use dpp::consensus::ConsensusError;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::state_transition::{StateTransition, StateTransitionValueConvert};
use dpp::version::PlatformVersion;
use dpp::{
    consensus::signature::SignatureError, state_transition::StateTransitionLike, ProtocolError,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::data_contract::DataContractWasm;
use crate::errors::protocol_error::from_protocol_error;
use crate::identity::IdentityPublicKeyWasm;
use crate::utils::ToSerdeJSONExt;
use crate::{
    bls_adapter::{BlsAdapter, JsBlsAdapter},
    utils::WithJsError,
};
use crate::{buffer::Buffer, identifier::IdentifierWrapper};

#[derive(Clone)]
#[wasm_bindgen(js_name=DataContractUpdateTransition)]
pub struct DataContractUpdateTransitionWasm(DataContractUpdateTransition);

impl From<DataContractUpdateTransition> for DataContractUpdateTransitionWasm {
    fn from(v: DataContractUpdateTransition) -> Self {
        DataContractUpdateTransitionWasm(v)
    }
}

impl From<DataContractUpdateTransitionWasm> for DataContractUpdateTransition {
    fn from(val: DataContractUpdateTransitionWasm) -> Self {
        val.0
    }
}

#[wasm_bindgen(js_class=DataContractUpdateTransition)]
impl DataContractUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<DataContractUpdateTransitionWasm, JsValue> {
        let platform_version = PlatformVersion::first();

        DataContractUpdateTransition::from_object(
            raw_parameters.with_serde_to_platform_value()?,
            platform_version,
        )
        .map(Into::into)
        .with_js_error()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        DataContractWasm::try_from_serialization_format(self.0.data_contract().clone(), false)
            .expect("should create data contract from serialized format")
    }

    // #[wasm_bindgen(js_name=setDataContractConfig)]
    // pub fn set_data_contract_config(&mut self, config: JsValue) -> Result<(), JsValue> {
    //     let res = serde_wasm_bindgen::from_value(config);
    //     self.0.data_contract.config = res.unwrap();
    //     Ok(())
    // }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u32 {
        self.0.state_transition_type() as u32
    }

    // #[wasm_bindgen(js_name=toJSON)]
    // pub fn to_json(&self, skip_signature: Option<bool>) -> Result<JsValue, JsValue> {
    //     let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //     Ok(self
    //         .0
    //         .to_json(skip_signature.unwrap_or(false))
    //         .with_js_error()?
    //         .serialize(&serializer)
    //         .expect("JSON is a valid object"))
    // }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize_to_bytes(&StateTransition::DataContractUpdate(
            self.0.clone(),
        ))
        .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=fromBuffer)]
    pub fn from_buffer(buffer: Vec<u8>) -> Result<DataContractUpdateTransitionWasm, JsValue> {
        let state_transition: StateTransition =
            PlatformDeserializable::deserialize_from_bytes(&buffer).with_js_error()?;
        match state_transition {
            StateTransition::DataContractUpdate(dct) => Ok(dct.into()),
            _ => Err(JsValue::from_str("Invalid state transition type")),
        }
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn modified_data_ids(&self) -> Vec<JsValue> {
        self.0
            .modified_data_ids()
            .into_iter()
            .map(|identifier| Into::<IdentifierWrapper>::into(identifier).into())
            .collect()
    }

    #[wasm_bindgen(js_name=isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        self.0.is_data_contract_state_transition()
    }

    #[wasm_bindgen(js_name=isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        self.0.is_document_state_transition()
    }

    #[wasm_bindgen(js_name=isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        self.0.is_identity_state_transition()
    }

    #[wasm_bindgen(js_name=isVotingStateTransition)]
    pub fn is_voting_state_transition(&self) -> bool {
        self.0.is_voting_state_transition()
    }

    // #[wasm_bindgen(js_name=hash)]
    // pub fn hash(&self, skip_signature: Option<bool>) -> Result<Buffer, JsValue> {
    //     let bytes = self
    //         .0
    //         .hash(skip_signature.unwrap_or(false))
    //         .with_js_error()?;
    //     Ok(Buffer::from_bytes(&bytes))
    // }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, skip_signature: Option<bool>) -> Result<JsValue, JsValue> {
        let serde_object = self
            .0
            .to_cleaned_object(skip_signature.unwrap_or(false))
            .map_err(from_protocol_error)?;

        serde_object
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| e.into())
    }

    #[wasm_bindgen]
    pub fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKeyWasm,
        private_key: Vec<u8>,
        bls: JsBlsAdapter,
    ) -> Result<(), JsValue> {
        let bls_adapter = BlsAdapter(bls);
        // TODO: come up with a better way to set signature to the binding.
        let mut state_transition = StateTransition::DataContractUpdate(self.0.clone());
        state_transition
            .sign(
                &identity_public_key.to_owned().into(),
                &private_key,
                &bls_adapter,
            )
            .with_js_error()?;

        let signature = state_transition.signature().to_owned();
        let signature_public_key_id = state_transition.signature_public_key_id().unwrap_or(0);

        self.0.set_signature(signature);
        self.0.set_signature_public_key_id(signature_public_key_id);

        Ok(())
    }

    #[wasm_bindgen(js_name=verifySignature)]
    pub fn verify_signature(
        &self,
        identity_public_key: &IdentityPublicKeyWasm,
        bls: JsBlsAdapter,
    ) -> Result<bool, JsValue> {
        let bls_adapter = BlsAdapter(bls);

        let verification_result = StateTransition::DataContractUpdate(self.0.clone())
            .verify_signature(&identity_public_key.to_owned().into(), &bls_adapter);

        match verification_result {
            Ok(()) => Ok(true),
            Err(protocol_error) => match &protocol_error {
                ProtocolError::ConsensusError(err) => match err.as_ref() {
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError { .. },
                    ) => Ok(false),
                    _ => Err(protocol_error),
                },
                _ => Err(protocol_error),
            },
        }
        .with_js_error()
    }
}
