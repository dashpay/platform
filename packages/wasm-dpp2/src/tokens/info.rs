use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::info::IdentityTokenInfo;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "IdentityTokenInfo")]
#[derive(Clone, Debug, PartialEq)]
pub struct IdentityTokenInfoWasm(IdentityTokenInfo);

impl From<IdentityTokenInfo> for IdentityTokenInfoWasm {
    fn from(info: IdentityTokenInfo) -> Self {
        Self(info)
    }
}

impl From<IdentityTokenInfoWasm> for IdentityTokenInfo {
    fn from(info: IdentityTokenInfoWasm) -> Self {
        info.0
    }
}

#[wasm_bindgen(js_class = IdentityTokenInfo)]
impl IdentityTokenInfoWasm {
    #[wasm_bindgen(getter = "frozen")]
    pub fn frozen(&self) -> bool {
        match &self.0 {
            IdentityTokenInfo::V0(v0) => v0.frozen(),
        }
    }
}
