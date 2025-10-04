use crate::identifier::IdentifierWASM;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
use dpp::prelude::{Identifier, TimestampMillis};
use js_sys::{BigInt, Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenPreProgrammedDistributionWASM")]
pub struct TokenPreProgrammedDistributionWASM(TokenPreProgrammedDistribution);

impl From<TokenPreProgrammedDistributionWASM> for TokenPreProgrammedDistribution {
    fn from(value: TokenPreProgrammedDistributionWASM) -> Self {
        value.0
    }
}

impl From<TokenPreProgrammedDistribution> for TokenPreProgrammedDistributionWASM {
    fn from(value: TokenPreProgrammedDistribution) -> Self {
        TokenPreProgrammedDistributionWASM(value)
    }
}

pub fn js_distributions_to_distributions(
    js_distributions: &JsValue,
) -> Result<BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>, JsValue> {
    let distributions_object = Object::from(js_distributions.clone());
    let distributions_keys = Object::keys(&distributions_object);

    let mut distributions = BTreeMap::new();

    for key in distributions_keys.iter() {
        let timestamp = match key.as_string() {
            None => Err(JsValue::from("Cannot read timestamp in distribution rules")),
            Some(timestamp) => Ok(timestamp
                .parse::<TimestampMillis>()
                .map_err(JsError::from)?),
        }?;

        let identifiers_object = Object::from(Reflect::get(&distributions_object, &key)?.clone());
        let identifiers_keys = Object::keys(&identifiers_object);

        let mut ids = BTreeMap::new();

        for id_key in identifiers_keys.iter() {
            let identifier = Identifier::from(IdentifierWASM::try_from(id_key.clone())?);

            let token_amount = BigInt::new(&Reflect::get(&identifiers_object, &id_key.clone())?)?
                .to_string(10)
                .map_err(|err| JsValue::from(format!("bigint to string: {}", err.to_string())))?
                .as_string()
                .ok_or_else(|| JsValue::from_str("Failed to convert BigInt to string"))?
                .parse::<TokenAmount>()
                .map_err(JsError::from)?;

            ids.insert(identifier, token_amount);
        }

        distributions.insert(timestamp, ids);
    }

    Ok(distributions)
}

#[wasm_bindgen]
impl TokenPreProgrammedDistributionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenPreProgrammedDistributionWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenPreProgrammedDistributionWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_distributions: &JsValue) -> Result<TokenPreProgrammedDistributionWASM, JsValue> {
        let distributions = js_distributions_to_distributions(js_distributions)?;

        Ok(TokenPreProgrammedDistributionWASM(
            TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions }),
        ))
    }

    #[wasm_bindgen(getter = "distributions")]
    pub fn get_distributions(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();

        for (key, value) in self.0.distributions() {
            let identifiers_obj = Object::new();
            // &BigInt::from(key.clone()).into()

            for (identifiers_key, identifiers_value) in value {
                Reflect::set(
                    &identifiers_obj,
                    &IdentifierWASM::from(identifiers_key.clone())
                        .get_base58()
                        .into(),
                    &BigInt::from(identifiers_value.clone()).into(),
                )?;
            }

            Reflect::set(&obj, &key.to_string().into(), &identifiers_obj.into())?;
        }

        Ok(obj.into())
    }

    #[wasm_bindgen(setter = "distributions")]
    pub fn set_distributions(&mut self, js_distributions: &JsValue) -> Result<(), JsValue> {
        let distributions = js_distributions_to_distributions(js_distributions)?;

        self.0.set_distributions(distributions);

        Ok(())
    }
}
