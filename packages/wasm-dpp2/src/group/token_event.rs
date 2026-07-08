use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use dpp::tokens::token_event::TokenEvent;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * TokenEvent serialized as a plain object.
 *
 * Custom Serialize emits an internally-tagged flat shape: `$type:` is the
 * variant discriminator, positional tuple fields are mapped to named JSON
 * keys per variant. No `data` wrapper.
 *
 * Common per-variant payloads:
 *   - Mint:    { $type: "mint",    amount, recipient, publicNote }
 *   - Burn:    { $type: "burn",    amount, burnFromIdentifier, publicNote }
 *   - Freeze:  { $type: "freeze",  frozenIdentifier, publicNote }
 *   - Unfreeze:{ $type: "unfreeze",frozenIdentifier, publicNote }
 *   - DestroyFrozenFunds:        { $type, frozenIdentifier, amount, publicNote }
 *   - Transfer:{ $type, recipient, publicNote, sharedEncryptedNote,
 *                personalEncryptedNote, amount }
 *   - Claim:   { $type, distributionType, amount, publicNote }
 *   - EmergencyAction:           { $type, emergencyAction, publicNote }
 *   - ConfigUpdate:              { $type, configChange, publicNote }
 *   - ChangePriceForDirectPurchase: { $type, pricingSchedule, publicNote }
 *   - DirectPurchase: { $type, amount, credits }
 *
 * `amount`/`credits` are routed through json_safe_u64 — small numbers, JS
 * BigInt-safe stringification above 2^53. Identifier fields use base58 in
 * JSON, Uint8Array in toObject().
 */
export interface TokenEventObject {
    $type: string;
    [field: string]: unknown;
}

/**
 * TokenEvent serialized as JSON. Same shape as TokenEventObject with
 * Identifier fields rendered as base58 strings.
 */
export interface TokenEventJSON {
    $type: string;
    [field: string]: unknown;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenEventObject")]
    pub type TokenEventObjectJs;

    #[wasm_bindgen(typescript_type = "TokenEventJSON")]
    pub type TokenEventJSONJs;
}

/// TypeScript enum for TokenEvent variants
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenEventVariant {
    Mint = 0,
    Burn = 1,
    Freeze = 2,
    Unfreeze = 3,
    DestroyFrozenFunds = 4,
    Transfer = 5,
    Claim = 6,
    EmergencyAction = 7,
    ConfigUpdate = 8,
    ChangePriceForDirectPurchase = 9,
    DirectPurchase = 10,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "TokenEvent")]
pub struct TokenEventWasm(pub(crate) TokenEvent);

impl From<TokenEvent> for TokenEventWasm {
    fn from(event: TokenEvent) -> Self {
        TokenEventWasm(event)
    }
}

impl From<TokenEventWasm> for TokenEvent {
    fn from(event: TokenEventWasm) -> Self {
        event.0
    }
}

#[wasm_bindgen(js_class = TokenEvent)]
impl TokenEventWasm {
    #[wasm_bindgen(getter = "variant")]
    pub fn variant(&self) -> TokenEventVariant {
        match &self.0 {
            TokenEvent::Mint(..) => TokenEventVariant::Mint,
            TokenEvent::Burn(..) => TokenEventVariant::Burn,
            TokenEvent::Freeze(..) => TokenEventVariant::Freeze,
            TokenEvent::Unfreeze(..) => TokenEventVariant::Unfreeze,
            TokenEvent::DestroyFrozenFunds(..) => TokenEventVariant::DestroyFrozenFunds,
            TokenEvent::Transfer(..) => TokenEventVariant::Transfer,
            TokenEvent::Claim(..) => TokenEventVariant::Claim,
            TokenEvent::EmergencyAction(..) => TokenEventVariant::EmergencyAction,
            TokenEvent::ConfigUpdate(..) => TokenEventVariant::ConfigUpdate,
            TokenEvent::ChangePriceForDirectPurchase(..) => {
                TokenEventVariant::ChangePriceForDirectPurchase
            }
            TokenEvent::DirectPurchase(..) => TokenEventVariant::DirectPurchase,
        }
    }
}

impl_wasm_conversions_inner!(
    TokenEventWasm,
    TokenEvent,
    TokenEvent,
    TokenEventObjectJs,
    TokenEventJSONJs
);
impl_wasm_type_info!(TokenEventWasm, TokenEvent);
