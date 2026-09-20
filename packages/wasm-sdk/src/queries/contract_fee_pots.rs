//! The fee pots of a data contract (`getContractFeePots`): what its document action fees have
//! paid into the owner pot and the moderators pot, and the last claim of each: the epoch and
//! the block time it was paid out in, and the identity that claimed it.

use crate::error::WasmSdkError;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::platform::contract_fee_pots::{ContractFeePotState, ContractFeePots};
use dash_sdk::platform::{Fetch, Identifier};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_FEE_POTS_TS: &'static str = r#"
/** One of the two pots the document action fees of a contract collect in. */
export type ContractFeePotKind = 'owner' | 'moderators';

/** What one fee pot of a contract holds. */
export interface ContractFeePotState {
  /** The credits in the pot: what a claim would pay out, less what an equal split leaves over. */
  credits: bigint;
  /**
   * The epoch the pot was last paid out in; undefined when it never was. A pot is paid out at
   * most once per epoch, so a claim in this epoch is refused.
   */
  lastClaimEpoch?: number;
  /** The time, in milliseconds, of the block that last paid the pot out; set with `lastClaimEpoch`. */
  lastClaimTimeMs?: bigint;
  /**
   * The identity that signed the last claim, base58; set with `lastClaimEpoch`. The contract
   * owner for the owner pot, and for the moderators pot the member of the moderation team
   * that claimed it for the team.
   */
  lastClaimantId?: string;
}

/**
 * The fee pots of a contract. A contract that charges no action fees, or whose fees nobody has
 * paid yet, has two empty pots.
 */
export interface ContractFeePots {
  /** The pot the contract owner claims. */
  owner: ContractFeePotState;
  /** The pot the contract's moderation team shares; any member claims it for all of them. */
  moderators: ContractFeePotState;
}
"#;

fn parse_contract_id(id: IdentifierLikeJs) -> Result<Identifier, WasmSdkError> {
    id.try_into()
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract id: {err}")))
}

fn pot_to_js(pot: &ContractFeePotState) -> Result<JsValue, WasmSdkError> {
    let result = js_sys::Object::new();
    let set = |key: &str, value: JsValue| {
        js_sys::Reflect::set(&result, &key.into(), &value)
            .map(|_| ())
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the fee pot")))
    };
    set("credits", js_sys::BigInt::from(pot.credits).into())?;
    // Epoch 0 is an epoch a pot can have been paid out in, so "never" is the absent fields.
    if let Some(last_claim) = pot.last_claim {
        set("lastClaimEpoch", JsValue::from(last_claim.epoch_index))?;
        set(
            "lastClaimTimeMs",
            js_sys::BigInt::from(last_claim.time_ms).into(),
        )?;
        set(
            "lastClaimantId",
            JsValue::from_str(&IdentifierWasm::from(last_claim.claimant_id).to_base58()),
        )?;
    }
    Ok(result.into())
}

fn pots_to_js(pots: &ContractFeePots) -> Result<JsValue, WasmSdkError> {
    let result = js_sys::Object::new();
    for (key, pot) in [("owner", &pots.owner), ("moderators", &pots.moderators)] {
        js_sys::Reflect::set(&result, &key.into(), &pot_to_js(pot)?)
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the fee pots")))?;
    }
    Ok(result.into())
}

#[wasm_bindgen]
impl WasmSdk {
    /// What the document action fees of a contract have collected for its owner and for its
    /// moderation team, and the last claim of each pot: its epoch, its block time and who
    /// claimed. Use it to decide whether a `contractClaimFees` is worth sending.
    ///
    /// # Example
    /// ```javascript
    /// const pots = await sdk.getContractFeePots(contractId);
    /// if (pots.moderators.credits > 0n) console.log('there are fees to claim');
    /// ```
    #[wasm_bindgen(
        js_name = "getContractFeePots",
        unchecked_return_type = "ContractFeePots"
    )]
    pub async fn get_contract_fee_pots(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<JsValue, WasmSdkError> {
        let contract_id = parse_contract_id(contract_id)?;
        let pots = ContractFeePots::fetch(self.as_ref(), contract_id)
            .await?
            .unwrap_or_default();
        pots_to_js(&pots)
    }

    /// The fee pots of a contract together with their proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractFeePotsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractFeePots>"
    )]
    pub async fn get_contract_fee_pots_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let contract_id = parse_contract_id(contract_id)?;
        let (pots, metadata, proof) =
            ContractFeePots::fetch_with_metadata_and_proof(self.as_ref(), contract_id, None)
                .await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            pots_to_js(&pots.unwrap_or_default())?,
            metadata,
            proof,
        ))
    }
}
