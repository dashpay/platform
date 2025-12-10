use crate::enums::batch::gas_fees_paid_by::GasFeesPaidByWasm;
use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::TokenContractPosition;
use dpp::prelude::Identifier;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenPaymentInfo".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenPaymentInfo".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_payment_token_contract_id: &JsValue,
        token_contract_position: TokenContractPosition,
        minimum_token_cost: Option<TokenAmount>,
        maximum_token_cost: Option<TokenAmount>,
        js_gas_fees_paid_by: &JsValue,
    ) -> WasmDppResult<Self> {
        let payment_token_contract_id: Option<Identifier> = match js_payment_token_contract_id
            .is_null()
            | js_payment_token_contract_id.is_undefined()
        {
            true => None,
            false => Some(IdentifierWasm::try_from(js_payment_token_contract_id)?.into()),
        };

        let gas_fees_paid_by =
            match js_gas_fees_paid_by.is_undefined() | js_gas_fees_paid_by.is_null() {
                true => GasFeesPaidBy::default(),
                false => GasFeesPaidByWasm::try_from(js_gas_fees_paid_by.clone())?
                    .clone()
                    .into(),
            };

        Ok(TokenPaymentInfoWasm(TokenPaymentInfo::V0(
            TokenPaymentInfoV0 {
                payment_token_contract_id,
                token_contract_position,
                minimum_token_cost,
                maximum_token_cost,
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
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_payment_token_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        let payment_token_contract_id: Option<Identifier> = match js_payment_token_contract_id
            .is_null()
            | js_payment_token_contract_id.is_undefined()
        {
            true => None,
            false => Some(IdentifierWasm::try_from(js_payment_token_contract_id)?.into()),
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
    pub fn set_gas_fees_paid_by(&mut self, js_gas_fees_paid_by: &JsValue) -> WasmDppResult<()> {
        let gas_fees_paid_by =
            match js_gas_fees_paid_by.is_undefined() | js_gas_fees_paid_by.is_null() {
                true => GasFeesPaidBy::default(),
                false => GasFeesPaidByWasm::try_from(js_gas_fees_paid_by.clone())?
                    .clone()
                    .into(),
            };

        self.0.set_gas_fees_paid_by(gas_fees_paid_by);

        Ok(())
    }
}
