//! Contract moderation queries: one identity's status on a moderated contract
//! (`getContractModerationStatus`) and one page of a contract's banlist or suspension list
//! (`getContractModerationEntries`).

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::contract_moderation::{
    ContractModerationEntries, ContractModerationEntriesPageQuery, ContractModerationList,
    ContractModerationListStatus, ContractModerationListStatuses, ContractModerationStatusQuery,
};
use dash_sdk::platform::{DataContract, Fetch, Identifier};
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
   * `config.moderation`). Omit to read every list the contract keeps, which costs one
   * extra request for the contract.
   */
  lists?: ContractModerationListKind[];
}

/**
 * One identity's status on the lists of a moderated contract that were read. A list that was
 * not read says nothing: `banned` is undefined, not false, unless `lists` includes `banlist`.
 */
export interface ContractModerationStatus {
  /** The lists this status covers: the ones the query named, or every list the contract keeps. */
  lists: ContractModerationListKind[];
  /** Set when `lists` includes `banlist`: the identity is on the banlist. */
  banned?: boolean;
  /**
   * When `lists` includes `suspensions`: the block time, in milliseconds, until which the
   * identity is suspended; undefined when it is not. A lapsed suspension stays until the
   * identity's next document transition sweeps it.
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
 * next page and is absent when this page holds fewer entries than the limit, which makes it
 * the last one.
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

impl WasmSdk {
    /// The status query for `query`. Without `lists`, the contract is fetched and every list
    /// it keeps is read, because the node refuses a list the contract does not keep.
    async fn contract_moderation_status_query(
        &self,
        query: ContractModerationStatusQueryJs,
    ) -> Result<ContractModerationStatusQuery, WasmSdkError> {
        let input: ContractModerationStatusQueryInput = deserialize_required_query(
            query,
            "Query object is required",
            "contract moderation status query",
        )?;
        let contract_id = Identifier::from(input.contract_id);
        let identity_id = Identifier::from(input.identity_id);

        if let Some(lists) = input.lists {
            return Ok(ContractModerationStatusQuery {
                contract_id,
                identity_id,
                lists: lists.into_iter().map(Into::into).collect(),
            });
        }

        let contract = DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found(format!("contract {contract_id} not found")))?;
        ContractModerationStatusQuery::for_contract(&contract, identity_id).ok_or_else(|| {
            WasmSdkError::invalid_argument(format!("contract {contract_id} is not moderated"))
        })
    }
}

fn parse_entries_query(
    query: ContractModerationEntriesQueryJs,
    platform_version: &PlatformVersion,
) -> Result<ContractModerationEntriesPageQuery, WasmSdkError> {
    let input: ContractModerationEntriesQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contract moderation entries query",
    )?;
    let mut page_query = ContractModerationEntriesPageQuery::new(
        Identifier::from(input.contract_id),
        input.list.into(),
        platform_version,
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

/// Sets `lists`, and `banned` and `suspendedUntil` for the lists read, on `target`. Only the
/// lists read are reported: the field of a list that was not read stays undefined (unknown)
/// rather than reading as "not banned" or "not suspended". The status query and the moderation
/// result share the shape, so they share this.
pub(crate) fn set_status_fields(
    target: &js_sys::Object,
    statuses: &ContractModerationListStatuses,
) -> Result<(), WasmSdkError> {
    let set = |key: &str, value: JsValue| {
        js_sys::Reflect::set(target, &key.into(), &value)
            .map(|_| ())
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the status")))
    };
    let lists = Array::new();
    for status in &statuses.0 {
        match status {
            ContractModerationListStatus::Banlist { banned } => {
                lists.push(&"banlist".into());
                set("banned", (*banned).into())?;
            }
            ContractModerationListStatus::Suspensions { suspended_until } => {
                lists.push(&"suspensions".into());
                if let Some(until) = suspended_until {
                    set("suspendedUntil", js_sys::BigInt::from(*until).into())?;
                }
            }
        }
    }
    set("lists", lists.into())
}

fn status_to_js(statuses: ContractModerationListStatuses) -> Result<JsValue, WasmSdkError> {
    let result = js_sys::Object::new();
    set_status_fields(&result, &statuses)?;
    Ok(result.into())
}

fn entries_to_js(
    page: ContractModerationEntries,
    query: &ContractModerationEntriesPageQuery,
) -> Result<JsValue, WasmSdkError> {
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
    // A page shorter than the limit is the last one, so it carries no cursor.
    if let Some(start_after) = query.after(&page).and_then(|next| next.query.start_after) {
        set(
            &result,
            "nextStartAfter",
            JsValue::from_str(&IdentifierWasm::from(start_after).to_base58()),
        )?;
    }
    Ok(result.into())
}

#[wasm_bindgen]
impl WasmSdk {
    /// One identity's status on a moderated contract: whether it is banned, and until when it
    /// is suspended, on the lists read. Every list the query names must be one the contract
    /// keeps; without `lists`, every list the contract keeps is read.
    ///
    /// # Example
    /// ```javascript
    /// const status = await sdk.getContractModerationStatus({ contractId, identityId });
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
        let query = self.contract_moderation_status_query(query).await?;
        let status = ContractModerationListStatuses::fetch(self.as_ref(), query)
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
        let query = self.contract_moderation_status_query(query).await?;
        let (status, metadata, proof) =
            ContractModerationListStatuses::fetch_with_metadata_and_proof(
                self.as_ref(),
                query,
                None,
            )
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
        let query = parse_entries_query(query, self.inner_sdk().version())?;
        let page = ContractModerationEntries::fetch(self.as_ref(), query.clone())
            .await?
            .unwrap_or_default();
        entries_to_js(page, &query)
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
        let query = parse_entries_query(query, self.inner_sdk().version())?;
        let (page, metadata, proof) = ContractModerationEntries::fetch_with_metadata_and_proof(
            self.as_ref(),
            query.clone(),
            None,
        )
        .await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            entries_to_js(page.unwrap_or_default(), &query)?,
            metadata,
            proof,
        ))
    }
}
