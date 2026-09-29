//! The GroveDB layout of a document type, computed by Drive
//! (`drive::drive::document::layout`) with the rules its index walkers use.

use crate::error::WasmSdkError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::version::PlatformVersion;
use drive::drive::document::layout::document_type_layout as drive_document_type_layout;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_dpp2::data_contract::model::DataContractWasm;
use wasm_dpp2::serialization::conversions::platform_value_to_object;
use wasm_dpp2::version::{PlatformVersionLikeJs, PlatformVersionWasm};

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TYPE_LAYOUT_TS: &'static str = r#"
/**
 * The GroveDB layout of one document type: every tree and element Drive
 * writes under `[64, contract id, 1, document type name]`.
 */
export interface DocumentTypeLayout {
  documentType: string;
  root: DocumentTypeLayoutNode;
}

/** One layer of a document type's layout. */
export interface DocumentTypeLayoutNode {
  /** The key, or the family of keys this node stands for. */
  key:
    | { kind: 'fixed'; hex: string; label: string }
    | { kind: 'documentId' }
    | { kind: 'revisionTime' }
    | { kind: 'propertyValue'; property: string }
    | { kind: 'timeRangeBucket'; property: string; rangeSeconds: number; stepSeconds: number; phaseSeconds: number }
    | { kind: 'integerRangeBucket'; property: string; range: number; step: number; phase: number }
    | { kind: 'memberKey'; components: string[] };
  role:
    | 'documentType' | 'primaryKey' | 'document' | 'latestRevision' | 'revision'
    | 'indexProperty' | 'indexValue' | 'nextIndexProperty' | 'terminal' | 'member';
  /** The id of the node of Drive's structure description (grovedb-structure.json) this layer is an instance of. */
  structureNode: string;
  /** The element kind: 'Tree', 'CountTree', 'ProvableCountTree', …, 'Item', 'Reference', …. */
  element: string;
  /** The wrapper that makes the tree contribute nothing to the aggregating tree above it, when there is one. */
  wrapper?: 'NonCounted' | 'NotSummed' | 'NotCountedOrSummed';
  /** The ranking axes of an indexed tree. */
  rankedAxes: Array<'count' | 'sum' | 'avg'>;
  /** The indexes of the document type that use this layer. */
  indexes: string[];
  notes: Array<{ code: string; text: string }>;
  /** What Drive writes at this key instead in some cases, and when. */
  alternative?: { when: string; node: DocumentTypeLayoutNode };
  children: DocumentTypeLayoutNode[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentTypeLayout")]
    pub type DocumentTypeLayoutJs;
}

/// The GroveDB layout of the document type `documentTypeName` of `contract`:
/// the document type tree, the documents by id and, for each index, the
/// property and value trees down to where the index ends, with the element
/// or tree type Drive writes at each (a count or sum tree, a ranked indexed
/// tree, a zero-contribution wrapper, a reference or an indexOnly item).
///
/// Computed from the contract alone, with the functions Drive's index walkers
/// use, so it needs no network. Those are the walkers of protocol version 14
/// on; an earlier `platformVersion` is refused. Each node names the node of Drive's structure
/// description it is an instance of (`structureNode`), for linking to the
/// GroveDB structure viewer.
#[wasm_bindgen(js_name = "documentTypeLayout")]
pub fn document_type_layout(
    contract: &DataContractWasm,
    #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
    #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
) -> Result<DocumentTypeLayoutJs, WasmSdkError> {
    let platform_version: PlatformVersion = PlatformVersionWasm::try_from(platform_version)?.into();
    let document_type = contract
        .as_ref()
        .document_type_for_name(&document_type_name)
        .map_err(|_| {
            WasmSdkError::invalid_argument(format!(
                "document type '{document_type_name}' not found in contract"
            ))
        })?;
    let layout = drive_document_type_layout(document_type, &platform_version)
        .map_err(|error| WasmSdkError::generic(format!("document type layout: {error}")))?;
    Ok(platform_value_to_object(&layout.to_value())?.into())
}
