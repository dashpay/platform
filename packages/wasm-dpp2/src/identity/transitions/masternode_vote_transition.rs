use crate::VoteWasm;
use crate::asset_lock_proof::AssetLockProofWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{IntoWasm, try_to_u64};
use dpp::identity::KeyID;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::IdentityNonce;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use dpp::state_transition::{
    StateTransition, StateTransitionIdentitySigned, StateTransitionLike,
    StateTransitionSingleSigned,
};
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const MASTERNODE_VOTE_OPTIONS_TS: &'static str = r#"
export interface MasternodeVoteTransitionOptions {
    proTxHash: IdentifierLike;
    voterIdentityId: IdentifierLike;
    vote: Vote;
    nonce: bigint | number;
    signaturePublicKeyId?: number;
    signature?: Uint8Array;
}

/**
 * MasternodeVoteTransition serialized as a plain object.
 */
export interface MasternodeVoteTransitionObject {
    proTxHash: Uint8Array;
    voterIdentityId: Uint8Array;
    vote: VoteObject;
    nonce: bigint;
    signaturePublicKeyId?: number;
    signature?: Uint8Array;
}

/**
 * MasternodeVoteTransition serialized as JSON.
 */
export interface MasternodeVoteTransitionJSON {
    proTxHash: string;
    voterIdentityId: string;
    vote: VoteJSON;
    nonce: string;
    signaturePublicKeyId?: number;
    signature?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "MasternodeVoteTransitionOptions")]
    pub type MasternodeVoteTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "MasternodeVoteTransitionObject")]
    pub type MasternodeVoteTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "MasternodeVoteTransitionJSON")]
    pub type MasternodeVoteTransitionJSONJs;
}

#[wasm_bindgen(js_name = "MasternodeVoteTransition")]
#[derive(Clone)]
pub struct MasternodeVoteTransitionWasm(MasternodeVoteTransition);

impl From<MasternodeVoteTransition> for MasternodeVoteTransitionWasm {
    fn from(val: MasternodeVoteTransition) -> Self {
        MasternodeVoteTransitionWasm(val)
    }
}

impl From<MasternodeVoteTransitionWasm> for MasternodeVoteTransition {
    fn from(val: MasternodeVoteTransitionWasm) -> Self {
        val.0
    }
}

#[wasm_bindgen(js_class = MasternodeVoteTransition)]
impl MasternodeVoteTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: MasternodeVoteTransitionOptionsJs,
    ) -> WasmDppResult<MasternodeVoteTransitionWasm> {
        let options_obj = Object::from(JsValue::from(options));

        let pro_tx_hash_js = Reflect::get(&options_obj, &"proTxHash".into())
            .map_err(|_| WasmDppError::invalid_argument("proTxHash is required"))?;
        let pro_tx_hash = IdentifierWasm::try_from(&pro_tx_hash_js)?.into();

        let voter_identity_id_js = Reflect::get(&options_obj, &"voterIdentityId".into())
            .map_err(|_| WasmDppError::invalid_argument("voterIdentityId is required"))?;
        let voter_identity_id = IdentifierWasm::try_from(&voter_identity_id_js)?.into();

        let vote_js = Reflect::get(&options_obj, &"vote".into())
            .map_err(|_| WasmDppError::invalid_argument("vote is required"))?;
        let vote = vote_js.to_wasm::<VoteWasm>("Vote")?.clone();

        let nonce = try_to_u64(
            Reflect::get(&options_obj, &"nonce".into())
                .map_err(|_| WasmDppError::invalid_argument("nonce is required"))?,
        )?;

        let signature_public_key_js = Reflect::get(&options_obj, &"signaturePublicKeyId".into())
            .unwrap_or(JsValue::UNDEFINED);
        let signature_public_key_id: KeyID = if signature_public_key_js.is_undefined() {
            0
        } else {
            signature_public_key_js.as_f64().ok_or_else(|| {
                WasmDppError::invalid_argument("signaturePublicKeyId must be a number")
            })? as KeyID
        };

        let signature_js =
            Reflect::get(&options_obj, &"signature".into()).unwrap_or(JsValue::UNDEFINED);
        let signature: Vec<u8> = if signature_js.is_undefined() {
            Vec::new()
        } else {
            Uint8Array::from(signature_js).to_vec()
        };

        Ok(MasternodeVoteTransitionWasm(MasternodeVoteTransition::V0(
            MasternodeVoteTransitionV0 {
                pro_tx_hash,
                voter_identity_id,
                vote: vote.into(),
                nonce,
                signature_public_key_id,
                signature: BinaryData::from(signature),
            },
        )))
    }

    #[wasm_bindgen(getter = proTxHash)]
    pub fn pro_tx_hash(&self) -> IdentifierWasm {
        self.0.pro_tx_hash().into()
    }

    #[wasm_bindgen(getter = voterIdentityId)]
    pub fn voter_identity_id(&self) -> IdentifierWasm {
        self.0.voter_identity_id().into()
    }

    #[wasm_bindgen(getter = vote)]
    pub fn vote(&self) -> VoteWasm {
        self.0.vote().clone().into()
    }

    #[wasm_bindgen(getter = nonce)]
    pub fn nonce(&self) -> IdentityNonce {
        self.0.nonce()
    }

    #[wasm_bindgen(getter=signaturePublicKeyId)]
    pub fn signature_public_key_id(&self) -> KeyID {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter=signature)]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().clone().to_vec()
    }

    #[wasm_bindgen(setter = proTxHash)]
    pub fn set_pro_tx_hash(&mut self, pro_tx_hash: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_pro_tx_hash(pro_tx_hash.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = voterIdentityId)]
    pub fn set_voter_identity_id(
        &mut self,
        voter_identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_voter_identity_id(voter_identity_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = vote)]
    pub fn set_vote(&mut self, vote: &VoteWasm) {
        self.0.set_vote(vote.clone().into())
    }

    #[wasm_bindgen(setter = nonce)]
    pub fn set_nonce(&mut self, nonce: IdentityNonce) {
        self.0 = match self.0.clone() {
            MasternodeVoteTransition::V0(mut vote) => {
                vote.nonce = nonce;

                MasternodeVoteTransition::V0(vote)
            }
        }
    }

    #[wasm_bindgen(setter=signaturePublicKeyId)]
    pub fn set_signature_public_key_id(&mut self, signature_public_key_id: KeyID) {
        self.0.set_signature_public_key_id(signature_public_key_id)
    }

    #[wasm_bindgen(setter=signature)]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature);
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<MasternodeVoteTransitionWasm> {
        let bytes = decode(hex.as_str(), Hex)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        MasternodeVoteTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<MasternodeVoteTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        MasternodeVoteTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        self.0.serialize_to_bytes().map_err(Into::into)
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<MasternodeVoteTransitionWasm> {
        let rs_transition = MasternodeVoteTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(MasternodeVoteTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn get_user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        self.0.signable_bytes().map_err(Into::into)
    }

    #[wasm_bindgen(getter = "assetLock")]
    pub fn get_asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
        self.0
            .optional_asset_lock_proof()
            .map(|asset_lock_proof| AssetLockProofWasm::from(asset_lock_proof.clone()))
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: u16) {
        self.0.set_user_fee_increase(amount)
    }

    #[wasm_bindgen(getter = "modifiedDataIds")]
    pub fn get_modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .iter()
            .map(|id| (*id).into())
            .collect()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::MasternodeVote(self.clone().0))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<MasternodeVoteTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::MasternodeVote(st) => Ok(MasternodeVoteTransitionWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state document_transition type",
            )),
        }
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
}

impl_wasm_conversions!(
    MasternodeVoteTransitionWasm,
    MasternodeVoteTransition,
    MasternodeVoteTransitionObjectJs,
    MasternodeVoteTransitionJSONJs
);

impl_wasm_type_info!(MasternodeVoteTransitionWasm, MasternodeVoteTransition);
