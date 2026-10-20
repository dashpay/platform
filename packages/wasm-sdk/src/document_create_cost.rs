//! What creating a document of a type costs, computed by Drive
//! (`drive::drive::document::cost`) from the contract alone.

use crate::error::WasmSdkError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::identity::KeyType;
use dash_sdk::dpp::version::PlatformVersion;
use drive::drive::document::cost::{document_type_create_cost, CostAssumptions, FieldChoice};
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_dpp2::data_contract::model::DataContractWasm;
use wasm_dpp2::serialization::conversions::platform_value_to_object;
use wasm_dpp2::version::{PlatformVersionLikeJs, PlatformVersionWasm};

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_CREATE_COST_TS: &'static str = r#"
/** What to assume when pricing a document create. Every field is optional. */
export interface DocumentCreateCostOptions {
  /**
   * The document to price, by field (`a.b` for a property of an object):
   * whether an optional field is present (default: present), and the length
   * of a variable-size one, in characters, bytes or elements (default: the
   * middle between its bounds, the size Drive's own fee estimates assume).
   */
  fields?: Record<string, { present?: boolean; length?: number }>;
  /** Documents of the type already stored, for the processing estimate (default 1000). */
  existingDocuments?: number;
  /** The type of the key the transition is signed with (default ECDSA_SECP256K1). */
  signatureKeyType?: 'ECDSA_SECP256K1' | 'BLS12_381' | 'ECDSA_HASH160' | 'BIP13_SCRIPT_HASH' | 'EDDSA_25519_HASH160';
  /** The fee increase the transition asks for, in percent of the processing fee (default 0). */
  userFeeIncrease?: number;
  /** The epoch's fee multiplier in permille, for action fees priced by it (default 1000). */
  feeMultiplierPermille?: number;
  /** Contenders already in the contest, for a contested create (default 0). */
  contenders?: number;
}

/**
 * An amount under both storage scenarios: every index value new (the first
 * document with these values creates their trees), and every value an
 * earlier document can hold already stored.
 */
export interface DocumentCostScenarios {
  newValues: number;
  knownValues: number;
}

/** What creating one document of a type costs; amounts in credits (1 Dash = `creditsPerDash`). */
export interface DocumentCreateCost {
  documentType: string;
  assumptions: {
    existingDocuments: number;
    signatureKeyType: string;
    userFeeIncrease: number;
    feeMultiplierPermille: number;
    contenders: number;
  };
  /** The serialized document, in bytes. */
  documentBytes: number;
  /** What a stored byte costs: 27,000 credits, or a ttl type's price for its lifetime. */
  creditsPerByte: number;
  creditsPerDash: number;
  storage: {
    bytes: DocumentCostScenarios;
    credits: DocumentCostScenarios;
    /** The document by id. */
    primaryBytes: DocumentCostScenarios;
    /** Trees created for preallocated indexes of types that will reference the document. */
    preallocatedBytes: DocumentCostScenarios;
    /** A document with a ttl's entry in the documents expirations tree. */
    expirationBytes: DocumentCostScenarios;
  };
  /** Each index: the bytes of the layers it shares with other indexes (counted once) and of its own. */
  indexes: Array<{
    name: string;
    sharedWith: string[];
    sharedBytes: DocumentCostScenarios;
    ownBytes: DocumentCostScenarios;
  }>;
  /** Every element the insert writes. */
  elements: Array<{
    role: string;
    path: string[];
    bytes: number;
    indexes: string[];
    /** Written only when absent: a tree an earlier document with the same values created. */
    ifAbsent: boolean;
    writtenWhenValuesKnown: boolean;
    /** On a time window with a ttl: priced as processing. */
    ephemeral: boolean;
    /** In the documents expirations tree: a document with a ttl's entry. */
    expiration: boolean;
    /** The axis, when the element is a ranked tree's row for the value. */
    rankingAxis?: 'count' | 'sum' | 'avg';
    /** The type whose tree the element is in, when it is a preallocated index's. */
    referringType?: string;
  }>;
  /** The processing fee, part by part; `exact: false` parts are estimated from the assumptions. */
  processing: Array<{ code: string; text: string; credits: DocumentCostScenarios; exact: boolean }>;
  processingCredits: DocumentCostScenarios;
  contractCharges: Array<
    | { kind: 'actionFee'; pricing: 'feeMultiplier' | 'fixed'; declared: { owner: number; moderators: number }; charged: { owner: number; moderators: number } }
    | { kind: 'tokenCost'; tokenPosition: number; amount: number; effect: 'transferToContractOwner' | 'burn'; gasFeesPaidBy: string; optional: boolean; tokenContractId?: string }
    /** Paid when the value is contested; such a create is stored in the vote poll until the contest ends, and that storage is not priced here. */
    | { kind: 'contestFund'; index: string; credits: number }
  >;
  /** The storage fee a delete refunds, in the same epoch and a year later (nothing for a ttl type). */
  refund: { sameEpoch: DocumentCostScenarios; afterOneYear: DocumentCostScenarios };
  /** Storage, processing and action fees; not a token cost or a contest fund. */
  totalCredits: DocumentCostScenarios;
  /** How each field of the priced document was filled; a length only for a variable-size field. */
  fields: Array<{
    path: string;
    kind: string;
    optional: boolean;
    present: boolean;
    length?: number;
    minLength?: number;
    maxLength?: number;
  }>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentCreateCostOptions | undefined")]
    pub type DocumentCreateCostOptionsJs;

    #[wasm_bindgen(typescript_type = "DocumentCreateCost")]
    pub type DocumentCreateCostJs;
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Options {
    fields: Option<BTreeMap<String, FieldOption>>,
    existing_documents: Option<u64>,
    signature_key_type: Option<String>,
    user_fee_increase: Option<u16>,
    fee_multiplier_permille: Option<u64>,
    contenders: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldOption {
    present: Option<bool>,
    length: Option<u32>,
}

/// Refuses a key of `object` that is not one of `known`: `serde_wasm_bindgen`
/// reads only the keys a struct names, so a misspelled option would be
/// silently ignored.
fn refuse_unknown_keys(object: &JsValue, known: &[&str], what: &str) -> Result<(), WasmSdkError> {
    let Some(object) = object.dyn_ref::<js_sys::Object>() else {
        return Ok(());
    };
    for key in js_sys::Object::keys(object).iter() {
        let key = key.as_string().unwrap_or_default();
        if !known.contains(&key.as_str()) {
            return Err(WasmSdkError::invalid_argument(format!(
                "unknown {what} {key:?}; expected one of {}",
                known.join(", ")
            )));
        }
    }
    Ok(())
}

fn key_type(name: &str) -> Result<KeyType, WasmSdkError> {
    Ok(match name {
        "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
        "BLS12_381" => KeyType::BLS12_381,
        "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
        "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
        "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
        other => {
            return Err(WasmSdkError::invalid_argument(format!(
                "unknown signature key type {other:?}"
            )))
        }
    })
}

/// What creating a document of the type `documentTypeName` of `contract`
/// costs: the storage every element it writes adds (exact, split by index
/// into the layers an index shares with others and its own), the processing
/// (the fixed charges of a signed batch, exact, and the writes' work,
/// estimated), what the contract adds (action fees, a token cost, a
/// contest's vote fund) and what a delete refunds.
///
/// The document is built from sizes, not values: each variable-size field
/// at its middle size and each optional field present, unless `options`
/// says otherwise. Computed by Drive from the contract alone, so it needs no
/// network. Follows protocol version 14 on; an earlier `platformVersion` is
/// refused.
#[wasm_bindgen(js_name = "documentCreateCost")]
pub fn document_create_cost(
    contract: &DataContractWasm,
    #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
    options: DocumentCreateCostOptionsJs,
    #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
) -> Result<DocumentCreateCostJs, WasmSdkError> {
    let platform_version: PlatformVersion = PlatformVersionWasm::try_from(platform_version)?.into();
    let options: JsValue = options.into();
    let options: Options = if options.is_undefined() || options.is_null() {
        Options::default()
    } else {
        refuse_unknown_keys(
            &options,
            &[
                "fields",
                "existingDocuments",
                "signatureKeyType",
                "userFeeIncrease",
                "feeMultiplierPermille",
                "contenders",
            ],
            "option",
        )?;
        if let Some(fields) = js_sys::Reflect::get(&options, &JsValue::from_str("fields"))
            .ok()
            .and_then(|fields| fields.dyn_into::<js_sys::Object>().ok())
        {
            for field in js_sys::Object::values(&fields).iter() {
                refuse_unknown_keys(&field, &["present", "length"], "field option")?;
            }
        }
        serde_wasm_bindgen::from_value(options)
            .map_err(|error| WasmSdkError::invalid_argument(format!("options: {error}")))?
    };

    let contract = contract.as_ref();
    let document_type = contract
        .document_type_for_name(&document_type_name)
        .map_err(|_| {
            WasmSdkError::invalid_argument(format!(
                "document type '{document_type_name}' not found in contract"
            ))
        })?;

    let mut assumptions = CostAssumptions::new(&platform_version);
    if let Some(existing_documents) = options.existing_documents {
        assumptions.existing_documents = existing_documents;
    }
    if let Some(name) = options.signature_key_type.as_deref() {
        assumptions.signature_key_type = key_type(name)?;
    }
    if let Some(user_fee_increase) = options.user_fee_increase {
        assumptions.user_fee_increase = user_fee_increase;
    }
    if let Some(fee_multiplier_permille) = options.fee_multiplier_permille {
        assumptions.fee_multiplier_permille = fee_multiplier_permille;
    }
    if let Some(contenders) = options.contenders {
        assumptions.contenders = contenders;
    }
    let choices: BTreeMap<String, FieldChoice> = options
        .fields
        .unwrap_or_default()
        .into_iter()
        .map(|(path, field)| {
            (
                path,
                FieldChoice {
                    present: field.present,
                    length: field.length,
                },
            )
        })
        .collect();

    let cost = document_type_create_cost(
        contract,
        document_type,
        &choices,
        &assumptions,
        &platform_version,
    )
    .map_err(|error| WasmSdkError::generic(format!("document create cost: {error}")))?;
    Ok(platform_value_to_object(&cost.to_value())?.into())
}
