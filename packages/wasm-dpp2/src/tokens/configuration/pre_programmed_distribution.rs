use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{JsValueExt, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
use dpp::prelude::{Identifier, TimestampMillis};
use js_sys::{BigInt, Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenPreProgrammedDistribution")]
pub struct TokenPreProgrammedDistributionWasm(TokenPreProgrammedDistribution);

impl From<TokenPreProgrammedDistributionWasm> for TokenPreProgrammedDistribution {
    fn from(value: TokenPreProgrammedDistributionWasm) -> Self {
        value.0
    }
}

impl From<TokenPreProgrammedDistribution> for TokenPreProgrammedDistributionWasm {
    fn from(value: TokenPreProgrammedDistribution) -> Self {
        TokenPreProgrammedDistributionWasm(value)
    }
}

pub fn js_distributions_to_distributions(
    js_distributions: &JsValue,
) -> WasmDppResult<BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>> {
    let distributions_object = Object::from(js_distributions.clone());
    let distributions_keys = Object::keys(&distributions_object);

    let mut distributions = BTreeMap::new();

    for key in distributions_keys.iter() {
        let timestamp_str = key.as_string().ok_or_else(|| {
            WasmDppError::invalid_argument("Cannot read timestamp in distribution rules")
        })?;

        let timestamp = timestamp_str.parse::<TimestampMillis>().map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "Invalid timestamp '{}': {}",
                timestamp_str, err
            ))
        })?;

        let identifiers_js = Reflect::get(&distributions_object, &key).map_err(|err| {
            let message = err.error_message();
            WasmDppError::invalid_argument(format!(
                "unable to access distribution entry for timestamp '{}': {}",
                timestamp_str, message
            ))
        })?;
        let identifiers_object = Object::from(identifiers_js);
        let identifiers_keys = Object::keys(&identifiers_object);

        let mut ids = BTreeMap::new();

        for id_key in identifiers_keys.iter() {
            let identifier = Identifier::from(IdentifierWasm::try_from(id_key.clone())?);

            let amount_js = Reflect::get(&identifiers_object, &id_key).map_err(|err| {
                let message = err.error_message();
                WasmDppError::invalid_argument(format!(
                    "unable to access distribution amount for identity '{}' at '{}': {}",
                    identifier, timestamp, message
                ))
            })?;

            let token_amount = try_to_u64(amount_js)
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

            ids.insert(identifier, token_amount);
        }

        distributions.insert(timestamp, ids);
    }

    Ok(distributions)
}

#[wasm_bindgen(js_class = TokenPreProgrammedDistribution)]
impl TokenPreProgrammedDistributionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenPreProgrammedDistribution".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenPreProgrammedDistribution".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_distributions: &JsValue) -> WasmDppResult<TokenPreProgrammedDistributionWasm> {
        let distributions = js_distributions_to_distributions(js_distributions)?;

        Ok(TokenPreProgrammedDistributionWasm(
            TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions }),
        ))
    }

    #[wasm_bindgen(getter = "distributions")]
    pub fn get_distributions(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();

        for (key, value) in self.0.distributions() {
            let identifiers_obj = Object::new();
            // &BigInt::from(key.clone()).into()

            for (identifiers_key, identifiers_value) in value {
                Reflect::set(
                    &identifiers_obj,
                    &IdentifierWasm::from(*identifiers_key)
                        .get_base58()
                        .into(),
                    &BigInt::from(*identifiers_value).into(),
                )
                .map_err(|err| {
                    let message = err.error_message();
                    WasmDppError::generic(format!(
                        "unable to serialize distribution amount for identity '{}' at '{}': {}",
                        identifiers_key, key, message
                    ))
                })?;
            }

            Reflect::set(&obj, &key.to_string().into(), &identifiers_obj.into()).map_err(
                |err| {
                    let message = err.error_message();
                    WasmDppError::generic(format!(
                        "unable to serialize distribution for timestamp '{}': {}",
                        key, message
                    ))
                },
            )?;
        }

        Ok(obj.into())
    }

    #[wasm_bindgen(setter = "distributions")]
    pub fn set_distributions(&mut self, js_distributions: &JsValue) -> WasmDppResult<()> {
        let distributions = js_distributions_to_distributions(js_distributions)?;

        self.0.set_distributions(distributions);

        Ok(())
    }
}
