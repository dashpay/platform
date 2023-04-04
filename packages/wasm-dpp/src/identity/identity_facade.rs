use std::convert::TryInto;

use std::sync::Arc;

use wasm_bindgen::prelude::*;

use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::{Identity, IdentityFacade};

use crate::bls_adapter::BlsAdapter;
use crate::buffer::Buffer;
use crate::errors::{from_dpp_err, RustConversionError};
use crate::identifier::IdentifierWrapper;
use crate::identity::errors::InvalidIdentityError;

use crate::utils::{ToSerdeJSONExt, WithJsError};
use crate::validation::ValidationResultWasm;
use crate::{
    create_asset_lock_proof_from_wasm_instance, with_js_error, ChainAssetLockProofWasm,
    IdentityCreateTransitionWasm, IdentityTopUpTransitionWasm, IdentityUpdateTransitionWasm,
    IdentityWasm, InstantAssetLockProofWasm,
};
use dpp::dashcore::{consensus, InstantLock, Transaction};

use dpp::version::ProtocolVersionValidator;
use dpp::{Convertible, NonConsensusError};
use serde::Deserialize;

#[derive(Clone)]
#[wasm_bindgen(js_name=IdentityFacade)]
pub struct IdentityFacadeWasm(IdentityFacade<BlsAdapter>);

impl IdentityFacadeWasm {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<PublicKeysValidator<BlsAdapter>>,
    ) -> IdentityFacadeWasm {
        let identity_facade = IdentityFacade::new(
            protocol_version_validator.protocol_version(),
            protocol_version_validator,
            public_keys_validator,
        )
        .unwrap();

        IdentityFacadeWasm(identity_facade)
    }
}

#[wasm_bindgen(js_class=IdentityFacade)]
impl IdentityFacadeWasm {
    #[wasm_bindgen]
    pub fn create(
        &self,
        asset_lock_proof: JsValue,
        public_keys: js_sys::Array,
    ) -> Result<IdentityWasm, JsValue> {
        let (asset_lock_proof, public_keys) =
            super::factory_utils::parse_create_args(asset_lock_proof, public_keys)?;

        self.0
            .create(asset_lock_proof, public_keys)
            .map(|identity| identity.into())
            .with_js_error()
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub fn create_from_object(
        &self,
        identity_object: JsValue,
        options: JsValue,
    ) -> Result<IdentityWasm, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let raw_identity = identity_object.with_serde_to_platform_value()?;

        let result = self
            .0
            .create_from_object(raw_identity, options.skip_validation.unwrap_or(false));

        match result {
            Ok(identity) => Ok(identity.into()),
            Err(dpp::ProtocolError::InvalidIdentityError { errors, .. }) => {
                Err(InvalidIdentityError::new(errors, identity_object).into())
            }
            Err(other) => Err(from_dpp_err(other)),
        }
    }

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
            .create_from_buffer(buffer.clone(), options.skip_validation.unwrap_or(false));

        match result {
            Ok(identity) => Ok(identity.into()),
            Err(dpp::ProtocolError::InvalidIdentityError { errors, .. }) => {
                Err(InvalidIdentityError::new(errors, Buffer::from_bytes(&buffer).into()).into())
            }
            Err(other) => Err(from_dpp_err(other)),
        }
    }

    #[wasm_bindgen]
    pub fn validate(&self, identity: IdentityWasm) -> Result<ValidationResultWasm, JsValue> {
        let identity: Identity = identity.into();
        let identity_json = identity.to_cleaned_object().with_js_error()?;

        let validation_result = self
            .0
            .validate(&identity_json)
            .map_err(|e| from_dpp_err(e.into()))?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
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

        Ok(IdentityFacade::<BlsAdapter>::create_instant_lock_proof(
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

        Ok(IdentityFacade::<BlsAdapter>::create_chain_asset_lock_proof(
            core_chain_locked_height,
            out_point,
        )
        .into())
    }

    #[wasm_bindgen(js_name=createIdentityCreateTransition)]
    pub fn create_identity_create_transition(
        &self,
        identity: &IdentityWasm,
    ) -> Result<IdentityCreateTransitionWasm, JsValue> {
        self.0
            .create_identity_create_transition(Identity::from(identity.to_owned()))
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

    #[wasm_bindgen(js_name=createIdentityUpdateTransition)]
    pub fn create_identity_update_transition(
        &self,
        identity: &IdentityWasm,
        public_keys: &JsValue,
    ) -> Result<IdentityUpdateTransitionWasm, JsValue> {
        let (add_public_keys, disable_public_keys) =
            super::factory_utils::parse_create_identity_update_transition_keys(public_keys)?;
        let now = js_sys::Date::now() as u64;

        self.0
            .create_identity_update_transition(
                identity.to_owned().into(),
                add_public_keys,
                disable_public_keys,
                Some(now),
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
