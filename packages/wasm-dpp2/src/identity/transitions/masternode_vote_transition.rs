use crate::VoteWasm;
use crate::asset_lock_proof::AssetLockProofWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_to_u16, try_to_u32, try_to_u64};
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
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const MASTERNODE_VOTE_OPTIONS_TS: &str = r#"
export interface MasternodeVoteTransitionOptions {
    proTxHash: IdentifierLike;
    voterIdentityId: IdentifierLike;
    vote: Vote;
    nonce: bigint;
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

/// Serde struct for MasternodeVoteTransitionOptions (primitives only)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MasternodeVoteTransitionOptionsInput {
    nonce: IdentityNonce,
    #[serde(default)]
    signature_public_key_id: KeyID,
    #[serde(default)]
    signature: Vec<u8>,
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
        // Extract complex types first (borrows &options)
        let pro_tx_hash: IdentifierWasm = try_from_options(&options, "proTxHash")?;
        let voter_identity_id: IdentifierWasm = try_from_options(&options, "voterIdentityId")?;

        let vote: VoteWasm = try_from_options(&options, "vote")?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: MasternodeVoteTransitionOptionsInput =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(MasternodeVoteTransitionWasm(MasternodeVoteTransition::V0(
            MasternodeVoteTransitionV0 {
                pro_tx_hash: pro_tx_hash.into(),
                voter_identity_id: voter_identity_id.into(),
                vote: vote.into(),
                nonce: input.nonce,
                signature_public_key_id: input.signature_public_key_id,
                signature: BinaryData::from(input.signature),
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
    pub fn set_pro_tx_hash(
        &mut self,
        #[wasm_bindgen(js_name = "proTxHash")] pro_tx_hash: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_pro_tx_hash(pro_tx_hash.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = voterIdentityId)]
    pub fn set_voter_identity_id(
        &mut self,
        #[wasm_bindgen(js_name = "voterIdentityId")] voter_identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_voter_identity_id(voter_identity_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = vote)]
    pub fn set_vote(&mut self, vote: &VoteWasm) {
        self.0.set_vote(vote.clone().into())
    }

    #[wasm_bindgen(setter = nonce)]
    pub fn set_nonce(&mut self, nonce: JsValue) -> WasmDppResult<()> {
        let nonce = try_to_u64(&nonce, "nonce")?;
        self.0 = match self.0.clone() {
            MasternodeVoteTransition::V0(mut vote) => {
                vote.nonce = nonce;

                MasternodeVoteTransition::V0(vote)
            }
        };
        Ok(())
    }

    #[wasm_bindgen(setter=signaturePublicKeyId)]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "signaturePublicKeyId")] signature_public_key_id: JsValue,
    ) -> WasmDppResult<()> {
        self.0.set_signature_public_key_id(try_to_u32(
            &signature_public_key_id,
            "signaturePublicKeyId",
        )?);
        Ok(())
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
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        self.0.signable_bytes().map_err(Into::into)
    }

    #[wasm_bindgen(getter = "assetLockProof")]
    pub fn asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
        self.0
            .optional_asset_lock_proof()
            .map(|asset_lock_proof| AssetLockProofWasm::from(asset_lock_proof.clone()))
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: JsValue) -> WasmDppResult<()> {
        self.0
            .set_user_fee_increase(try_to_u16(&amount, "userFeeIncrease")?);
        Ok(())
    }

    #[wasm_bindgen(getter = "modifiedDataIds")]
    pub fn modified_data_ids(&self) -> Vec<IdentifierWasm> {
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
