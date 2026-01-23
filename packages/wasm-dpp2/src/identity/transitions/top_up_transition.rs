use crate::asset_lock_proof::AssetLockProofWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use dpp::identifier::Identifier;
use dpp::identity::state_transition::{AssetLockProved, OptionallyAssetLockProved};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike, StateTransitionSingleSigned};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * IdentityTopUpTransition serialized as a plain object.
 */
export interface IdentityTopUpTransitionObject {
    identityId: Uint8Array;
    assetLockProof: AssetLockProofObject;
    userFeeIncrease: number;
    signature?: Uint8Array;
}

/**
 * IdentityTopUpTransition serialized as JSON.
 */
export interface IdentityTopUpTransitionJSON {
    identityId: string;
    assetLockProof: AssetLockProofJSON;
    userFeeIncrease: number;
    signature?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityTopUpTransitionObject")]
    pub type IdentityTopUpTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityTopUpTransitionJSON")]
    pub type IdentityTopUpTransitionJSONJs;
}

#[wasm_bindgen(js_name = "IdentityTopUpTransition")]
#[derive(Clone)]
pub struct IdentityTopUpTransitionWasm(IdentityTopUpTransition);

#[wasm_bindgen(js_class = IdentityTopUpTransition)]
impl IdentityTopUpTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        asset_lock_proof: &AssetLockProofWasm,
        identity_id: IdentifierLikeJs,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> WasmDppResult<IdentityTopUpTransitionWasm> {
        let identity_id: Identifier = identity_id.try_into()?;

        Ok(IdentityTopUpTransitionWasm(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0 {
                asset_lock_proof: asset_lock_proof.clone().into(),
                identity_id,
                user_fee_increase: user_fee_increase.unwrap_or(0),
                signature: Default::default(),
            },
        )))
    }

    #[wasm_bindgen(getter = "modifiedDataIds")]
    pub fn modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .iter()
            .map(|id| (*id).into())
            .collect()
    }

    #[wasm_bindgen(getter = "optionalAssetLockProof")]
    pub fn optional_asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
        self.0
            .optional_asset_lock_proof()
            .map(|asset_lock| AssetLockProofWasm::from(asset_lock.clone()))
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "identityIdentifier")]
    pub fn identity_identifier(&self) -> IdentifierWasm {
        (*self.0.identity_id()).into()
    }

    #[wasm_bindgen(getter = "assetLockProof")]
    pub fn asset_lock_proof(&self) -> AssetLockProofWasm {
        self.0.asset_lock_proof().clone().into()
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.0.set_user_fee_increase(user_fee_increase);
    }

    #[wasm_bindgen(setter = "identityIdentifier")]
    pub fn set_identity_identifier(
        &mut self,
        identity_identifier: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_identity_id(identity_identifier.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "assetLockProof")]
    pub fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: &AssetLockProofWasm,
    ) -> WasmDppResult<()> {
        self.0
            .set_asset_lock_proof(asset_lock_proof.clone().into())?;
        Ok(())
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.signable_bytes()?)
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityTopUpTransitionWasm> {
        let rs_transition = IdentityTopUpTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityTopUpTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityTopUpTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityTopUpTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityTopUpTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityTopUpTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityTopUpTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityTopUp(st) => Ok(IdentityTopUpTransitionWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(
    IdentityTopUpTransitionWasm,
    IdentityTopUpTransition,
    IdentityTopUpTransitionObjectJs,
    IdentityTopUpTransitionJSONJs
);
impl_wasm_type_info!(IdentityTopUpTransitionWasm, IdentityTopUpTransition);
