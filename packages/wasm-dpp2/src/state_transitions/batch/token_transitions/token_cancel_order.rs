use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_string};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_cancel_order_transition::v0::v0_methods::TokenCancelOrderTransitionV0Methods;
use dpp::state_transition::batch_transition::token_cancel_order_transition::TokenCancelOrderTransitionV0;
use dpp::state_transition::batch_transition::TokenCancelOrderTransition;
use dpp::tokens::token_order_side::TokenOrderSide;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_CANCEL_ORDER_OPTIONS_TS: &str = r#"
/** The side of a token marketplace order: a listing (`ask`) or an offer (`bid`). */
export type TokenOrderSide = "ask" | "bid";

/** Cancels the sender's open order `orderId` and returns what it holds in escrow. */
export interface TokenCancelOrderTransitionOptions {
    base: TokenBaseTransition;
    orderId: IdentifierLike;
    side: TokenOrderSide;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenCancelOrderTransitionOptions")]
    pub type TokenCancelOrderTransitionOptionsJs;
}

fn order_side_from_str(side: &str) -> WasmDppResult<TokenOrderSide> {
    match side {
        "ask" => Ok(TokenOrderSide::Ask),
        "bid" => Ok(TokenOrderSide::Bid),
        other => Err(WasmDppError::invalid_argument(format!(
            "'side' must be \"ask\" or \"bid\", got \"{}\"",
            other
        ))),
    }
}

fn order_side_to_str(side: TokenOrderSide) -> &'static str {
    match side {
        TokenOrderSide::Ask => "ask",
        TokenOrderSide::Bid => "bid",
    }
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenCancelOrderTransition")]
pub struct TokenCancelOrderTransitionWasm(TokenCancelOrderTransition);

impl From<TokenCancelOrderTransitionWasm> for TokenCancelOrderTransition {
    fn from(transition: TokenCancelOrderTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenCancelOrderTransition> for TokenCancelOrderTransitionWasm {
    fn from(transition: TokenCancelOrderTransition) -> Self {
        TokenCancelOrderTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenCancelOrderTransition)]
impl TokenCancelOrderTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenCancelOrderTransitionOptionsJs,
    ) -> WasmDppResult<TokenCancelOrderTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let order_id: IdentifierWasm = try_from_options(&options, "orderId")?;

        let side = try_from_options_with(&options, "side", |v| {
            order_side_from_str(&try_to_string(v, "side")?)
        })?;

        Ok(TokenCancelOrderTransitionWasm(
            TokenCancelOrderTransition::V0(TokenCancelOrderTransitionV0 {
                base: base.into(),
                order_id: order_id.into(),
                side,
            }),
        ))
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = orderId)]
    pub fn order_id(&self) -> IdentifierWasm {
        self.0.order_id().into()
    }

    #[wasm_bindgen(getter = side, unchecked_return_type = "TokenOrderSide")]
    pub fn side(&self) -> String {
        order_side_to_str(self.0.side()).to_string()
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

    #[wasm_bindgen(setter = side)]
    pub fn set_side(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "TokenOrderSide")] side: String,
    ) -> WasmDppResult<()> {
        self.0.set_side(order_side_from_str(&side)?);
        Ok(())
    }
}

impl_try_from_js_value!(TokenCancelOrderTransitionWasm, "TokenCancelOrderTransition");
impl_wasm_type_info!(TokenCancelOrderTransitionWasm, TokenCancelOrderTransition);
