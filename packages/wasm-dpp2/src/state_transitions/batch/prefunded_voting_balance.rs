use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use dpp::fee::Credits;
use wasm_bindgen::prelude::wasm_bindgen;

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
        #[wasm_bindgen(js_name = "indexName")] index_name: String,
        credits: Credits,
    ) -> PrefundedVotingBalanceWasm {
        PrefundedVotingBalanceWasm {
            index_name,
            credits,
        }
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
