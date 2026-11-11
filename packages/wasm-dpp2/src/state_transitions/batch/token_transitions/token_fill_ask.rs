use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_fill_ask_transition::v0::v0_methods::TokenFillAskTransitionV0Methods;
use dpp::state_transition::batch_transition::token_fill_ask_transition::TokenFillAskTransitionV0;
use dpp::state_transition::batch_transition::TokenFillAskTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_FILL_ASK_OPTIONS_TS: &str = r#"
/**
 * Buys from the listing `orderId`: between `minAmount` and `maxAmount` base units, for at most
 * `maxTotalPrice` credits in all. The fill takes as much as the range and the listing allow.
 */
export interface TokenFillAskTransitionOptions {
    base: TokenBaseTransition;
    orderId: IdentifierLike;
    minAmount: bigint;
    maxAmount: bigint;
    maxTotalPrice: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenFillAskTransitionOptions")]
    pub type TokenFillAskTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenFillAskTransition")]
pub struct TokenFillAskTransitionWasm(TokenFillAskTransition);

impl From<TokenFillAskTransitionWasm> for TokenFillAskTransition {
    fn from(transition: TokenFillAskTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenFillAskTransition> for TokenFillAskTransitionWasm {
    fn from(transition: TokenFillAskTransition) -> Self {
        TokenFillAskTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenFillAskTransition)]
impl TokenFillAskTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenFillAskTransitionOptionsJs,
    ) -> WasmDppResult<TokenFillAskTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let order_id: IdentifierWasm = try_from_options(&options, "orderId")?;

        let min_amount: TokenAmount =
            try_from_options_with(&options, "minAmount", |v| try_to_u64(v, "minAmount"))?;

        let max_amount: TokenAmount =
            try_from_options_with(&options, "maxAmount", |v| try_to_u64(v, "maxAmount"))?;

        let max_total_price: Credits = try_from_options_with(&options, "maxTotalPrice", |v| {
            try_to_u64(v, "maxTotalPrice")
        })?;

        Ok(TokenFillAskTransitionWasm(TokenFillAskTransition::V0(
            TokenFillAskTransitionV0 {
                base: base.into(),
                order_id: order_id.into(),
                min_amount,
                max_amount,
                max_total_price,
            },
        )))
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = orderId)]
    pub fn order_id(&self) -> IdentifierWasm {
        self.0.order_id().into()
    }

    #[wasm_bindgen(getter = minAmount)]
    pub fn min_amount(&self) -> TokenAmount {
        self.0.min_amount()
    }

    #[wasm_bindgen(getter = maxAmount)]
    pub fn max_amount(&self) -> TokenAmount {
        self.0.max_amount()
    }

    #[wasm_bindgen(getter = maxTotalPrice)]
    pub fn max_total_price(&self) -> Credits {
        self.0.max_total_price()
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = orderId)]
    pub fn set_order_id(
        &mut self,
        #[wasm_bindgen(js_name = "orderId")] order_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_order_id(order_id.try_into()?);
        Ok(())
    }

    // Each integer setter converts with `try_to_u64` and leaves the transition unchanged when the
    // value is out of range, rather than letting the ABI narrow it to another valid term.
    #[wasm_bindgen(setter = minAmount)]
    pub fn set_min_amount(
        &mut self,
        #[wasm_bindgen(js_name = "minAmount")] min_amount: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.0.set_min_amount(try_to_u64(min_amount, "minAmount")?);
        Ok(())
    }

    #[wasm_bindgen(setter = maxAmount)]
    pub fn set_max_amount(
        &mut self,
        #[wasm_bindgen(js_name = "maxAmount")] max_amount: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.0.set_max_amount(try_to_u64(max_amount, "maxAmount")?);
        Ok(())
    }

    #[wasm_bindgen(setter = maxTotalPrice)]
    pub fn set_max_total_price(
        &mut self,
        #[wasm_bindgen(js_name = "maxTotalPrice")] max_total_price: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.0
            .set_max_total_price(try_to_u64(max_total_price, "maxTotalPrice")?);
        Ok(())
    }
}

impl_try_from_js_value!(TokenFillAskTransitionWasm, "TokenFillAskTransition");
impl_wasm_type_info!(TokenFillAskTransitionWasm, TokenFillAskTransition);
