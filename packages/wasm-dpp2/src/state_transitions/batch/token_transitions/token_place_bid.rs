use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_from_options_with, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_place_bid_transition::v0::v0_methods::TokenPlaceBidTransitionV0Methods;
use dpp::state_transition::batch_transition::token_place_bid_transition::TokenPlaceBidTransitionV0;
use dpp::state_transition::batch_transition::TokenPlaceBidTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_PLACE_BID_OPTIONS_TS: &str = r#"
/**
 * Offers `price` credits per whole token for `amount` of the token. What the offer is worth moves
 * from the buyer's credits into escrow until it is filled or cancelled. A fill takes at least
 * `minFill`, unless it takes everything still wanted. An offer with `expiresAt` can no longer be
 * filled from that block time.
 */
export interface TokenPlaceBidTransitionOptions {
    base: TokenBaseTransition;
    amount: bigint;
    price: bigint;
    minFill: bigint;
    expiresAt?: bigint;
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

        let min_fill: TokenAmount =
            try_from_options_with(&options, "minFill", |v| try_to_u64(v, "minFill"))?;

        let expires_at: Option<TimestampMillis> =
            try_from_options_optional_with(&options, "expiresAt", |v| try_to_u64(v, "expiresAt"))?;

        Ok(TokenPlaceBidTransitionWasm(TokenPlaceBidTransition::V0(
            TokenPlaceBidTransitionV0 {
                base: base.into(),
                amount,
                price,
                min_fill,
                expires_at,
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

    #[wasm_bindgen(getter = minFill)]
    pub fn min_fill(&self) -> TokenAmount {
        self.0.min_fill()
    }

    #[wasm_bindgen(getter = expiresAt)]
    pub fn expires_at(&self) -> Option<TimestampMillis> {
        self.0.expires_at()
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

    #[wasm_bindgen(setter = minFill)]
    pub fn set_min_fill(
        &mut self,
        #[wasm_bindgen(js_name = "minFill")] min_fill: &js_sys::BigInt,
    ) -> WasmDppResult<()> {
        self.0.set_min_fill(try_to_u64(min_fill, "minFill")?);
        Ok(())
    }

    #[wasm_bindgen(setter = expiresAt)]
    pub fn set_expires_at(
        &mut self,
        #[wasm_bindgen(js_name = "expiresAt")] expires_at: Option<js_sys::BigInt>,
    ) -> WasmDppResult<()> {
        let expires_at = expires_at
            .map(|value| try_to_u64(&value, "expiresAt"))
            .transpose()?;
        self.0.set_expires_at(expires_at);
        Ok(())
    }
}

impl_try_from_js_value!(TokenPlaceBidTransitionWasm, "TokenPlaceBidTransition");
impl_wasm_type_info!(TokenPlaceBidTransitionWasm, TokenPlaceBidTransition);
