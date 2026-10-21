use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_fill_bid_transition::v0::v0_methods::TokenFillBidTransitionV0Methods;
use dpp::state_transition::batch_transition::token_fill_bid_transition::TokenFillBidTransitionV0;
use dpp::state_transition::batch_transition::TokenFillBidTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_FILL_BID_OPTIONS_TS: &str = r#"
/**
 * Sells into the offer `orderId`: between `minAmount` and `maxAmount` base units, for at least
 * `minTotalReceived` credits in all. The fill takes as much as the range and the offer allow.
 */
export interface TokenFillBidTransitionOptions {
    base: TokenBaseTransition;
    orderId: IdentifierLike;
    minAmount: bigint;
    maxAmount: bigint;
    minTotalReceived: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenFillBidTransitionOptions")]
    pub type TokenFillBidTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenFillBidTransition")]
pub struct TokenFillBidTransitionWasm(TokenFillBidTransition);

impl From<TokenFillBidTransitionWasm> for TokenFillBidTransition {
    fn from(transition: TokenFillBidTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenFillBidTransition> for TokenFillBidTransitionWasm {
    fn from(transition: TokenFillBidTransition) -> Self {
        TokenFillBidTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenFillBidTransition)]
impl TokenFillBidTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenFillBidTransitionOptionsJs,
    ) -> WasmDppResult<TokenFillBidTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let order_id: IdentifierWasm = try_from_options(&options, "orderId")?;

        let min_amount: TokenAmount =
            try_from_options_with(&options, "minAmount", |v| try_to_u64(v, "minAmount"))?;

        let max_amount: TokenAmount =
            try_from_options_with(&options, "maxAmount", |v| try_to_u64(v, "maxAmount"))?;

        let min_total_received: Credits =
            try_from_options_with(&options, "minTotalReceived", |v| {
                try_to_u64(v, "minTotalReceived")
            })?;

        Ok(TokenFillBidTransitionWasm(TokenFillBidTransition::V0(
            TokenFillBidTransitionV0 {
                base: base.into(),
                order_id: order_id.into(),
                min_amount,
                max_amount,
                min_total_received,
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

    #[wasm_bindgen(getter = minTotalReceived)]
    pub fn min_total_received(&self) -> Credits {
        self.0.min_total_received()
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

    #[wasm_bindgen(setter = minAmount)]
    pub fn set_min_amount(
        &mut self,
        #[wasm_bindgen(js_name = "minAmount")] min_amount: TokenAmount,
    ) {
        self.0.set_min_amount(min_amount)
    }

    #[wasm_bindgen(setter = maxAmount)]
    pub fn set_max_amount(
        &mut self,
        #[wasm_bindgen(js_name = "maxAmount")] max_amount: TokenAmount,
    ) {
        self.0.set_max_amount(max_amount)
    }

    #[wasm_bindgen(setter = minTotalReceived)]
    pub fn set_min_total_received(
        &mut self,
        #[wasm_bindgen(js_name = "minTotalReceived")] min_total_received: Credits,
    ) {
        self.0.set_min_total_received(min_total_received)
    }
}

impl_try_from_js_value!(TokenFillBidTransitionWasm, "TokenFillBidTransition");
impl_wasm_type_info!(TokenFillBidTransitionWasm, TokenFillBidTransition);
