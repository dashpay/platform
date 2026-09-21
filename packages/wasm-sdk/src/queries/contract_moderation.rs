//! Contract moderation queries: one identity's status on a moderated contract
//! (`getContractModerationStatus`), one page of a contract's banlist, suspension list or
//! warning list (`getContractModerationEntries`) and the records of the documents its
//! moderators deleted (`getContractDocumentRemovals`).

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::contract_moderation::{
    ContractDocumentRemoval, ContractDocumentRemovals, ContractDocumentRemovalsPageQuery,
    ContractDocumentRemovalsSelection, ContractModerationEntries,
    ContractModerationEntriesPageQuery, ContractModerationList, ContractModerationListStatus,
    ContractModerationListStatuses, ContractModerationStatusQuery,
};
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use js_sys::Array;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::{moderation_reason_to_js, moderation_warnings_to_js};
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_MODERATION_QUERY_TS: &'static str = r#"
/** One of the moderation lists a moderated contract may keep. */
export type ContractModerationListKind = 'banlist' | 'suspensions' | 'warnings';

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
  /** When the identity is banned: why, as the moderator wrote it. */
  banReason?: ContractModerationReason;
  /**
   * When `lists` includes `suspensions`: the block time, in milliseconds, until which the
   * identity is suspended; undefined when it is not. A lapsed suspension stays until the
   * identity's next document transition sweeps it.
   */
  suspendedUntil?: bigint;
  /** When the identity is suspended: why, as the moderator wrote it. */
  suspensionReason?: ContractModerationReason;
  /**
   * When `lists` includes `warnings`: the identity's warnings, oldest first, an empty array
   * when it carries none. Warnings bar nothing and stay until a moderator clears them.
   */
  warnings?: ContractWarning[];
}

/**
 * Query parameters for one page of a moderated contract's banlist, suspension list or
 * warning list (`getContractModerationEntries`).
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
  /**
   * Why the identity is on the list, as the moderator wrote it: the ban's reason, the
   * suspension's, or for a warning list entry the latest warning's.
   */
  reason: ContractModerationReason;
  /** For a warning list entry: every warning the identity carries, oldest first. */
  warnings?: ContractWarning[];
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

/**
 * Query parameters for the records of the documents a contract's moderators deleted within one
 * document type (`getContractDocumentRemovals`): the records of the documents `documentIds`
 * names, or else one page of them all.
 */
export interface ContractDocumentRemovalsQuery {
  /** The moderated contract. */
  contractId: IdentifierLike;
  /** The document type; it must set `canBeDeletedByModerators`, as no other keeps records. */
  documentTypeName: string;
  /**
   * Read the records of these documents alone, 1 to 100 distinct ids. A document with no
   * record is left out of the answer. Refused beside `startAfter` or `limit`.
   */
  documentIds?: IdentifierLike[];
  /** Continue after this document; omit for the first page. Use the page's `nextStartAfter`. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of records to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/**
 * The record a contract keeps of one document a moderator deleted. A document id is produced
 * at most once, so the removed id can not be created again; what can bring the document back
 * is a moderator's restore within a week of the deletion, which marks the record restored and
 * leaves it in place. A restored document deleted again gets a fresh record.
 */
export interface ContractDocumentRemovalEntry {
  documentId: string;
  /** The identity that owned the document when it was removed. */
  documentOwnerId: string;
  /** The contract owner or moderator that removed it. */
  moderatorId: string;
  /** Why, as the moderator wrote it: the text may be empty. */
  reason: ContractModerationReason;
  /** The time of the block that removed it, in milliseconds. */
  removedAt: bigint;
  /**
   * A double SHA-256 of the document as it was serialized under its type when it was
   * removed, as 64 hex characters: what a restore must bring back byte for byte.
   */
  documentHash: string;
  /** The contract owner or moderator that restored the document; absent while the removal stands. */
  restoredBy?: string;
  /** The time of the block that restored it, in milliseconds; absent while the removal stands. */
  restoredAt?: bigint;
}

/**
 * Removal records in document id order. For a page, `nextStartAfter` is the cursor of the next
 * page and is absent when this page holds fewer records than the limit, which makes it the
 * last one. A read by `documentIds` never carries one.
 */
export interface ContractDocumentRemovalsPage {
  removals: ContractDocumentRemovalEntry[];
  nextStartAfter?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractModerationStatusQuery")]
    pub type ContractModerationStatusQueryJs;

    #[wasm_bindgen(typescript_type = "ContractModerationEntriesQuery")]
    pub type ContractModerationEntriesQueryJs;

    #[wasm_bindgen(typescript_type = "ContractDocumentRemovalsQuery")]
    pub type ContractDocumentRemovalsQueryJs;
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum ContractModerationListInput {
    Banlist,
    Suspensions,
    Warnings,
}

impl From<ContractModerationListInput> for ContractModerationList {
    fn from(list: ContractModerationListInput) -> Self {
        match list {
            ContractModerationListInput::Banlist => ContractModerationList::Banlist,
            ContractModerationListInput::Suspensions => ContractModerationList::Suspensions,
            ContractModerationListInput::Warnings => ContractModerationList::Warnings,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractDocumentRemovalsQueryInput {
    contract_id: IdentifierWasm,
    document_type_name: String,
    #[serde(default)]
    document_ids: Option<Vec<IdentifierWasm>>,
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
            // A moderated contract may keep no list at all, when its moderators only delete
            // documents: there is then no status to read, which is not the same as no
            // moderation.
            let reason = match contract.config().moderation() {
                Some(_) => "keeps no moderation list, so no identity has a status on it",
                None => "is not moderated",
            };
            WasmSdkError::invalid_argument(format!("contract {contract_id} {reason}"))
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

fn parse_removals_query(
    query: ContractDocumentRemovalsQueryJs,
    platform_version: &PlatformVersion,
) -> Result<ContractDocumentRemovalsPageQuery, WasmSdkError> {
    let input: ContractDocumentRemovalsQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contract document removals query",
    )?;
    let contract_id = Identifier::from(input.contract_id);

    if let Some(document_ids) = input.document_ids {
        // A read by ids is not paged: a cursor or a limit beside it is refused rather than
        // dropped, or a caller expecting a page would read something else.
        if input.start_after.is_some() || input.limit.is_some() {
            return Err(WasmSdkError::invalid_argument(
                "`startAfter` and `limit` page through every record and are not valid beside `documentIds`",
            ));
        }
        // No id named is not a page of every record either, and the node refuses it.
        if document_ids.is_empty() {
            return Err(WasmSdkError::invalid_argument(
                "`documentIds` must name at least one document",
            ));
        }
        return Ok(ContractDocumentRemovalsPageQuery::for_document_ids(
            contract_id,
            input.document_type_name,
            document_ids.into_iter().map(Identifier::from).collect(),
        ));
    }

    let mut page_query = ContractDocumentRemovalsPageQuery::new(
        contract_id,
        input.document_type_name,
        platform_version,
    );
    if let Some(limit) = input.limit {
        let limit = u16::try_from(limit).map_err(|_| {
            WasmSdkError::invalid_argument(format!("limit {limit} exceeds maximum of {}", u16::MAX))
        })?;
        page_query = page_query.with_limit(limit);
    }
    if let ContractDocumentRemovalsSelection::Page { start_after, .. } =
        &mut page_query.query.selection
    {
        *start_after = input.start_after.map(Identifier::from);
    }
    Ok(page_query)
}

/// Sets `lists`, and `banned`, `suspendedUntil` and `warnings` for the lists read, each with
/// the reason of the entry found, on `target`. Only the lists read are reported: the field of
/// a list that was not read stays undefined (unknown) rather than reading as "not banned",
/// "not suspended" or "never warned". The status query and the moderation result share the
/// shape, so they share this.
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
            ContractModerationListStatus::Banlist { ban } => {
                lists.push(&"banlist".into());
                set("banned", ban.is_some().into())?;
                if let Some(ban) = ban {
                    set("banReason", moderation_reason_to_js(&ban.reason))?;
                }
            }
            ContractModerationListStatus::Suspensions { suspension } => {
                lists.push(&"suspensions".into());
                if let Some(suspension) = suspension {
                    set(
                        "suspendedUntil",
                        js_sys::BigInt::from(suspension.until).into(),
                    )?;
                    set(
                        "suspensionReason",
                        moderation_reason_to_js(&suspension.reason),
                    )?;
                }
            }
            ContractModerationListStatus::Warnings { warnings } => {
                lists.push(&"warnings".into());
                set("warnings", moderation_warnings_to_js(warnings, false))?;
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
        set(&js_entry, "reason", moderation_reason_to_js(&entry.reason))?;
        if !entry.warnings.is_empty() {
            set(
                &js_entry,
                "warnings",
                moderation_warnings_to_js(&entry.warnings, false),
            )?;
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

/// Sets `documentOwnerId`, `moderatorId`, `reason`, `removedAt` and `documentHash` on
/// `target`, and `restoredBy` and `restoredAt` when the document was restored. The removals
/// query and the result of a deletion or a restore carry the same record, so they share this.
/// Each writes identifiers its own way, which `id_to_js` decides: base58 strings in a query
/// answer, as the entries of a moderation list are, and `Identifier`s in the result of a
/// transition. The hash is 64 hex characters either way.
pub(crate) fn set_removal_fields(
    target: &js_sys::Object,
    removal: &ContractDocumentRemoval,
    id_to_js: impl Fn(Identifier) -> JsValue,
) -> Result<(), WasmSdkError> {
    let set = |key: &str, value: JsValue| {
        js_sys::Reflect::set(target, &key.into(), &value)
            .map(|_| ())
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the removal")))
    };
    set("documentOwnerId", id_to_js(removal.document_owner_id))?;
    set("moderatorId", id_to_js(removal.moderator_id))?;
    set("reason", moderation_reason_to_js(&removal.reason))?;
    set("removedAt", js_sys::BigInt::from(removal.removed_at).into())?;
    set(
        "documentHash",
        JsValue::from_str(&hex::encode(removal.document_hash)),
    )?;
    if let Some(restoration) = &removal.restoration {
        set("restoredBy", id_to_js(restoration.moderator_id))?;
        set(
            "restoredAt",
            js_sys::BigInt::from(restoration.restored_at).into(),
        )?;
    }
    Ok(())
}

fn removals_to_js(
    page: ContractDocumentRemovals,
    query: &ContractDocumentRemovalsPageQuery,
) -> Result<JsValue, WasmSdkError> {
    let result = js_sys::Object::new();
    let set = |target: &js_sys::Object, key: &str, value: JsValue| {
        js_sys::Reflect::set(target, &key.into(), &value)
            .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}` on the page")))
    };
    let id_to_js = |id: Identifier| JsValue::from_str(&IdentifierWasm::from(id).to_base58());
    let removals = Array::new();
    for entry in page.removals() {
        let js_entry = js_sys::Object::new();
        set(&js_entry, "documentId", id_to_js(entry.document_id))?;
        set_removal_fields(&js_entry, &entry.removal, id_to_js)?;
        removals.push(&js_entry);
    }
    set(&result, "removals", removals.into())?;
    // A page shorter than the limit is the last one, and a read by ids has no page after it:
    // neither carries a cursor.
    let next_start_after = query
        .after(&page)
        .and_then(|next| match next.query.selection {
            ContractDocumentRemovalsSelection::Page { start_after, .. } => start_after,
            ContractDocumentRemovalsSelection::DocumentIds(_) => None,
        });
    if let Some(start_after) = next_start_after {
        set(&result, "nextStartAfter", id_to_js(start_after))?;
    }
    Ok(result.into())
}

#[wasm_bindgen]
impl WasmSdk {
    /// One identity's status on a moderated contract: whether it is banned, until when it is
    /// suspended, and the warnings it carries, on the lists read. Every list the query names must be one the contract
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

    /// One page of a moderated contract's banlist, suspension list or warning list, in
    /// identity id order. Pass the page's `nextStartAfter` as the next query's `startAfter`;
    /// a page without one is the last.
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

    /// The records of the documents a contract's moderators deleted within one document type,
    /// in document id order: the records of the `documentIds` named, where a document with no
    /// record is left out, or else one page of them all. Pass a page's `nextStartAfter` as the
    /// next query's `startAfter`; a page without one is the last. The document type must set
    /// `canBeDeletedByModerators`: no other keeps records, and the node refuses the query.
    ///
    /// # Example
    /// ```javascript
    /// const { removals } = await sdk.getContractDocumentRemovals({ contractId, documentTypeName: 'post', documentIds: [documentId] });
    /// if (removals.length) console.log('removed by', removals[0].moderatorId);
    /// ```
    #[wasm_bindgen(
        js_name = "getContractDocumentRemovals",
        unchecked_return_type = "ContractDocumentRemovalsPage"
    )]
    pub async fn get_contract_document_removals(
        &self,
        query: ContractDocumentRemovalsQueryJs,
    ) -> Result<JsValue, WasmSdkError> {
        let query = parse_removals_query(query, self.inner_sdk().version())?;
        let page = ContractDocumentRemovals::fetch(self.as_ref(), query.clone())
            .await?
            .unwrap_or_default();
        removals_to_js(page, &query)
    }

    /// The removal records of a document type together with their proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractDocumentRemovalsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractDocumentRemovalsPage>"
    )]
    pub async fn get_contract_document_removals_with_proof_info(
        &self,
        query: ContractDocumentRemovalsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_removals_query(query, self.inner_sdk().version())?;
        let (page, metadata, proof) = ContractDocumentRemovals::fetch_with_metadata_and_proof(
            self.as_ref(),
            query.clone(),
            None,
        )
        .await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            removals_to_js(page.unwrap_or_default(), &query)?,
            metadata,
            proof,
        ))
    }
}
