use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use dpp::data_contract::document_type::action_fees::agreement::v0::DocumentActionFeeAgreementV0;
use dpp::data_contract::document_type::action_fees::agreement::{
    AgreedFeeMultiplier, DocumentActionFeeAgreement,
};
use dpp::fee::Credits;
use dpp::prelude::FeeMultiplier;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgreedFeeMultiplierOptions {
    known_permille: FeeMultiplier,
    increase_tolerance_percent: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentActionFeeAgreementOptions {
    // Optional as `Option` rather than `#[serde(default)]` on the amount itself: a property
    // present with the value `undefined` is not a missing one, and an omitted amount is 0.
    #[serde(default)]
    owner: Option<Credits>,
    #[serde(default)]
    moderators: Option<Credits>,
    #[serde(default)]
    fee_multiplier: Option<AgreedFeeMultiplierOptions>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * The fee multiplier a transition's signer knew, and how far above it they accept to pay.
 */
export interface AgreedFeeMultiplierOptions {
    knownPermille: bigint;
    increaseTolerancePercent: number;
}

/**
 * Options for creating a DocumentActionFeeAgreement instance.
 *
 * `feeMultiplier` is named for a fee the document type prices by the fee multiplier and left
 * out for a fixed one.
 */
export interface DocumentActionFeeAgreementOptions {
    owner?: bigint;
    moderators?: bigint;
    feeMultiplier?: AgreedFeeMultiplierOptions;
}

/**
 * AgreedFeeMultiplier serialized as a plain object.
 */
export interface AgreedFeeMultiplierObject {
    knownPermille: bigint;
    increaseTolerancePercent: number;
}

/**
 * AgreedFeeMultiplier serialized as JSON.
 */
export interface AgreedFeeMultiplierJSON {
    knownPermille: number | string;
    increaseTolerancePercent: number;
}

/**
 * DocumentActionFeeAgreement serialized as a plain object.
 */
export interface DocumentActionFeeAgreementObject {
    $formatVersion: string;
    owner: bigint;
    moderators: bigint;
    feeMultiplier: AgreedFeeMultiplierObject | null;
}

/**
 * DocumentActionFeeAgreement serialized as JSON.
 */
export interface DocumentActionFeeAgreementJSON {
    $formatVersion: string;
    owner: number | string;
    moderators: number | string;
    feeMultiplier: AgreedFeeMultiplierJSON | null;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentActionFeeAgreementOptions")]
    pub type DocumentActionFeeAgreementOptionsJs;

    #[wasm_bindgen(typescript_type = "DocumentActionFeeAgreementObject")]
    pub type DocumentActionFeeAgreementObjectJs;

    #[wasm_bindgen(typescript_type = "DocumentActionFeeAgreementJSON")]
    pub type DocumentActionFeeAgreementJSONJs;
}

/// What a document transition agrees to pay in document action fees. The amounts are the ones
/// the document type declares for the action, read off the contract by the signer.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "DocumentActionFeeAgreement")]
pub struct DocumentActionFeeAgreementWasm(DocumentActionFeeAgreement);

impl From<DocumentActionFeeAgreement> for DocumentActionFeeAgreementWasm {
    fn from(agreement: DocumentActionFeeAgreement) -> Self {
        DocumentActionFeeAgreementWasm(agreement)
    }
}

impl From<DocumentActionFeeAgreementWasm> for DocumentActionFeeAgreement {
    fn from(agreement: DocumentActionFeeAgreementWasm) -> Self {
        agreement.0
    }
}

#[wasm_bindgen(js_class = DocumentActionFeeAgreement)]
impl DocumentActionFeeAgreementWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: DocumentActionFeeAgreementOptionsJs) -> WasmDppResult<Self> {
        let opts: DocumentActionFeeAgreementOptions =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(DocumentActionFeeAgreementWasm(
            DocumentActionFeeAgreement::V0(DocumentActionFeeAgreementV0 {
                owner: opts.owner.unwrap_or_default(),
                moderators: opts.moderators.unwrap_or_default(),
                fee_multiplier: opts.fee_multiplier.map(|multiplier| AgreedFeeMultiplier {
                    known_permille: multiplier.known_permille,
                    increase_tolerance_percent: multiplier.increase_tolerance_percent,
                }),
            }),
        ))
    }

    #[wasm_bindgen(getter = "owner")]
    pub fn owner(&self) -> Credits {
        self.0.owner()
    }

    #[wasm_bindgen(getter = "moderators")]
    pub fn moderators(&self) -> Credits {
        self.0.moderators()
    }

    #[wasm_bindgen(getter = "knownFeeMultiplierPermille")]
    pub fn known_fee_multiplier_permille(&self) -> Option<FeeMultiplier> {
        self.0
            .fee_multiplier()
            .map(|multiplier| multiplier.known_permille)
    }

    #[wasm_bindgen(getter = "feeMultiplierIncreaseTolerancePercent")]
    pub fn fee_multiplier_increase_tolerance_percent(&self) -> Option<u16> {
        self.0
            .fee_multiplier()
            .map(|multiplier| multiplier.increase_tolerance_percent)
    }

    #[wasm_bindgen(getter = "pricing")]
    pub fn pricing(&self) -> String {
        self.0.pricing().as_str().to_string()
    }
}

impl_wasm_conversions_inner!(
    DocumentActionFeeAgreementWasm,
    DocumentActionFeeAgreement,
    DocumentActionFeeAgreement,
    DocumentActionFeeAgreementObjectJs,
    DocumentActionFeeAgreementJSONJs
);
impl_try_from_js_value!(DocumentActionFeeAgreementWasm, "DocumentActionFeeAgreement");
impl_wasm_type_info!(DocumentActionFeeAgreementWasm, DocumentActionFeeAgreement);
