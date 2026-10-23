use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_u64, try_to_u8};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_place_ask_transition::v0::v0_methods::TokenPlaceAskTransitionV0Methods;
use dpp::state_transition::batch_transition::token_place_ask_transition::TokenPlaceAskTransitionV0;
use dpp::state_transition::batch_transition::TokenPlaceAskTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_PLACE_ASK_OPTIONS_TS: &str = r#"
/**
 * Lists `amount` of the token for sale at `price` credits per whole token. The tokens move into
 * escrow until the listing is filled or cancelled. A fill takes at least `minFill`, unless it
 * takes everything left.
 */
export interface TokenPlaceAskTransitionOptions {
    base: TokenBaseTransition;
    amount: bigint;
    price: bigint;
    /** The token's decimals the price is at; the listing is refused unless they are the token's. */
    decimals: number;
    minFill: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenPlaceAskTransitionOptions")]
    pub type TokenPlaceAskTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenPlaceAskTransition")]
pub struct TokenPlaceAskTransitionWasm(TokenPlaceAskTransition);

impl From<TokenPlaceAskTransitionWasm> for TokenPlaceAskTransition {
    fn from(transition: TokenPlaceAskTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenPlaceAskTransition> for TokenPlaceAskTransitionWasm {
    fn from(transition: TokenPlaceAskTransition) -> Self {
        TokenPlaceAskTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenPlaceAskTransition)]
impl TokenPlaceAskTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenPlaceAskTransitionOptionsJs,
    ) -> WasmDppResult<TokenPlaceAskTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let amount: TokenAmount =
            try_from_options_with(&options, "amount", |v| try_to_u64(v, "amount"))?;

        let price: Credits = try_from_options_with(&options, "price", |v| try_to_u64(v, "price"))?;

        let decimals: u8 =
            try_from_options_with(&options, "decimals", |v| try_to_u8(v, "decimals"))?;

        let min_fill: TokenAmount =
            try_from_options_with(&options, "minFill", |v| try_to_u64(v, "minFill"))?;

        Ok(TokenPlaceAskTransitionWasm(TokenPlaceAskTransition::V0(
            TokenPlaceAskTransitionV0 {
                base: base.into(),
                amount,
                price,
                decimals,
                min_fill,
            },
        )))
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = amount)]
    pub fn amount(&self) -> TokenAmount {
        self.0.amount()
    }

    #[wasm_bindgen(getter = price)]
    pub fn price(&self) -> Credits {
        self.0.price()
    }

    #[wasm_bindgen(getter = decimals)]
    pub fn decimals(&self) -> u8 {
        self.0.decimals()
    }

    #[wasm_bindgen(getter = minFill)]
    pub fn min_fill(&self) -> TokenAmount {
        self.0.min_fill()
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    // Each integer setter converts with `try_to_u64` and leaves the transition unchanged when the
    // value is out of range, rather than letting the ABI narrow it to another valid term.
    #[wasm_bindgen(setter = amount)]
    pub fn set_amount(&mut self, amount: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_amount(try_to_u64(amount, "amount")?);
        Ok(())
    }

    #[wasm_bindgen(setter = price)]
    pub fn set_price(&mut self, price: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_price(try_to_u64(price, "price")?);
        Ok(())
    }

    #[wasm_bindgen(setter = decimals)]
    pub fn set_decimals(&mut self, decimals: &js_sys::Number) -> WasmDppResult<()> {
        self.0.set_decimals(try_to_u8(decimals, "decimals")?);
        Ok(())
    }

    #[wasm_bindgen(setter = minFill)]
    pub fn set_min_fill(
        &mut self,
        #[wasm_bindgen(js_name = "minFill")] min_fill: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.0.set_min_fill(try_to_u64(min_fill, "minFill")?);
        Ok(())
    }
}

impl_try_from_js_value!(TokenPlaceAskTransitionWasm, "TokenPlaceAskTransition");
impl_wasm_type_info!(TokenPlaceAskTransitionWasm, TokenPlaceAskTransition);
