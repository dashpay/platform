use dpp::fee::Credits;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "PrefundedVotingBalance")]
#[derive(Clone)]
pub struct PrefundedVotingBalanceWASM {
    index_name: String,
    credits: Credits,
}

impl From<(String, Credits)> for PrefundedVotingBalanceWASM {
    fn from((index_name, credits): (String, Credits)) -> Self {
        PrefundedVotingBalanceWASM {
            index_name,
            credits,
        }
    }
}

impl From<PrefundedVotingBalanceWASM> for (String, Credits) {
    fn from(value: PrefundedVotingBalanceWASM) -> Self {
        (value.index_name, value.credits)
    }
}

#[wasm_bindgen(js_class = PrefundedVotingBalance)]
impl PrefundedVotingBalanceWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "PrefundedVotingBalance".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "PrefundedVotingBalance".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(index_name: String, credits: Credits) -> PrefundedVotingBalanceWASM {
        PrefundedVotingBalanceWASM {
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
        self.credits.clone()
    }
}
