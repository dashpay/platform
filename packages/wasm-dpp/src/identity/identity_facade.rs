use std::convert::TryInto;

use wasm_bindgen::prelude::*;

use dpp::identity::{Identity, IdentityFacade};

use crate::buffer::Buffer;
use crate::errors::{from_dpp_err, RustConversionError};
use crate::identifier::IdentifierWrapper;
use crate::identity::errors::InvalidIdentityError;

use crate::identity::state_transition::{
    create_asset_lock_proof_from_wasm_instance, ChainAssetLockProofWasm,
    IdentityCreateTransitionWasm, IdentityCreditTransferTransitionWasm,
    IdentityCreditWithdrawalTransitionWasm, IdentityTopUpTransitionWasm,
    IdentityUpdateTransitionWasm, InstantAssetLockProofWasm,
};

use crate::utils::Inner;

use crate::utils::WithJsError;
use crate::with_js_error;
use dpp::dashcore::{consensus, InstantLock, Transaction};

use crate::identity::IdentityWasm;
use dpp::identity::core_script::CoreScript;
use dpp::prelude::IdentityNonce;
use dpp::withdrawal::Pooling;
use dpp::NonConsensusError;
use serde::Deserialize;

#[derive(Clone)]
#[wasm_bindgen(js_name=IdentityFacade)]
pub struct IdentityFacadeWasm(IdentityFacade);

impl IdentityFacadeWasm {
    pub fn new(protocol_version: u32) -> IdentityFacadeWasm {
        let identity_facade = IdentityFacade::new(protocol_version);

        IdentityFacadeWasm(identity_facade)
    }
}

impl From<&IdentityFacade> for IdentityFacadeWasm {
    fn from(identity_facade: &IdentityFacade) -> Self {
        Self(identity_facade.to_owned())
    }
}
#[wasm_bindgen(js_class=IdentityFacade)]
impl IdentityFacadeWasm {
    #[wasm_bindgen]
    pub fn create(
        &self,
        id: IdentifierWrapper,
        public_keys: js_sys::Array,
    ) -> Result<IdentityWasm, JsValue> {
        let public_keys = super::factory_utils::parse_public_keys(public_keys)?;

        self.0
            .create(id.into(), public_keys)
            .map(|identity| identity.into())
            .with_js_error()
    }

    // TODO(versioning): not used anymore?
    // #[wasm_bindgen(js_name=createFromObject)]
    // pub fn create_from_object(
    //     &self,
    //     identity_object: JsValue,
    //     options: Option<js_sys::Object>,
    // ) -> Result<IdentityWasm, JsValue> {
    //     let options: FromObjectOptions = if let Some(options) = options {
    //         with_js_error!(serde_wasm_bindgen::from_value(options.into()))?
    //     } else {
    //         Default::default()
    //     };
    //
    //     let raw_identity = identity_object.with_serde_to_platform_value()?;
    //
    //     let result = self
    //         .0
    //         .create_from_object(raw_identity, options.skip_validation.unwrap_or(false));
    //
    //     match result {
    //         Ok(identity) => Ok(identity.into()),
    //         Err(dpp::ProtocolError::InvalidIdentityError { errors, .. }) => {
    //             Err(InvalidIdentityError::new(errors, identity_object).into())
    //         }
    //         Err(other) => Err(from_dpp_err(other)),
    //     }
    // }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: Option<js_sys::Object>,
    ) -> Result<IdentityWasm, JsValue> {
        let options: FromObjectOptions = if let Some(options) = options {
            with_js_error!(serde_wasm_bindgen::from_value(options.into()))?
        } else {
            Default::default()
        };

        let result = self
            .0
            .create_from_buffer(buffer.clone(), options.skip_validation.unwrap_or(false));

        match result {
            Ok(identity) => Ok(identity.into()),
            Err(dpp::ProtocolError::InvalidIdentityError { errors, .. }) => {
                Err(InvalidIdentityError::new(errors, Buffer::from_bytes(&buffer).into()).into())
            }
            Err(other) => Err(from_dpp_err(other)),
        }
    }

    // TODO(versioning): restore?
    // #[wasm_bindgen]
    // pub fn validate(&self, identity: &IdentityWasm) -> Result<ValidationResultWasm, JsValue> {
    //     let identity: Identity = identity.to_owned().into();
    //     let identity_json = identity.to_cleaned_object().with_js_error()?;
    //
    //     let validation_result = self
    //         .0
    //         .validate(&identity_json)
    //         .map_err(|e| from_dpp_err(e.into()))?;
    //     Ok(validation_result.map(|_| JsValue::undefined()).into())
    // }

    #[wasm_bindgen(js_name=createInstantAssetLockProof)]
    pub fn create_instant_asset_lock_proof(
        &self,
        instant_lock: Vec<u8>,
        asset_lock_transaction: Vec<u8>,
        output_index: u32,
    ) -> Result<InstantAssetLockProofWasm, JsError> {
        let instant_lock: InstantLock = consensus::deserialize(&instant_lock)
            .map_err(|e| JsError::new(format!("can't deserialize instant lock: {e}").as_str()))?;

        let asset_lock_transaction: Transaction = consensus::deserialize(&asset_lock_transaction)
            .map_err(|e| {
            JsError::new(format!("can't deserialize transaction: {e}").as_str())
        })?;

        Ok(IdentityFacade::create_instant_lock_proof(
            instant_lock,
            asset_lock_transaction,
            output_index,
        )
        .into())
    }

    #[wasm_bindgen(js_name=createChainAssetLockProof)]
    pub fn create_chain_asset_lock_proof(
        &self,
        core_chain_locked_height: u32,
        out_point: Vec<u8>,
    ) -> Result<ChainAssetLockProofWasm, JsValue> {
        let out_point: [u8; 36] = out_point.try_into().map_err(|_| {
            RustConversionError::Error(String::from("outPoint must be a 36 byte array"))
                .to_js_value()
        })?;

        Ok(
            IdentityFacade::create_chain_asset_lock_proof(core_chain_locked_height, out_point)
                .into(),
        )
    }

    #[wasm_bindgen(js_name=createIdentityCreateTransition)]
    pub fn create_identity_create_transition(
        &self,
        identity: &IdentityWasm,
        asset_lock_proof: JsValue,
    ) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&asset_lock_proof)?;

        self.0
            .create_identity_create_transition(
                &Identity::from(identity.to_owned()),
                asset_lock_proof,
            )
            .map(Into::into)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=createIdentityTopUpTransition)]
    pub fn create_identity_topup_transition(
        &self,
        identity_id: &IdentifierWrapper,
        asset_lock_proof: &JsValue,
    ) -> Result<IdentityTopUpTransitionWasm, JsValue> {
        let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(asset_lock_proof)?;

        self.0
            .create_identity_topup_transition(identity_id.to_owned().into(), asset_lock_proof)
            .map(Into::into)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=createIdentityCreditWithdrawalTransition)]
    pub fn create_identity_credit_withdrawal_transition(
        &self,
        identity_id: &IdentifierWrapper,
        amount: u64,
        core_fee_per_byte: u32,
        pooling: u8,
        output_script: Option<Vec<u8>>,
        identity_nonce: u64,
    ) -> Result<IdentityCreditWithdrawalTransitionWasm, JsValue> {
        let pooling = match pooling {
            0 => Pooling::Never,
            1 => Pooling::IfAvailable,
            2 => Pooling::Standard,
            _ => return Err(JsError::new("Invalid pooling value").into()),
        };

        self.0
            .create_identity_credit_withdrawal_transition(
                identity_id.to_owned().into(),
                amount,
                core_fee_per_byte,
                pooling,
                output_script.map(CoreScript::from_bytes),
                identity_nonce as IdentityNonce,
            )
            .map(Into::into)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=createIdentityCreditTransferTransition)]
    pub fn create_identity_credit_transfer_transition(
        &self,
        identity: &IdentityWasm,
        recipient_id: &IdentifierWrapper,
        amount: u64,
        identity_nonce: u64,
    ) -> Result<IdentityCreditTransferTransitionWasm, JsValue> {
        self.0
            .create_identity_credit_transfer_transition(
                identity.inner(),
                recipient_id.to_owned().into(),
                amount,
                identity_nonce,
            )
            .map(Into::into)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=createIdentityUpdateTransition)]
    pub fn create_identity_update_transition(
        &self,
        identity: &IdentityWasm,
        identity_nonce: u64,
        public_keys: &JsValue,
    ) -> Result<IdentityUpdateTransitionWasm, JsValue> {
        let (add_public_keys, disable_public_keys) =
            super::factory_utils::parse_create_identity_update_transition_keys(public_keys)?;

        self.0
            .create_identity_update_transition(
                identity.to_owned().into(),
                identity_nonce,
                add_public_keys,
                disable_public_keys,
            )
            .map(Into::into)
            .with_js_error()
    }
}

#[wasm_bindgen(js_name=NonConsensusErrorWasm)]
pub struct NonConsensusErrorWasm(NonConsensusError);

impl From<NonConsensusError> for NonConsensusErrorWasm {
    fn from(err: NonConsensusError) -> Self {
        Self(err)
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FromObjectOptions {
    pub skip_validation: Option<bool>,
}
