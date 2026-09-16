//! Chained document queries — the provable semi-join surface.
//!
//! `SELECT * FROM <outer> WHERE $id IN (SELECT <joinProperty> FROM
//! <inner> WHERE …)` riding the typed `getDocuments` V1 wire (the
//! request's `chained` message):
//! the inner indexOnly page and the outer by-ids fetch derived from its
//! proven values come back as ONE merged grovedb proof — a single
//! quorum-signed state root by construction. The SDK verifies the
//! composition — the outer query is re-derived from the response's
//! untrusted join-value hint and checked against the PROVEN inner
//! results — so the join cannot be steered by the responding server. The request deliberately carries no outer clauses; filter
//! the returned outer documents locally if needed.
//!
//! Pagination lives on the inner query alone: order by the join
//! property and continue with `where: [[joinProperty, ">", <last inner
//! entry's join value>]]`.

use crate::error::WasmSdkError;
use crate::queries::document::{build_documents_query, DocumentsQueryInput};
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::platform::documents::chained_document_query::ChainedDocumentQuery;
use dash_sdk::platform::{ChainedDocuments, Fetch};
use js_sys::{Array, Object, Reflect};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(typescript_custom_section)]
const CHAINED_DOCUMENTS_QUERY_TS: &'static str = r#"
/**
 * A chained document query — a provable semi-join:
 * `SELECT * FROM <outerDocumentType> WHERE $id IN
 *   (SELECT <joinProperty> FROM <innerDocumentType> WHERE ...)`.
 *
 * The inner query must target an indexOnly document type and resolve to
 * an index carrying `joinProperty`, and `joinProperty` must declare a
 * same-contract `refersTo: permanentDocument` targeting
 * `outerDocumentType` ("posts I liked": inner `like` through `byLiker`,
 * join `postId`, outer `post`). There are no outer-side clauses by
 * design — the verifier derives the outer query from the proven inner
 * results.
 */
interface ChainedDocumentsQuery {
  /** The contract both document types live in. */
  dataContractId: string | Uint8Array;
  /** The indexOnly document type queried directly (e.g. "like"). */
  innerDocumentType: string;
  /** Inner where clauses, same shape as a documents query's `where`. */
  where?: any[];
  /** Inner ordering, same shape as a documents query's `orderBy`. */
  orderBy?: any[];
  /**
   * REQUIRED page size of the inner query — it bounds the derived
   * outer query, so there is no server-default fallback.
   */
  innerLimit: number;
  /** The inner property whose proven values become the outer `$id`s. */
  joinProperty: string;
  /** The joined document type — the `refersTo` target (e.g. "post"). */
  outerDocumentType: string;
}

/**
 * Both halves of a verified chained query, in inner-proof order.
 */
interface ChainedDocumentsResult {
  /**
   * The inner projections exactly as the inner query alone would
   * return them; the last one's join property is the pagination
   * cursor.
   */
  innerDocuments: Document[];
  /**
   * The joined outer documents, ordered by first appearance of their
   * id among the inner projections (deduplicated).
   */
  outerDocuments: Document[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ChainedDocumentsQuery")]
    pub type ChainedDocumentsQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChainedDocumentsQueryInput {
    data_contract_id: IdentifierWasm,
    inner_document_type: String,
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
    #[serde(default)]
    order_by: Option<Vec<JsonValue>>,
    inner_limit: u32,
    join_property: String,
    outer_document_type: String,
}

async fn parse_chained_documents_query(
    sdk: &WasmSdk,
    query: ChainedDocumentsQueryJs,
) -> Result<ChainedDocumentQuery, WasmSdkError> {
    let input: ChainedDocumentsQueryInput =
        deserialize_required_query(query, "Query object is required", "chained documents query")?;

    let inner_limit = input.inner_limit;
    let inner = build_documents_query(
        sdk,
        DocumentsQueryInput {
            data_contract_id: input.data_contract_id,
            document_type_name: input.inner_document_type,
            where_clauses: input.where_clauses,
            order_by: input.order_by,
            limit: Some(inner_limit),
            start_after: None,
            start_at: None,
            group_by: None,
            time_range: None,
        },
    )
    .await?;

    Ok(ChainedDocumentQuery::new(
        inner.with_limit(inner_limit),
        input.join_property,
        input.outer_document_type,
    ))
}

fn chained_result_to_js(
    chained: &ChainedDocuments,
    query: &ChainedDocumentQuery,
) -> Result<Object, WasmSdkError> {
    let contract_id = query.inner.data_contract.id();
    let to_array = |documents: &[dash_sdk::platform::Document], type_name: &str| {
        let array = Array::new();
        for document in documents {
            let wasm_doc =
                DocumentWasm::new(document.clone(), contract_id, type_name.to_string(), None);
            array.push(&JsValue::from(wasm_doc));
        }
        array
    };
    let inner_documents = to_array(&chained.inner_documents, &query.inner.document_type_name);
    let outer_documents = to_array(&chained.outer_documents, &query.outer_document_type_name);

    let result = Object::new();
    Reflect::set(
        &result,
        &JsValue::from_str("innerDocuments"),
        &inner_documents,
    )
    .map_err(|_| WasmSdkError::generic("failed to build chained result object"))?;
    Reflect::set(
        &result,
        &JsValue::from_str("outerDocuments"),
        &outer_documents,
    )
    .map_err(|_| WasmSdkError::generic("failed to build chained result object"))?;
    Ok(result)
}

#[wasm_bindgen]
impl WasmSdk {
    /// Run a chained document query (provable semi-join) and return
    /// both verified halves.
    ///
    /// The composition is always proof-verified: one merged grovedb
    /// proof commits to one quorum-signed root, and the proven outer
    /// documents must match the proven inner join values exactly (a
    /// missing referenced document is a verification error, not an
    /// absence).
    #[wasm_bindgen(
        js_name = "getChainedDocuments",
        unchecked_return_type = "ChainedDocumentsResult"
    )]
    pub async fn get_chained_documents(
        &self,
        query: ChainedDocumentsQueryJs,
    ) -> Result<Object, WasmSdkError> {
        let query = parse_chained_documents_query(self, query).await?;
        let chained = ChainedDocuments::fetch(self.as_ref(), query.clone())
            .await?
            .unwrap_or_default();
        chained_result_to_js(&chained, &query)
    }

    /// [`Self::get_chained_documents`] with the response metadata and
    /// proof envelope attached.
    #[wasm_bindgen(
        js_name = "getChainedDocumentsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ChainedDocumentsResult>"
    )]
    pub async fn get_chained_documents_with_proof_info(
        &self,
        query: ChainedDocumentsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_chained_documents_query(self, query).await?;
        let (chained, metadata, proof) =
            ChainedDocuments::fetch_with_metadata_and_proof(self.as_ref(), query.clone(), None)
                .await?;
        let result = chained_result_to_js(&chained.unwrap_or_default(), &query)?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            result, metadata, proof,
        ))
    }
}
