use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::utils::try_to_u64;
use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
use dpp::data_contract::associated_token::token_once_per_identity_distribution::accessors::v0::TokenOncePerIdentityDistributionV0Methods;
use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// A distribution every identity may claim exactly once: a fixed `amount` minted to the claimant
/// on its first claim of type `OncePerIdentity`. Bounded only by the token's max supply.
#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenOncePerIdentityDistribution")]
pub struct TokenOncePerIdentityDistributionWasm(TokenOncePerIdentityDistribution);

impl From<TokenOncePerIdentityDistributionWasm> for TokenOncePerIdentityDistribution {
    fn from(value: TokenOncePerIdentityDistributionWasm) -> Self {
        value.0
    }
}

impl From<TokenOncePerIdentityDistribution> for TokenOncePerIdentityDistributionWasm {
    fn from(value: TokenOncePerIdentityDistribution) -> Self {
        TokenOncePerIdentityDistributionWasm(value)
    }
}

#[wasm_bindgen(js_class = TokenOncePerIdentityDistribution)]
impl TokenOncePerIdentityDistributionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(amount: JsValue) -> WasmDppResult<TokenOncePerIdentityDistributionWasm> {
        let amount = try_to_u64(&amount, "amount")?;

        Ok(TokenOncePerIdentityDistributionWasm(
            TokenOncePerIdentityDistribution::V0(TokenOncePerIdentityDistributionV0 { amount }),
        ))
    }

    #[wasm_bindgen(getter = "amount")]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: JsValue) -> WasmDppResult<()> {
        let amount = try_to_u64(&amount, "amount")?;

        self.0.set_amount(amount);

        Ok(())
    }
}

impl_try_from_js_value!(
    TokenOncePerIdentityDistributionWasm,
    "TokenOncePerIdentityDistribution"
);
impl_wasm_type_info!(
    TokenOncePerIdentityDistributionWasm,
    TokenOncePerIdentityDistribution
);
