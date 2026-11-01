use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_place_bid_transition::v0::v0_methods::TokenPlaceBidTransitionV0Methods;
use dpp::state_transition::batch_transition::token_place_bid_transition::TokenPlaceBidTransitionV0;
use dpp::state_transition::batch_transition::TokenPlaceBidTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_PLACE_BID_OPTIONS_TS: &str = r#"
/**
 * Offers `price` credits per whole token for `amount` of the token, at the token's decimals when
 * the offer is placed, which it keeps. What the offer is worth at them moves from the buyer's
 * credits into escrow until it is filled or cancelled. A sale takes at least the token's minimum
 * fill, unless it takes everything still wanted; `amount` must be at least that minimum.
 */
export interface TokenPlaceBidTransitionOptions {
    base: TokenBaseTransition;
    amount: bigint;
    price: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenPlaceBidTransitionOptions")]
    pub type TokenPlaceBidTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenPlaceBidTransition")]
pub struct TokenPlaceBidTransitionWasm(TokenPlaceBidTransition);

impl From<TokenPlaceBidTransitionWasm> for TokenPlaceBidTransition {
    fn from(transition: TokenPlaceBidTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenPlaceBidTransition> for TokenPlaceBidTransitionWasm {
    fn from(transition: TokenPlaceBidTransition) -> Self {
        TokenPlaceBidTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenPlaceBidTransition)]
impl TokenPlaceBidTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenPlaceBidTransitionOptionsJs,
    ) -> WasmDppResult<TokenPlaceBidTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let amount: TokenAmount =
            try_from_options_with(&options, "amount", |v| try_to_u64(v, "amount"))?;

        let price: Credits = try_from_options_with(&options, "price", |v| try_to_u64(v, "price"))?;

        Ok(TokenPlaceBidTransitionWasm(TokenPlaceBidTransition::V0(
            TokenPlaceBidTransitionV0 {
                base: base.into(),
                amount,
                price,
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
}

impl_try_from_js_value!(TokenPlaceBidTransitionWasm, "TokenPlaceBidTransition");
impl_wasm_type_info!(TokenPlaceBidTransitionWasm, TokenPlaceBidTransition);
