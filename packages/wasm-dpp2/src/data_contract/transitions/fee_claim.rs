use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_to_u16, try_to_u32, try_to_u64};
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::state_transition::contract_fee_claim_transition::accessors::ContractFeeClaimTransitionAccessorsV0;
use dpp::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use dpp::state_transition::{
    StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
    StateTransitionOwned, StateTransitionSingleSigned,
};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_FEE_CLAIM_TS: &str = r#"
/**
 * Pays out one of the two fee pots a data contract's document action fees collect in (protocol
 * version 14). The owner pot goes to the contract owner, who alone may claim it; the moderators
 * pot is split equally between the contract's moderation team, and any member may claim it. A
 * pot is paid out at most once per epoch. Signed with a CRITICAL authentication key, under the
 * signer's contract-scoped nonce.
 */
export interface ContractFeeClaimTransitionOptions {
    /** The claimant that signs */
    ownerId: IdentifierLike;
    /** The contract whose pot is paid out */
    dataContractId: IdentifierLike;
    /** The signer's nonce for the contract */
    identityContractNonce: bigint;
    /** The pot that is paid out */
    pot: "owner" | "moderators";
    userFeeIncrease?: number;
}

/**
 * ContractFeeClaim serialized as a plain object.
 */
export interface ContractFeeClaimObject {
    $formatVersion: string;
    ownerId: Uint8Array;
    dataContractId: Uint8Array;
    identityContractNonce: bigint;
    pot: string;
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * ContractFeeClaim serialized as JSON.
 */
export interface ContractFeeClaimJSON {
    $formatVersion: string;
    ownerId: string;
    dataContractId: string;
    identityContractNonce: number | string;
    pot: string;
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractFeeClaimTransitionOptions")]
    pub type ContractFeeClaimTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ContractFeeClaimObject")]
    pub type ContractFeeClaimObjectJs;

    #[wasm_bindgen(typescript_type = "ContractFeeClaimJSON")]
    pub type ContractFeeClaimJSONJs;
}

/// Serde struct for the primitive fields of the options
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractFeeClaimOptionsInput {
    identity_contract_nonce: u64,
    pot: String,
    /// `undefined` reaches serde as a unit value, so the fee is read as an option and defaulted
    #[serde(default)]
    user_fee_increase: Option<UserFeeIncrease>,
}

#[wasm_bindgen(js_name = "ContractFeeClaim")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ContractFeeClaimWasm(ContractFeeClaimTransition);

impl From<ContractFeeClaimTransition> for ContractFeeClaimWasm {
    fn from(val: ContractFeeClaimTransition) -> Self {
        ContractFeeClaimWasm(val)
    }
}

impl From<ContractFeeClaimWasm> for ContractFeeClaimTransition {
    fn from(val: ContractFeeClaimWasm) -> Self {
        val.0
    }
}

/// The pot for its name, as the wire shape spells it.
pub fn contract_fee_pot_from_str(pot: &str) -> WasmDppResult<ContractFeePot> {
    match pot {
        "owner" => Ok(ContractFeePot::Owner),
        "moderators" => Ok(ContractFeePot::Moderators),
        other => Err(WasmDppError::invalid_argument(format!(
            "unknown fee pot `{other}`: expected owner or moderators"
        ))),
    }
}

#[wasm_bindgen(js_class = ContractFeeClaim)]
impl ContractFeeClaimWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: ContractFeeClaimTransitionOptionsJs,
    ) -> WasmDppResult<ContractFeeClaimWasm> {
        // Extract complex types first (borrows &options)
        let owner_id: IdentifierWasm = try_from_options(&options, "ownerId")?;
        let data_contract_id: IdentifierWasm = try_from_options(&options, "dataContractId")?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: ContractFeeClaimOptionsInput = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let pot = contract_fee_pot_from_str(&input.pot)?;

        Ok(ContractFeeClaimWasm(ContractFeeClaimTransition::V0(
            ContractFeeClaimTransitionV0 {
                owner_id: owner_id.into(),
                data_contract_id: data_contract_id.into(),
                identity_contract_nonce: input.identity_contract_nonce,
                pot,
                user_fee_increase: input.user_fee_increase.unwrap_or_default(),
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        )))
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ContractFeeClaimWasm> {
        let rs_transition =
            ContractFeeClaimTransition::deserialize_from_bytes_untrusted(bytes.as_slice())?;

        Ok(ContractFeeClaimWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<ContractFeeClaimWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ContractFeeClaimWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<ContractFeeClaimWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ContractFeeClaimWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(setter = "ownerId")]
    pub fn set_owner_id(&mut self, owner_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_owner_id(owner_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_data_contract_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0
            .set_identity_contract_nonce(try_to_u64(nonce, "identityContractNonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "pot")]
    pub fn set_pot(&mut self, pot: String) -> WasmDppResult<()> {
        self.0.set_pot(contract_fee_pot_from_str(&pot)?);
        Ok(())
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "publicKeyId")] public_key_id: &js_sys::Number,
    ) -> WasmDppResult<()> {
        self.0
            .set_signature_public_key_id(try_to_u32(public_key_id, "signaturePublicKeyId")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: &js_sys::Number) -> WasmDppResult<()> {
        self.0
            .set_user_fee_increase(try_to_u16(amount, "userFeeIncrease")?);
        Ok(())
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> IdentifierWasm {
        self.0.owner_id().into()
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> u64 {
        self.0.identity_contract_nonce()
    }

    /// The pot that is paid out: owner or moderators
    #[wasm_bindgen(getter = "pot")]
    pub fn pot(&self) -> String {
        self.0.pot().to_string()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(st: &StateTransitionWasm) -> WasmDppResult<ContractFeeClaimWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::ContractFeeClaim(st) => Ok(ContractFeeClaimWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl ContractFeeClaimWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions_inner!(
    ContractFeeClaimWasm,
    ContractFeeClaimTransition,
    ContractFeeClaim,
    ContractFeeClaimObjectJs,
    ContractFeeClaimJSONJs
);

impl_wasm_type_info!(ContractFeeClaimWasm, ContractFeeClaim);
