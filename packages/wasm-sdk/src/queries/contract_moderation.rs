//! Contract moderation queries: one identity's status on a moderated contract
//! (`getContractModerationStatus`) and one page of a contract's banlist or suspension list
//! (`getContractModerationEntries`).

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::platform::contract_moderation::{
    ContractModerationEntries, ContractModerationEntriesPageQuery, ContractModerationList,
    ContractModerationStatus, ContractModerationStatusQuery,
};
use dash_sdk::platform::{Fetch, Identifier};
use js_sys::Array;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_MODERATION_QUERY_TS: &'static str = r#"
/** One of the two moderation lists a moderated contract may keep. */
export type ContractModerationListKind = 'banlist' | 'suspensions';

/**
 * Query parameters for one identity's status on a moderated contract
 * (`getContractModerationStatus`).
 */
export interface ContractModerationStatusQuery {
  /** The moderated contract. */
  contractId: IdentifierLike;
  /** The identity. */
  identityId: IdentifierLike;
  /**
   * The lists to read; each must be one the contract keeps (see the contract's
   * `config.moderation`). Omit to read both, which a contract keeping only one refuses.
   */
  lists?: ContractModerationListKind[];
}

/** One identity's status on a moderated contract. */
export interface ContractModerationStatus {
  /** The identity is on the banlist. */
  banned: boolean;
  /**
   * The block time, in milliseconds, until which the identity is suspended. A lapsed
   * suspension stays until the identity's next document transition sweeps it.
   */
  suspendedUntil?: bigint;
}

/**
 * Query parameters for one page of a moderated contract's banlist or suspension list
 * (`getContractModerationEntries`).
 */
export interface ContractModerationEntriesQuery {
  /** The moderated contract. */
  contractId: IdentifierLike;
  /** The list to read; the contract must keep it. */
  list: ContractModerationListKind;
  /** Continue after this identity; omit for the first page. Use the page's `nextStartAfter`. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of entries to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/** One entry of a moderation list. */
export interface ContractModerationEntry {
  identityId: string;
  /** For a suspension list entry: the block time, in milliseconds, at which it lapses. */
  until?: bigint;
}

/**
 * One page of a moderation list, in identity id order. `nextStartAfter` is the cursor of the
 * next page and is absent when this page is empty, which makes it the last one.
 */
export interface ContractModerationEntriesPage {
  entries: ContractModerationEntry[];
  nextStartAfter?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractModerationStatusQuery")]
    pub type ContractModerationStatusQueryJs;

    #[wasm_bindgen(typescript_type = "ContractModerationEntriesQuery")]
    pub type ContractModerationEntriesQueryJs;
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum ContractModerationListInput {
    Banlist,
    Suspensions,
}

impl From<ContractModerationListInput> for ContractModerationList {
    fn from(list: ContractModerationListInput) -> Self {
        match list {
            ContractModerationListInput::Banlist => ContractModerationList::Banlist,
            ContractModerationListInput::Suspensions => ContractModerationList::Suspensions,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractModerationStatusQueryInput {
    contract_id: IdentifierWasm,
    identity_id: IdentifierWasm,
    #[serde(default)]
    lists: Option<Vec<ContractModerationListInput>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractModerationEntriesQueryInput {
    contract_id: IdentifierWasm,
    list: ContractModerationListInput,
    #[serde(default)]
    start_after: Option<IdentifierWasm>,
    #[serde(default)]
    limit: Option<u32>,
}

fn parse_status_query(
    query: ContractModerationStatusQueryJs,
) -> Result<ContractModerationStatusQuery, WasmSdkError> {
    let input: ContractModerationStatusQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contract moderation status query",
    )?;
    let lists = input
        .lists
        .map(|lists| lists.into_iter().map(Into::into).collect())
        .unwrap_or_else(|| {
            vec![
                ContractModerationList::Banlist,
                ContractModerationList::Suspensions,
            ]
        });
    Ok(ContractModerationStatusQuery {
        contract_id: Identifier::from(input.contract_id),
        identity_id: Identifier::from(input.identity_id),
        lists,
    })
}

fn parse_entries_query(
    query: ContractModerationEntriesQueryJs,
) -> Result<ContractModerationEntriesPageQuery, WasmSdkError> {
    let input: ContractModerationEntriesQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contract moderation entries query",
    )?;
    let mut page_query = ContractModerationEntriesPageQuery::new(
        Identifier::from(input.contract_id),
        input.list.into(),
    );
    page_query.query.start_after = input.start_after.map(Identifier::from);
    if let Some(limit) = input.limit {
        let limit = u16::try_from(limit).map_err(|_| {
            WasmSdkError::invalid_argument(format!("limit {limit} exceeds maximum of {}", u16::MAX))
        })?;
        page_query = page_query.with_limit(limit);
    }
    Ok(page_query)
}

fn status_to_js(status: ContractModerationStatus) -> Result<JsValue, WasmSdkError> {
    let result = js_sys::Object::new();
    let set = |key: &str, value: JsValue| {
        js_sys::Reflect::set(&result, &key.into(), &value)
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the status")))
    };
    set("banned", status.banned.into())?;
    if let Some(until) = status.suspended_until {
        set("suspendedUntil", js_sys::BigInt::from(until).into())?;
    }
    Ok(result.into())
}

fn entries_to_js(page: ContractModerationEntries) -> Result<JsValue, WasmSdkError> {
    let result = js_sys::Object::new();
    let set = |target: &js_sys::Object, key: &str, value: JsValue| {
        js_sys::Reflect::set(target, &key.into(), &value)
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the page")))
    };
    let entries = Array::new();
    for entry in page.entries() {
        let js_entry = js_sys::Object::new();
        set(
            &js_entry,
            "identityId",
            JsValue::from_str(&IdentifierWasm::from(entry.identity_id).to_base58()),
        )?;
        if let Some(until) = entry.until {
            set(&js_entry, "until", js_sys::BigInt::from(until).into())?;
        }
        entries.push(&js_entry);
    }
    set(&result, "entries", entries.into())?;
    if let Some(last) = page.entries().last() {
        set(
            &result,
            "nextStartAfter",
            JsValue::from_str(&IdentifierWasm::from(last.identity_id).to_base58()),
        )?;
    }
    Ok(result.into())
}

#[wasm_bindgen]
impl WasmSdk {
    /// One identity's status on a moderated contract: whether it is banned, and until when it
    /// is suspended. Every list the query names must be one the contract keeps.
    ///
    /// # Example
    /// ```javascript
    /// const status = await sdk.getContractModerationStatus({ contractId, identityId, lists: ['banlist'] });
    /// if (status.banned) console.log('banned');
    /// ```
    #[wasm_bindgen(
        js_name = "getContractModerationStatus",
        unchecked_return_type = "ContractModerationStatus"
    )]
    pub async fn get_contract_moderation_status(
        &self,
        query: ContractModerationStatusQueryJs,
    ) -> Result<JsValue, WasmSdkError> {
        let query = parse_status_query(query)?;
        let status = ContractModerationStatus::fetch(self.as_ref(), query)
            .await?
            .unwrap_or_default();
        status_to_js(status)
    }

    /// One identity's status on a moderated contract together with its proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractModerationStatusWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractModerationStatus>"
    )]
    pub async fn get_contract_moderation_status_with_proof_info(
        &self,
        query: ContractModerationStatusQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_status_query(query)?;
        let (status, metadata, proof) =
            ContractModerationStatus::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            status_to_js(status.unwrap_or_default())?,
            metadata,
            proof,
        ))
    }

    /// One page of a moderated contract's banlist or suspension list, in identity id order.
    /// Pass the page's `nextStartAfter` as the next query's `startAfter`; a page without one
    /// is the last.
    ///
    /// # Example
    /// ```javascript
    /// let page = await sdk.getContractModerationEntries({ contractId, list: 'banlist', limit: 50 });
    /// while (page.nextStartAfter) {
    ///   page = await sdk.getContractModerationEntries({ contractId, list: 'banlist', limit: 50, startAfter: page.nextStartAfter });
    /// }
    /// ```
    #[wasm_bindgen(
        js_name = "getContractModerationEntries",
        unchecked_return_type = "ContractModerationEntriesPage"
    )]
    pub async fn get_contract_moderation_entries(
        &self,
        query: ContractModerationEntriesQueryJs,
    ) -> Result<JsValue, WasmSdkError> {
        let query = parse_entries_query(query)?;
        let page = ContractModerationEntries::fetch(self.as_ref(), query)
            .await?
            .unwrap_or_default();
        entries_to_js(page)
    }

    /// One page of a moderation list together with its proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractModerationEntriesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractModerationEntriesPage>"
    )]
    pub async fn get_contract_moderation_entries_with_proof_info(
        &self,
        query: ContractModerationEntriesQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_entries_query(query)?;
        let (page, metadata, proof) =
            ContractModerationEntries::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            entries_to_js(page.unwrap_or_default())?,
            metadata,
            proof,
        ))
    }
}
