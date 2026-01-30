use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use dpp::fee::Credits;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrefundedVotingBalanceOptions {
    index_name: String,
    credits: Credits,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for creating a PrefundedVotingBalance instance.
 */
export interface PrefundedVotingBalanceOptions {
    indexName: string;
    credits: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PrefundedVotingBalanceOptions")]
    pub type PrefundedVotingBalanceOptionsJs;
}

#[wasm_bindgen(js_name = "PrefundedVotingBalance")]
#[derive(Clone)]
pub struct PrefundedVotingBalanceWasm {
    index_name: String,
    credits: Credits,
}

impl From<(String, Credits)> for PrefundedVotingBalanceWasm {
    fn from((index_name, credits): (String, Credits)) -> Self {
        PrefundedVotingBalanceWasm {
            index_name,
            credits,
        }
    }
}

impl From<PrefundedVotingBalanceWasm> for (String, Credits) {
    fn from(value: PrefundedVotingBalanceWasm) -> Self {
        (value.index_name, value.credits)
    }
}

#[wasm_bindgen(js_class = PrefundedVotingBalance)]
impl PrefundedVotingBalanceWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: PrefundedVotingBalanceOptionsJs,
    ) -> WasmDppResult<PrefundedVotingBalanceWasm> {
        let opts: PrefundedVotingBalanceOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(PrefundedVotingBalanceWasm {
            index_name: opts.index_name,
            credits: opts.credits,
        })
    }

    #[wasm_bindgen(getter, js_name = "indexName")]
    pub fn index_name(&self) -> String {
        self.index_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn credits(&self) -> Credits {
        self.credits
    }
}

impl_try_from_js_value!(PrefundedVotingBalanceWasm, "PrefundedVotingBalance");
impl_wasm_type_info!(PrefundedVotingBalanceWasm, PrefundedVotingBalance);
