use crate::buffer::Buffer;
use crate::errors::{from_dpp_err, RustConversionError};
use crate::identifier::IdentifierWrapper;
use crate::identity::errors::InvalidIdentityError;

use crate::identity::identity::IdentityWasm;
use crate::identity::state_transition::IdentityCreditTransferTransitionWasm;
use crate::identity::state_transition::InstantAssetLockProofWasm;
use crate::identity::state_transition::{
    ChainAssetLockProofWasm, IdentityCreditWithdrawalTransitionWasm,
};

use crate::{
    identity::state_transition::create_asset_lock_proof_from_wasm_instance,
    identity::state_transition::IdentityCreateTransitionWasm,
    identity::state_transition::IdentityTopUpTransitionWasm,
    identity::state_transition::IdentityUpdateTransitionWasm, with_js_error,
};
use dpp::dashcore::{consensus, InstantLock, Transaction};

use dpp::prelude::{Identity, IdentityNonce};

use serde::Deserialize;
use std::convert::TryInto;

use crate::utils::{Inner, WithJsError};
use dpp::identity::identity_factory::IdentityFactory;

use dpp::identity::core_script::CoreScript;
use dpp::withdrawal::Pooling;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name=IdentityFactory)]
pub struct IdentityFactoryWasm(IdentityFactory);

impl From<IdentityFactory> for IdentityFactoryWasm {
    fn from(factory: IdentityFactory) -> Self {
        Self(factory)
    }
}

#[wasm_bindgen(js_class=IdentityFactory)]
impl IdentityFactoryWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(protocol_version: u32) -> Result<IdentityFactoryWasm, JsValue> {
        let factory = IdentityFactory::new(protocol_version);
        Ok(factory.into())
    }

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
    //     options: JsValue,
    // ) -> Result<IdentityWasm, JsValue> {
    //     let options: FromObjectOptions = if options.is_object() {
    //         with_js_error!(serde_wasm_bindgen::from_value(options))?
    //     } else {
    //         Default::default()
    //     };
    //
    //     let raw_identity = with_serde_to_platform_value(&identity_object)?;
    //
    //     let result = self
    //         .0
    //         .create_from_object(raw_identity);
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
        options: JsValue,
    ) -> Result<IdentityWasm, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let result = self
            .0
            .create_from_buffer(buffer.clone(), options.skip_validation.unwrap_or(true));

        match result {
            Ok(identity) => Ok(identity.into()),
            Err(dpp::ProtocolError::InvalidIdentityError { errors, .. }) => {
                Err(InvalidIdentityError::new(errors, Buffer::from_bytes(&buffer).into()).into())
            }
            Err(other) => Err(from_dpp_err(other)),
        }
    }

    #[wasm_bindgen(js_name=createInstantAssetLockProof)]
    pub fn create_instant_asset_lock_proof(
        &self,
        instant_lock: Vec<u8>,
        asset_lock_transaction: Vec<u8>,
        output_index: u32,
    ) -> Result<InstantAssetLockProofWasm, JsValue> {
        let instant_lock: InstantLock =
            consensus::deserialize(&instant_lock).map_err(|e| e.to_string())?;

        let asset_lock_transaction: Transaction =
            consensus::deserialize(&asset_lock_transaction).map_err(|e| e.to_string())?;

        Ok(IdentityFactory::create_instant_lock_proof(
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
            IdentityFactory::create_chain_asset_lock_proof(core_chain_locked_height, out_point)
                .into(),
        )
    }

    #[wasm_bindgen(js_name=createIdentityCreateTransition)]
    pub fn create_identity_create_transition(
        &self,
        identity: &IdentityWasm,
        asset_lock_proof: &JsValue,
    ) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(asset_lock_proof)?;

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

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FromObjectOptions {
    pub skip_validation: Option<bool>,
}
