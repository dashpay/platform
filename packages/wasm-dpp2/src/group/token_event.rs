use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use dpp::tokens::token_event::TokenEvent;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * TokenEvent serialized as a plain object.
 */
export interface TokenEventObject {
    variant: TokenEventVariant;
    [key: string]: unknown;
}

/**
 * TokenEvent serialized as JSON.
 */
export interface TokenEventJSON {
    variant: number;
    [key: string]: unknown;
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

impl_wasm_conversions!(
    TokenEventWasm,
    TokenEvent,
    TokenEventObjectJs,
    TokenEventJSONJs
);
impl_wasm_type_info!(TokenEventWasm, TokenEvent);
