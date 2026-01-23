use crate::enums::batch::gas_fees_paid_by::{GasFeesPaidByLikeJs, GasFeesPaidByWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::utils::{get_optional_property, get_optional_property_with};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::TokenContractPosition;
use dpp::prelude::Identifier;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use js_sys::Object;
use serde::Deserialize;
use wasm_bindgen::JsValue;
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
const TS_TYPES: &'static str = r#"
export interface TokenPaymentInfoOptions {
    paymentTokenContractId?: IdentifierLike | null;
    tokenContractPosition: number;
    minimumTokenCost?: bigint;
    maximumTokenCost?: bigint;
    gasFeesPaidBy?: GasFeesPaidBy | string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenPaymentInfoOptions")]
    pub type TokenPaymentInfoOptionsJs;
}

#[derive(Clone)]
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
        let options: JsValue = options.into();
        let object = Object::from(options.clone());

        // Extract paymentTokenContractId (optional, can be null/undefined)
        let payment_token_contract_id: Option<Identifier> =
            get_optional_property::<IdentifierWasm>(&object, "paymentTokenContractId")?
                .map(Into::into);

        // Extract gasFeesPaidBy (optional)
        let gas_fees_paid_by: GasFeesPaidBy =
            get_optional_property_with(&object, "gasFeesPaidBy", |v| {
                GasFeesPaidByWasm::try_from(v).map(|g| g.into())
            })?
            .unwrap_or_default();

        // Extract simple fields via serde
        let opts: TokenPaymentInfoOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(TokenPaymentInfoWasm(TokenPaymentInfo::V0(
            TokenPaymentInfoV0 {
                payment_token_contract_id,
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
        payment_token_contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let id_value: JsValue = payment_token_contract_id.into();
        let payment_token_contract_id: Option<Identifier> =
            if id_value.is_null() || id_value.is_undefined() {
                None
            } else {
                Some(IdentifierWasm::try_from(&id_value)?.into())
            };

        self.0
            .set_payment_token_contract_id(payment_token_contract_id);

        Ok(())
    }

    #[wasm_bindgen(setter = "tokenContractPosition")]
    pub fn set_token_contract_position(&mut self, token_contract_position: TokenContractPosition) {
        self.0.set_token_contract_position(token_contract_position)
    }

    #[wasm_bindgen(setter = "minimumTokenCost")]
    pub fn set_minimum_token_cost(&mut self, minimum_cost: Option<TokenAmount>) {
        self.0.set_maximum_token_cost(minimum_cost);
    }

    #[wasm_bindgen(setter = "maximumTokenCost")]
    pub fn set_maximum_token_cost(&mut self, maximum_cost: Option<TokenAmount>) {
        self.0.set_maximum_token_cost(maximum_cost)
    }

    #[wasm_bindgen(setter = "gasFeesPaidBy")]
    pub fn set_gas_fees_paid_by(
        &mut self,
        gas_fees_paid_by: GasFeesPaidByLikeJs,
    ) -> WasmDppResult<()> {
        let value: JsValue = gas_fees_paid_by.into();
        let gas_fees_paid_by = if value.is_undefined() || value.is_null() {
            GasFeesPaidBy::default()
        } else {
            GasFeesPaidByWasm::try_from(value)?.into()
        };

        self.0.set_gas_fees_paid_by(gas_fees_paid_by);

        Ok(())
    }
}

impl_wasm_type_info!(TokenPaymentInfoWasm, TokenPaymentInfo);
