use crate::enums::batch::gas_fees_paid_by::{GasFeesPaidByLikeJs, GasFeesPaidByWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeOrUndefinedJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_serde;
use crate::impl_wasm_type_info;
use crate::utils::try_from_options_optional;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::TokenContractPosition;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenPaymentInfoOptions {
    token_contract_position: TokenContractPosition,
    #[serde(default)]
    minimum_token_cost: Option<TokenAmount>,
    #[serde(default)]
    maximum_token_cost: Option<TokenAmount>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface TokenPaymentInfoOptions {
    paymentTokenContractId?: IdentifierLike;
    tokenContractPosition: number;
    minimumTokenCost?: bigint;
    maximumTokenCost?: bigint;
    gasFeesPaidBy?: GasFeesPaidByLike;
}

/**
 * TokenPaymentInfo serialized as a plain object.
 */
export interface TokenPaymentInfoObject {
    $formatVersion: string;
    paymentTokenContractId: Uint8Array | null;
    tokenContractPosition: number;
    minimumTokenCost: bigint | null;
    maximumTokenCost: bigint | null;
    gasFeesPaidBy: string;
}

/**
 * TokenPaymentInfo serialized as JSON.
 */
export interface TokenPaymentInfoJSON {
    $formatVersion: string;
    paymentTokenContractId: string | null;
    tokenContractPosition: number;
    minimumTokenCost: number | string | null;
    maximumTokenCost: number | string | null;
    gasFeesPaidBy: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenPaymentInfoOptions")]
    pub type TokenPaymentInfoOptionsJs;

    #[wasm_bindgen(typescript_type = "TokenPaymentInfoObject")]
    pub type TokenPaymentInfoObjectJs;

    #[wasm_bindgen(typescript_type = "TokenPaymentInfoJSON")]
    pub type TokenPaymentInfoJSONJs;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "TokenPaymentInfo")]
pub struct TokenPaymentInfoWasm(TokenPaymentInfo);

impl From<TokenPaymentInfo> for TokenPaymentInfoWasm {
    fn from(info: TokenPaymentInfo) -> Self {
        TokenPaymentInfoWasm(info)
    }
}

impl From<TokenPaymentInfoWasm> for TokenPaymentInfo {
    fn from(info: TokenPaymentInfoWasm) -> Self {
        info.0
    }
}

#[wasm_bindgen(js_class = TokenPaymentInfo)]
impl TokenPaymentInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: TokenPaymentInfoOptionsJs) -> WasmDppResult<Self> {
        // Extract complex types first (borrows &options)
        let payment_token_contract_id: Option<IdentifierWasm> =
            try_from_options_optional(&options, "paymentTokenContractId")?;

        let gas_fees_paid_by: GasFeesPaidBy =
            try_from_options_optional::<GasFeesPaidByWasm>(&options, "gasFeesPaidBy")?
                .map(Into::into)
                .unwrap_or_default();

        // Deserialize primitive fields via serde last (consumes options)
        let opts: TokenPaymentInfoOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(TokenPaymentInfoWasm(TokenPaymentInfo::V0(
            TokenPaymentInfoV0 {
                payment_token_contract_id: payment_token_contract_id.map(Into::into),
                token_contract_position: opts.token_contract_position,
                minimum_token_cost: opts.minimum_token_cost,
                maximum_token_cost: opts.maximum_token_cost,
                gas_fees_paid_by,
            },
        )))
    }

    #[wasm_bindgen(getter = "paymentTokenContractId")]
    pub fn payment_token_contract_id(&self) -> Option<IdentifierWasm> {
        self.0.payment_token_contract_id().map(|id| id.into())
    }

    #[wasm_bindgen(getter = "tokenContractPosition")]
    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.0.token_contract_position()
    }

    #[wasm_bindgen(getter = "minimumTokenCost")]
    pub fn minimum_token_cost(&self) -> Option<TokenAmount> {
        self.0.minimum_token_cost()
    }

    #[wasm_bindgen(getter = "maximumTokenCost")]
    pub fn maximum_token_cost(&self) -> Option<TokenAmount> {
        self.0.maximum_token_cost()
    }

    #[wasm_bindgen(getter = "gasFeesPaidBy")]
    pub fn gas_fees_paid_by(&self) -> String {
        GasFeesPaidByWasm::from(self.0.gas_fees_paid_by()).into()
    }

    #[wasm_bindgen(setter = "paymentTokenContractId")]
    pub fn set_payment_token_contract_id(
        &mut self,
        payment_token_contract_id: IdentifierLikeOrUndefinedJs,
    ) -> WasmDppResult<()> {
        self.0
            .set_payment_token_contract_id(payment_token_contract_id.try_into()?);

        Ok(())
    }

    #[wasm_bindgen(setter = "tokenContractPosition")]
    pub fn set_token_contract_position(
        &mut self,
        #[wasm_bindgen(js_name = "tokenContractPosition")]
        token_contract_position: TokenContractPosition,
    ) {
        self.0.set_token_contract_position(token_contract_position)
    }

    #[wasm_bindgen(setter = "minimumTokenCost")]
    pub fn set_minimum_token_cost(
        &mut self,
        #[wasm_bindgen(js_name = "minimumCost")] minimum_cost: Option<TokenAmount>,
    ) {
        self.0.set_minimum_token_cost(minimum_cost);
    }

    #[wasm_bindgen(setter = "maximumTokenCost")]
    pub fn set_maximum_token_cost(
        &mut self,
        #[wasm_bindgen(js_name = "maximumCost")] maximum_cost: Option<TokenAmount>,
    ) {
        self.0.set_maximum_token_cost(maximum_cost)
    }

    #[wasm_bindgen(setter = "gasFeesPaidBy")]
    pub fn set_gas_fees_paid_by(
        &mut self,
        #[wasm_bindgen(js_name = "gasFeesPaidBy")] gas_fees_paid_by: GasFeesPaidByLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_gas_fees_paid_by(gas_fees_paid_by.try_into()?);
        Ok(())
    }
}

impl_wasm_conversions_serde!(
    TokenPaymentInfoWasm,
    TokenPaymentInfo,
    TokenPaymentInfoObjectJs,
    TokenPaymentInfoJSONJs
);
impl_try_from_js_value!(TokenPaymentInfoWasm, "TokenPaymentInfo");
impl_wasm_type_info!(TokenPaymentInfoWasm, TokenPaymentInfo);
