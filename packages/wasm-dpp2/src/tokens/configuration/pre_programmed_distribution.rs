use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::utils::{try_to_map, try_to_u64};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
use dpp::prelude::{Identifier, TimestampMillis};
use js_sys::{BigInt, Map};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Distribution amounts per identity: base58 Identifier string -> token amount (bigint).
 */
export type DistributionAmountsMap = Map<string, bigint>;

/**
 * Pre-programmed distributions: timestamp (string) -> distribution amounts map.
 */
export type PreProgrammedDistributionsMap = Map<string, DistributionAmountsMap>;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PreProgrammedDistributionsMap")]
    pub type PreProgrammedDistributionsMapJs;

    #[wasm_bindgen(typescript_type = "DistributionAmountsMap")]
    pub type DistributionAmountsMapJs;
}

impl_from_for_extern_type!(PreProgrammedDistributionsMapJs, Map);
impl_from_for_extern_type!(DistributionAmountsMapJs, Map);

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

fn distribution_amounts_from_map(
    amounts_map: &Map,
) -> WasmDppResult<BTreeMap<Identifier, TokenAmount>> {
    let mut amounts = BTreeMap::new();

    for entry in amounts_map.entries().into_iter() {
        let entry = entry.map_err(|e| {
            WasmDppError::invalid_argument(format!("Failed to iterate map entries: {:?}", e))
        })?;

        let entry_array = js_sys::Array::from(&entry);
        let key = entry_array.get(0);
        let value = entry_array.get(1);

        let identifier: Identifier = IdentifierWasm::try_from(key)
            .map_err(|e| WasmDppError::invalid_argument(format!("Invalid identifier: {}", e)))?
            .into();

        let token_amount = try_to_u64(&value, "tokenAmount")?;

        amounts.insert(identifier, token_amount);
    }

    Ok(amounts)
}

pub fn distributions_from_map(
    distributions_map: &Map,
) -> WasmDppResult<BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>> {
    let mut distributions = BTreeMap::new();

    for entry in distributions_map.entries().into_iter() {
        let entry = entry.map_err(|e| {
            WasmDppError::invalid_argument(format!("Failed to iterate map entries: {:?}", e))
        })?;

        let entry_array = js_sys::Array::from(&entry);
        let key = entry_array.get(0);
        let value = entry_array.get(1);

        let timestamp_str = key.as_string().ok_or_else(|| {
            WasmDppError::invalid_argument("Cannot read timestamp in distribution rules")
        })?;

        let timestamp = timestamp_str.parse::<TimestampMillis>().map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "Invalid timestamp '{}': {}",
                timestamp_str, err
            ))
        })?;

        let inner_map = try_to_map(value, "distribution amounts")?;
        let amounts = distribution_amounts_from_map(&inner_map)?;

        distributions.insert(timestamp, amounts);
    }

    Ok(distributions)
}

fn distribution_amounts_to_map(
    amounts: &BTreeMap<Identifier, TokenAmount>,
) -> DistributionAmountsMapJs {
    let js_map = Map::new();

    for (identifier, amount) in amounts {
        let identifier_wasm = IdentifierWasm::from(*identifier);
        js_map.set(
            &identifier_wasm.to_base58().into(),
            &BigInt::from(*amount).into(),
        );
    }

    js_map.into()
}

fn distributions_to_map(
    distributions: &BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
) -> PreProgrammedDistributionsMapJs {
    let js_map = Map::new();

    for (timestamp, amounts) in distributions {
        let amounts_map = distribution_amounts_to_map(amounts);
        js_map.set(&JsValue::from(timestamp.to_string()), &amounts_map.into());
    }

    js_map.into()
}

#[wasm_bindgen(js_class = TokenPreProgrammedDistribution)]
impl TokenPreProgrammedDistributionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        distributions: PreProgrammedDistributionsMapJs,
    ) -> WasmDppResult<TokenPreProgrammedDistributionWasm> {
        let distributions_map =
            distributions_from_map(&try_to_map(distributions.into(), "distributions")?)?;

        Ok(TokenPreProgrammedDistributionWasm(
            TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 {
                distributions: distributions_map,
            }),
        ))
    }

    #[wasm_bindgen(getter = "distributions")]
    pub fn distributions(&self) -> PreProgrammedDistributionsMapJs {
        distributions_to_map(self.0.distributions())
    }

    #[wasm_bindgen(setter = "distributions")]
    pub fn set_distributions(
        &mut self,
        distributions: PreProgrammedDistributionsMapJs,
    ) -> WasmDppResult<()> {
        let distributions_map =
            distributions_from_map(&try_to_map(distributions.into(), "distributions")?)?;

        self.0.set_distributions(distributions_map);

        Ok(())
    }
}

impl_wasm_type_info!(
    TokenPreProgrammedDistributionWasm,
    TokenPreProgrammedDistribution
);
