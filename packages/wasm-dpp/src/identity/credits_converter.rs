use dpp::identity::RATIO;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=getCreditsConversionRatio)]
pub fn credit_conversion_ratio() -> f64 {
    RATIO as f64
}
