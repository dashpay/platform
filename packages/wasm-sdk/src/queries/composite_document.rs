//! Composite document queries — a page plus the sub-queries derived
//! from it, answered as ONE merged proof.
//!
//! The page is an ordinary documents query with an explicit limit;
//! each sub-query is a by-id join, an indexed lookup, a grouped count,
//! or an independent sibling, whose `IN` clause the node derives from
//! the proven page (or an earlier documents sub-query). Everything
//! rides the typed `getDocuments` V1 wire (the request's `subQueries`)
//! and comes back as one merged grovedb proof — a single quorum-signed
//! state root by construction. The SDK bootstraps the page from the
//! proof, re-derives every sub-query itself, and verifies the whole
//! composition, so a substituted, omitted or injected sub-result fails
//! verification.
//!
//! Pagination lives on the page alone: order by a property and
//! continue with a range clause past the last proven page document.

use crate::error::WasmSdkError;
use crate::queries::document::{
    build_documents_query, parse_order_clause, parse_where_clause, DocumentsQueryInput,
};
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::documents::composite_document_query::{
    CompositeBindingSource, CompositeSubQuery,
};
use dash_sdk::platform::{CompositeDocuments, CompositeSubQueryResult, DocumentQuery, Fetch};
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::MAX_SUB_QUERIES;
use js_sys::{Array, Map, Object, Reflect};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(typescript_custom_section)]
const COMPOSITE_DOCUMENTS_QUERY_TS: &'static str = r#"
/**
 * Where a sub-query's derived values come from: `'page'` for the page's
 * proven documents, or the index of an earlier `documents` sub-query.
 */
export type CompositeBindSource = 'page' | number;

/**
 * The derived clause of a sub-query: `<field> IN <values>`, the values
 * read off the source's proven documents. The request never names them.
 */
export interface CompositeBind {
  /** Defaults to `'page'`. */
  source?: CompositeBindSource;
  /**
   * The source property read off each document: `$id`, `$ownerId`, or an
   * identifier-typed property (dotted paths reach nested properties).
   */
  sourceProperty: string;
  /**
   * The sub-query field receiving the `IN` clause. `$id` makes this a
   * by-id JOIN (the source property must declare `refersTo:
   * permanentDocument` targeting the sub-query's document type, so a
   * missing document is a verification error); otherwise `$ownerId` or an
   * indexed property (a LOOKUP, where absence is a proven fact).
   */
  field: string;
}

/**
 * One sub-query of a composite request.
 */
export interface CompositeSubQuery {
  /** Defaults to the page's contract. Any contract works (profiles keyed by owner, names by identity). */
  dataContractId?: string | Uint8Array;
  documentType: string;
  /** `'documents'` (default) returns the matching documents; `'counts'` one count per derived value. */
  kind?: 'documents' | 'counts';
  /** The FIXED clauses, same shape as a documents query's `where`. Must not name the bound field. */
  where?: any[];
  /**
   * Ordering (documents only), same shape as a documents query's `orderBy`.
   * Every component walks in the page's direction: leave the bound field
   * unordered and it inherits that direction; an ordering that disagrees
   * with the page's direction is refused.
   */
  orderBy?: any[];
  /**
   * Required for a documents lookup on a non-unique index: caps the rows the
   * lookup returns in total, in walk order, like an ordinary query's limit
   * (at most 100). Forbidden for a lookup already bounded by its values, a
   * by-id join, and a count.
   */
  limit?: number;
  /** The derived clause. Omit for a SIBLING: an independent documents query proven under the same root. */
  bind?: CompositeBind;
}

/**
 * A composite document query: the page plus its sub-queries, in binding
 * order (a sub-query may bind only the page or an earlier `documents`
 * sub-query). Paginate with a range clause on the page's ordering
 * property; there is no cursor on this surface.
 */
export interface CompositeDocumentsQuery {
  dataContractId: string | Uint8Array;
  documentType: string;
  /** Page where clauses, same shape as a documents query's `where`. */
  where?: any[];
  /** Page ordering, same shape as a documents query's `orderBy`. */
  orderBy?: any[];
  /** REQUIRED page size — it bounds every derived clause, so there is no server-default fallback. */
  limit: number;
  /** Between 1 and 10. */
  subQueries: CompositeSubQuery[];
}

/**
 * A verified `documents` sub-result: a by-id join in first-appearance
 * order of the derived ids among the source documents; a lookup or
 * sibling in query order.
 */
export interface CompositeDocumentsSubResult {
  kind: 'documents';
  documents: Document[];
}

/**
 * A verified `counts` sub-result: one entry per derived value that has a
 * count, keyed by the value's base58 identifier. A value with no entry
 * counts zero.
 */
export interface CompositeCountsSubResult {
  kind: 'counts';
  counts: Map<string, bigint>;
}

export type CompositeSubResult = CompositeDocumentsSubResult | CompositeCountsSubResult;

/**
 * A verified composite result.
 */
export interface CompositeDocumentsResult {
  /** The page, exactly as the page query alone would return it. */
  pageDocuments: Document[];
  /** One result per sub-query, in request order. */
  subResults: CompositeSubResult[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "CompositeDocumentsQuery")]
    pub type CompositeDocumentsQueryJs;
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BindSourceInput {
    Index(usize),
    Named(String),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompositeBindInput {
    #[serde(default)]
    source: Option<BindSourceInput>,
    source_property: String,
    field: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum SubQueryKindInput {
    #[default]
    Documents,
    Counts,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompositeSubQueryInput {
    #[serde(default)]
    data_contract_id: Option<IdentifierWasm>,
    document_type: String,
    #[serde(default)]
    kind: SubQueryKindInput,
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
    #[serde(default)]
    order_by: Option<Vec<JsonValue>>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    bind: Option<CompositeBindInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompositeDocumentsQueryInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
    #[serde(default)]
    order_by: Option<Vec<JsonValue>>,
    limit: u32,
    sub_queries: Vec<CompositeSubQueryInput>,
}

impl CompositeBindInput {
    fn source(&self, index: usize) -> Result<CompositeBindingSource, WasmSdkError> {
        match &self.source {
            None => Ok(CompositeBindingSource::Page),
            Some(BindSourceInput::Named(name)) if name == "page" => {
                Ok(CompositeBindingSource::Page)
            }
            Some(BindSourceInput::Index(source_index)) if *source_index < index => {
                Ok(CompositeBindingSource::SubQuery(*source_index))
            }
            _ => Err(WasmSdkError::invalid_argument(format!(
                "subQueries[{index}].bind.source must be 'page' or the index of an earlier documents sub-query"
            ))),
        }
    }
}

impl CompositeDocumentsQueryInput {
    fn validate(&self) -> Result<(), WasmSdkError> {
        if self.sub_queries.is_empty() || self.sub_queries.len() > MAX_SUB_QUERIES {
            return Err(WasmSdkError::invalid_argument(format!(
                "a composite document query requires between 1 and {MAX_SUB_QUERIES} sub-queries"
            )));
        }
        if self.limit == 0 || self.limit > u32::from(DEFAULT_QUERY_LIMIT) {
            return Err(WasmSdkError::invalid_argument(format!(
                "a composite document query requires a page limit between 1 and {DEFAULT_QUERY_LIMIT}"
            )));
        }
        for (index, sub_query) in self.sub_queries.iter().enumerate() {
            if let Some(limit) = sub_query.limit {
                if matches!(sub_query.kind, SubQueryKindInput::Counts) {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "subQueries[{index}].limit is forbidden for counts sub-queries"
                    )));
                }
                if limit == 0 || limit > u32::from(DEFAULT_QUERY_LIMIT) {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "subQueries[{index}].limit must be between 1 and {DEFAULT_QUERY_LIMIT}"
                    )));
                }
            }
            if let Some(bind) = &sub_query.bind {
                if let CompositeBindingSource::SubQuery(source) = bind.source(index)? {
                    if !matches!(self.sub_queries[source].kind, SubQueryKindInput::Documents) {
                        return Err(WasmSdkError::invalid_argument(format!(
                            "subQueries[{index}].bind.source must name an earlier documents sub-query"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

async fn parse_composite_documents_query(
    sdk: &WasmSdk,
    query: CompositeDocumentsQueryJs,
) -> Result<DocumentQuery, WasmSdkError> {
    let input: CompositeDocumentsQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "composite documents query",
    )?;

    build_composite_documents_query(sdk, input).await
}

async fn build_composite_documents_query(
    sdk: &WasmSdk,
    input: CompositeDocumentsQueryInput,
) -> Result<DocumentQuery, WasmSdkError> {
    // Validate before fetching even the page contract. In particular, an empty
    // list must not fall through to an ordinary page-only DocumentQuery.
    input.validate()?;
    let page_limit = input.limit;
    let page = build_documents_query(
        sdk,
        DocumentsQueryInput {
            data_contract_id: input.data_contract_id,
            document_type_name: input.document_type,
            where_clauses: input.where_clauses,
            order_by: input.order_by,
            limit: Some(page_limit),
            start_after: None,
            start_at: None,
            group_by: None,
            time_range: None,
        },
    )
    .await?
    .with_limit(page_limit);

    let mut composite = page;
    for (index, sub_input) in input.sub_queries.into_iter().enumerate() {
        let contract = match sub_input.data_contract_id {
            Some(id) => std::sync::Arc::new(sdk.get_or_fetch_contract(id.into()).await?),
            None => composite.data_contract.clone(),
        };
        let mut sub_query = match sub_input.kind {
            SubQueryKindInput::Documents => {
                CompositeSubQuery::documents(contract, &sub_input.document_type)?
            }
            SubQueryKindInput::Counts => {
                CompositeSubQuery::count(contract, &sub_input.document_type)?
            }
        };
        if let Some(where_values) = sub_input.where_clauses {
            for clause_json in where_values.iter() {
                sub_query = sub_query.with_where(parse_where_clause(clause_json)?);
            }
        }
        if let Some(order_values) = sub_input.order_by {
            for clause_json in order_values.iter() {
                sub_query = sub_query.with_order_by(parse_order_clause(clause_json)?);
            }
        }
        if let Some(limit) = sub_input.limit {
            sub_query = sub_query.with_limit(limit);
        }
        if let Some(bind) = sub_input.bind {
            let source = bind.source(index)?;
            sub_query = sub_query.bound_to(source, bind.source_property, bind.field);
        }
        composite = composite.with_sub_query(sub_query);
    }

    Ok(composite)
}

/// A count entry's key is the bound value's index-key bytes; derived
/// values are identifiers, so a 32-byte key renders as base58 (what the
/// page documents' ids render as). Anything else falls back to hex.
fn count_key_to_js(key: &[u8]) -> JsValue {
    match Identifier::from_bytes(key) {
        Ok(identifier) => identifier.to_string(Encoding::Base58).into(),
        Err(_) => hex::encode(key).into(),
    }
}

fn set_field(target: &Object, key: &str, value: &JsValue) -> Result<(), WasmSdkError> {
    Reflect::set(target, &JsValue::from_str(key), value)
        .map(|_| ())
        .map_err(|_| WasmSdkError::generic("failed to build composite result object"))
}

fn composite_result_to_js(
    composite: &CompositeDocuments,
    query: &DocumentQuery,
) -> Result<Object, WasmSdkError> {
    let to_array =
        |documents: &[dash_sdk::platform::Document], contract_id: Identifier, type_name: &str| {
            let array = Array::new();
            for document in documents {
                let wasm_doc =
                    DocumentWasm::new(document.clone(), contract_id, type_name.to_string(), None);
                array.push(&JsValue::from(wasm_doc));
            }
            array
        };

    let page_documents = to_array(
        &composite.page_documents,
        query.data_contract.id(),
        &query.document_type_name,
    );

    let sub_results = Array::new();
    for (sub_query, result) in query.sub_queries.iter().zip(composite.sub_results.iter()) {
        let entry = Object::new();
        match result {
            CompositeSubQueryResult::Documents(documents) => {
                set_field(&entry, "kind", &JsValue::from_str("documents"))?;
                set_field(
                    &entry,
                    "documents",
                    &to_array(
                        documents,
                        sub_query.data_contract.id(),
                        &sub_query.document_type_name,
                    ),
                )?;
            }
            CompositeSubQueryResult::Counts(entries) => {
                let counts = Map::new();
                for count_entry in entries {
                    if let Some(count) = count_entry.count {
                        counts.set(&count_key_to_js(&count_entry.key), &JsValue::from(count));
                    }
                }
                set_field(&entry, "kind", &JsValue::from_str("counts"))?;
                set_field(&entry, "counts", &counts)?;
            }
        }
        sub_results.push(&entry);
    }

    let result = Object::new();
    set_field(&result, "pageDocuments", &page_documents)?;
    set_field(&result, "subResults", &sub_results)?;
    Ok(result)
}

#[wasm_bindgen]
impl WasmSdk {
    /// Run a composite document query (a page plus the sub-queries
    /// derived from it) and return the verified page and every
    /// sub-result.
    ///
    /// The composition is always proof-verified: one merged grovedb
    /// proof commits to one quorum-signed root, every sub-query is
    /// re-derived from the proven page, and a by-id join whose
    /// referenced document is missing is a verification error, not an
    /// absence.
    #[wasm_bindgen(
        js_name = "getCompositeDocuments",
        unchecked_return_type = "CompositeDocumentsResult"
    )]
    pub async fn get_composite_documents(
        &self,
        query: CompositeDocumentsQueryJs,
    ) -> Result<Object, WasmSdkError> {
        let query = parse_composite_documents_query(self, query).await?;
        let composite = CompositeDocuments::fetch(self.as_ref(), query.clone())
            .await?
            .unwrap_or_default();
        composite_result_to_js(&composite, &query)
    }

    /// [`Self::get_composite_documents`] with the response metadata and
    /// proof envelope attached.
    #[wasm_bindgen(
        js_name = "getCompositeDocumentsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<CompositeDocumentsResult>"
    )]
    pub async fn get_composite_documents_with_proof_info(
        &self,
        query: CompositeDocumentsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_composite_documents_query(self, query).await?;
        let (composite, metadata, proof) =
            CompositeDocuments::fetch_with_metadata_and_proof(self.as_ref(), query.clone(), None)
                .await?;
        let result = composite_result_to_js(&composite.unwrap_or_default(), &query)?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            result, metadata, proof,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_provider::WasmTrustedContext;
    use crate::error::WasmSdkErrorKind;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::platform::documents::composite_document_query::CompositeSubQueryKind;
    use dash_sdk::Sdk;
    use serde_json::json;

    fn query_json() -> JsonValue {
        json!({
            "dataContractId": Identifier::new([42; 32]).to_string(Encoding::Base58),
            "documentType": "domain",
            "limit": 10,
            "subQueries": [{
                "documentType": "domain",
                "bind": {"sourceProperty": "$ownerId", "field": "$ownerId"},
                "limit": 10
            }]
        })
    }

    // No mock fetch is registered: reaching contract fetching produces a mock
    // error instead of InvalidArgument, so this also pins validation ordering.
    async fn assert_invalid_before_fetch(value: JsonValue) {
        let input = serde_json::from_value(value).expect("input deserializes");
        let sdk = WasmSdk::new_for_testing(Sdk::new_mock(), None);
        let error = build_composite_documents_query(&sdk, input)
            .await
            .expect_err("invalid composition must fail before fetching contracts");
        assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument);
    }

    #[tokio::test]
    async fn should_reject_empty_and_oversized_compositions_before_fetching() {
        for count in [0, MAX_SUB_QUERIES + 1] {
            let mut value = query_json();
            value["subQueries"] = json!(vec![value["subQueries"][0].clone(); count]);
            assert_invalid_before_fetch(value).await;
        }
    }

    #[tokio::test]
    async fn should_reject_invalid_page_and_sub_query_limits_before_fetching() {
        for limit in [0, u32::from(DEFAULT_QUERY_LIMIT) + 1, u32::MAX] {
            let mut page = query_json();
            page["limit"] = json!(limit);
            assert_invalid_before_fetch(page).await;
            let mut sub = query_json();
            sub["subQueries"][0]["limit"] = json!(limit);
            assert_invalid_before_fetch(sub).await;
        }
    }

    #[tokio::test]
    async fn should_reject_limits_on_count_sub_queries_before_fetching() {
        for limit in [1, u32::from(DEFAULT_QUERY_LIMIT)] {
            let mut value = query_json();
            value["subQueries"][0]["kind"] = json!("counts");
            value["subQueries"][0]["limit"] = json!(limit);
            assert_invalid_before_fetch(value).await;
        }
    }

    #[tokio::test]
    async fn should_reject_invalid_binding_sources_before_fetching() {
        for source in [json!("first"), json!(0), json!(1), json!(u32::MAX)] {
            let mut value = query_json();
            value["subQueries"][0]["bind"]["source"] = source;
            assert_invalid_before_fetch(value).await;
        }
        let mut value = query_json();
        let mut second = value["subQueries"][0].clone();
        second["bind"]["source"] = json!(0);
        value["subQueries"][0]["kind"] = json!("counts");
        value["subQueries"][0]
            .as_object_mut()
            .unwrap()
            .remove("limit");
        value["subQueries"].as_array_mut().unwrap().push(second);
        assert_invalid_before_fetch(value).await;
    }

    #[test]
    fn should_reject_unsupported_fields_and_non_integer_limits_or_sources() {
        for field in [
            "startAt",
            "startAfter",
            "offset",
            "select",
            "groupBy",
            "having",
            "timeRange",
        ] {
            let mut value = query_json();
            value[field] = json!(null);
            assert!(serde_json::from_value::<CompositeDocumentsQueryInput>(value).is_err());
        }
        for invalid in [json!(-1), json!(0.5), json!("10")] {
            let mut value = query_json();
            value["limit"] = invalid;
            assert!(serde_json::from_value::<CompositeDocumentsQueryInput>(value).is_err());
        }
        for invalid in [json!(-1), json!(0.5)] {
            let mut value = query_json();
            value["subQueries"][0]["bind"]["source"] = invalid;
            assert!(serde_json::from_value::<CompositeDocumentsQueryInput>(value).is_err());
        }
    }

    #[test]
    fn should_accept_maximum_size_and_both_page_binding_spellings() {
        let mut value = query_json();
        value["limit"] = json!(DEFAULT_QUERY_LIMIT);
        value["subQueries"][0]["limit"] = json!(DEFAULT_QUERY_LIMIT);
        value["subQueries"] = json!(vec![value["subQueries"][0].clone(); MAX_SUB_QUERIES]);
        value["subQueries"][1]["bind"]["source"] = json!("page");
        value["subQueries"][2]["bind"]["source"] = json!(1);
        serde_json::from_value::<CompositeDocumentsQueryInput>(value)
            .unwrap()
            .validate()
            .expect("valid maximum shape");
    }

    #[tokio::test]
    async fn should_build_one_document_query_preserving_contracts_clauses_and_bindings() {
        let mut inner_sdk = Sdk::new_mock();
        let mut page_contract =
            load_system_data_contract(SystemDataContract::DPNS, inner_sdk.version()).unwrap();
        page_contract.set_id(Identifier::new([42; 32]));
        let mut other_contract = page_contract.clone();
        other_contract.set_id(Identifier::new([43; 32]));
        for contract in [&page_contract, &other_contract] {
            inner_sdk
                .mock()
                .expect_fetch(contract.id(), Some(contract.clone()))
                .await
                .unwrap();
        }
        let sdk =
            WasmSdk::new_for_testing(inner_sdk, Some(WasmTrustedContext::for_testing(vec![])));
        let mut value = query_json();
        value["where"] = json!([["normalizedParentDomainName", "==", "dash"]]);
        value["orderBy"] = json!([["normalizedLabel", "asc"]]);
        value["subQueries"][0]["where"] = value["where"].clone();
        value["subQueries"][0]["orderBy"] = value["orderBy"].clone();
        value["subQueries"].as_array_mut().unwrap().extend([
            json!({
                "dataContractId": other_contract.id().to_string(Encoding::Base58),
                "documentType": "domain", "kind": "counts",
                "bind": {"source": 0, "sourceProperty": "$ownerId", "field": "$ownerId"}
            }),
            json!({
                "dataContractId": other_contract.id().to_string(Encoding::Base58),
                "documentType": "domain", "limit": 5
            }),
        ]);
        let query = build_composite_documents_query(&sdk, serde_json::from_value(value).unwrap())
            .await
            .unwrap();
        assert_eq!(*query.data_contract, page_contract);
        assert_eq!(query.document_type_name, "domain");
        assert_eq!(query.limit, 10);
        assert_eq!(query.sub_queries.len(), 3);
        let lookup = &query.sub_queries[0];
        assert!(std::sync::Arc::ptr_eq(
            &query.data_contract,
            &lookup.data_contract
        ));
        assert_eq!(lookup.where_clauses, query.where_clauses);
        assert_eq!(lookup.order_by_clauses, query.order_by_clauses);
        assert_eq!(lookup.limit, Some(10));
        assert_eq!(lookup.kind, CompositeSubQueryKind::Documents);
        assert_eq!(
            lookup.binding.as_ref().unwrap().source,
            CompositeBindingSource::Page
        );
        let count = &query.sub_queries[1];
        assert_eq!(count.data_contract.id(), other_contract.id());
        assert_eq!(count.kind, CompositeSubQueryKind::Count);
        assert_eq!(count.limit, None);
        let bind = count.binding.as_ref().unwrap();
        assert_eq!(bind.source, CompositeBindingSource::SubQuery(0));
        assert_eq!(bind.source_property, "$ownerId");
        assert_eq!(bind.field, "$ownerId");
        let sibling = &query.sub_queries[2];
        assert_eq!(sibling.data_contract, count.data_contract);
        assert_eq!(sibling.limit, Some(5));
        assert_eq!(sibling.binding, None);
        assert!(sdk.get_cached_contract(&other_contract.id()).is_some());
    }
}
