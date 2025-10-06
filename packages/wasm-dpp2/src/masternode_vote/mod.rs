pub mod resource_vote_choice;
pub mod vote;
pub mod vote_poll;

use crate::asset_lock_proof::AssetLockProofWasm;
use crate::identifier::IdentifierWasm;
use crate::masternode_vote::vote::VoteWasm;
use crate::state_transition::StateTransitionWasm;
use crate::utils::WithJsError;
use dpp::identity::KeyID;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::decode;
use dpp::prelude::IdentityNonce;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionLike};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "MasternodeVoteTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "MasternodeVoteTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_pro_tx_hash: &JsValue,
        js_voter_identity_id: &JsValue,
        vote: &VoteWasm,
        nonce: IdentityNonce,
        signature_public_key: Option<KeyID>,
        signature: Option<Vec<u8>>,
    ) -> Result<MasternodeVoteTransitionWasm, JsValue> {
        let pro_tx_hash = IdentifierWasm::try_from(js_pro_tx_hash)?;
        let voter_identity_id = IdentifierWasm::try_from(js_voter_identity_id)?;

        Ok(MasternodeVoteTransitionWasm(MasternodeVoteTransition::V0(
            MasternodeVoteTransitionV0 {
                pro_tx_hash: pro_tx_hash.into(),
                voter_identity_id: voter_identity_id.into(),
                vote: vote.clone().into(),
                nonce,
                signature_public_key_id: signature_public_key.unwrap_or(0),
                signature: BinaryData::from(signature.unwrap_or(vec![])),
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
    pub fn set_pro_tx_hash(&mut self, js_pro_tx_hash: &JsValue) -> Result<(), JsValue> {
        let pro_tx_hash = IdentifierWasm::try_from(js_pro_tx_hash)?;

        self.0.set_pro_tx_hash(pro_tx_hash.into());

        Ok(())
    }

    #[wasm_bindgen(setter = voterIdentityId)]
    pub fn set_voter_identity_id(&mut self, js_voter_identity_id: &JsValue) -> Result<(), JsValue> {
        let voter_identity_id = IdentifierWasm::try_from(js_voter_identity_id)?;

        self.0.set_voter_identity_id(voter_identity_id.into());

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
    pub fn from_hex(hex: String) -> Result<MasternodeVoteTransitionWasm, JsValue> {
        let bytes = decode(hex.as_str(), Hex).map_err(JsError::from)?;

        MasternodeVoteTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> Result<MasternodeVoteTransitionWasm, JsValue> {
        let bytes = decode(base64.as_str(), Base64).map_err(JsError::from)?;

        MasternodeVoteTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.serialize_to_bytes().with_js_error()
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<MasternodeVoteTransitionWasm, JsValue> {
        let rs_transition =
            MasternodeVoteTransition::deserialize_from_bytes(bytes.as_slice()).with_js_error()?;

        Ok(MasternodeVoteTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn get_user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.signable_bytes().with_js_error()
    }

    #[wasm_bindgen(getter = "assetLock")]
    pub fn get_asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
        match self.0.optional_asset_lock_proof().clone() {
            None => None,
            Some(asset_lock_proof) => Some(AssetLockProofWasm::from(asset_lock_proof.clone())),
        }
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
            .map(|id| id.clone().into())
            .collect()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::MasternodeVote(self.clone().0))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> Result<MasternodeVoteTransitionWasm, JsValue> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::MasternodeVote(st) => Ok(MasternodeVoteTransitionWasm(st)),
            _ => Err(JsValue::from_str(
                &"Invalid state document_transition type)",
            )),
        }
    }
}
