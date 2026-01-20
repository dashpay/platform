use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use crate::utils::{JsValueExt, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
use dpp::prelude::{Identifier, TimestampMillis};
use js_sys::{BigInt, Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/**
 * Distribution amounts per identity: identifier (base58) -> token amount (bigint).
 */
export interface DistributionAmounts {
    [identifierBase58: string]: bigint;
}

/**
 * Pre-programmed distributions: timestamp (string) -> distribution amounts.
 */
export interface PreProgrammedDistributions {
    [timestampMs: string]: DistributionAmounts;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PreProgrammedDistributions")]
    pub type PreProgrammedDistributionsJs;
}

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

pub fn distributions_to_map(
    distributions_object: &Object,
) -> WasmDppResult<BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>> {
    let distributions_keys = Object::keys(distributions_object);

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

        let identifiers_js = Reflect::get(distributions_object, &key).map_err(|err| {
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
            let identifier = IdentifierWasm::try_from(id_key.clone())?.into();

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(distributions: &Object) -> WasmDppResult<TokenPreProgrammedDistributionWasm> {
        let distributions_map = distributions_to_map(distributions)?;

        Ok(TokenPreProgrammedDistributionWasm(
            TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions: distributions_map }),
        ))
    }

    #[wasm_bindgen(getter = "distributions")]
    pub fn get_distributions(&self) -> WasmDppResult<PreProgrammedDistributionsJs> {
        let obj = Object::new();

        for (key, value) in self.0.distributions() {
            let identifiers_obj = Object::new();

            for (identifiers_key, identifiers_value) in value {
                Reflect::set(
                    &identifiers_obj,
                    &IdentifierWasm::from(*identifiers_key).to_base58().into(),
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

        Ok(obj.unchecked_into())
    }

    #[wasm_bindgen(setter = "distributions")]
    pub fn set_distributions(&mut self, distributions: &Object) -> WasmDppResult<()> {
        let distributions_map = distributions_to_map(distributions)?;

        self.0.set_distributions(distributions_map);

        Ok(())
    }
}

impl_wasm_type_info!(TokenPreProgrammedDistributionWasm, TokenPreProgrammedDistribution);
