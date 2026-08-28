//! `refersTo` declarations — the document-reference metadata a contract
//! carries from protocol version 14 onward.
//!
//! `refersTo` annotates an identifier property with what it points at, and
//! consensus enforces that the target exists whenever a document carrying
//! it is written. It is a **write-time constraint only**: nothing anywhere
//! in the stack resolves a reference for a reader. What this module adds is
//! the ability to *discover* the declarations — "which properties of this
//! document type are references, and to what?" — without hand-parsing the
//! contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentTypeRef,
};
use dpp::prelude::Identifier;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PROPERTY_REFERENCE_TS: &'static str = r#"
/**
 * What a `refersTo` declaration points at.
 *
 * Mirrors the `refersTo` keyword of the v3 document meta-schema, which is
 * active from protocol version 14. The field names are the schema keyword's
 * own, so what `contract.toJSON()` shows under `refersTo` and what these
 * accessors return line up key for key.
 */
export type DocumentPropertyReferenceTarget =
  | { type: 'identity' }
  | { type: 'contract' }
  | { type: 'token' }
  | {
      type: 'permanentDocument';
      /**
       * The contract the referenced document type lives in.
       *
       * Always present. When the schema omits `contractId` the declaration
       * targets the declaring contract itself, and this field reports the
       * declaring contract's own id — consensus resolves the two cases
       * identically, so a caller never has to special-case an absent value.
       * `ref.contractId.equals(contract.id)` is the self-reference test.
       */
      contractId: Identifier;
      /**
       * Name of the referenced document type. It must declare
       * `canBeDeleted: false`, which is what makes the reference
       * permanent — a target that could be deleted would leave the
       * reference dangling.
       */
      documentType: string;
      /**
       * Write-time equality bindings between the two documents:
       * `{ <referring property path>: <referenced property path> }`.
       * Consensus refuses a write whose referring property does not equal
       * the referenced document's property (code 40127). Absent — not
       * `{}`-valued — when the declaration carries none.
       */
      propertyAgreement?: Record<string, string>;
    }
  | {
      type: 'identityPublicKey';
      /**
       * Property of the same document type whose value carries the
       * referenced key id. The declaring property's own value carries the
       * identity id. A dotted path when the property is nested.
       */
      keyIdProperty: string;
    };

/**
 * A single `refersTo` declaration on a document type.
 */
export type DocumentPropertyReference = {
  /**
   * Dotted path of the declaring property within the document type — for
   * example `"author"`, or `"meta.parentId"` for a nested one.
   *
   * This is the same string consensus reports in the `path` field of the
   * document-write reference errors (codes 40120-40125). Note that contract
   * *registration* errors prefix it with the document type name
   * (`"<documentType>.<path>"`) while document *write* errors do not.
   */
  path: string;
} & DocumentPropertyReferenceTarget;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Array<DocumentPropertyReference>")]
    pub type DocumentPropertyReferenceArrayJs;

    #[wasm_bindgen(typescript_type = "Map<string, Array<DocumentPropertyReference>>")]
    pub type DocumentPropertyReferenceMapJs;
}

/// `Reflect::set` with the collection-getter error convention the `tokens`
/// and `groups` getters on `DataContract` already use.
fn set_field(target: &Object, key: &str, value: &JsValue, path: &str) -> WasmDppResult<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(|_| {
        WasmDppError::generic(format!(
            "unable to serialize the `{key}` field of the reference declared at '{path}'"
        ))
    })?;
    Ok(())
}

/// Build the flat, internally-tagged JS object for one declaration.
///
/// `declaring_contract_id` resolves the `PermanentDocument` variant's
/// absent `contract_id`, which consensus reads as "the declaring contract"
/// — it computes `contract_id.unwrap_or(contract.id())` and treats an
/// explicit self-id identically, so collapsing the two here loses nothing.
fn reference_to_js(
    path: &str,
    target: &DocumentPropertyReferenceTarget,
    declaring_contract_id: Identifier,
) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;

    let kind = match target {
        DocumentPropertyReferenceTarget::Identity => "identity",
        DocumentPropertyReferenceTarget::Contract => "contract",
        DocumentPropertyReferenceTarget::Token => "token",
        DocumentPropertyReferenceTarget::PermanentDocument { .. } => "permanentDocument",
        DocumentPropertyReferenceTarget::IdentityPublicKey { .. } => "identityPublicKey",
    };
    set_field(&object, "type", &JsValue::from_str(kind), path)?;

    match target {
        DocumentPropertyReferenceTarget::Identity
        | DocumentPropertyReferenceTarget::Contract
        | DocumentPropertyReferenceTarget::Token => {}
        DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id,
            document_type_name,
            property_agreement,
        } => {
            let effective = contract_id.unwrap_or(declaring_contract_id);
            set_field(
                &object,
                "contractId",
                &JsValue::from(IdentifierWasm::from(effective)),
                path,
            )?;
            set_field(
                &object,
                "documentType",
                &JsValue::from_str(document_type_name),
                path,
            )?;
            // `propertyAgreement` binds a referring property to a property
            // of the referenced document (consensus-enforced equality at
            // write time). Absent — not `{}`-valued — when the declaration
            // carries none, matching the schema's own omission and the
            // absent-field convention of the other optional target fields.
            if !property_agreement.is_empty() {
                let agreement = Object::new();
                for (referring, referenced) in property_agreement {
                    set_field(&agreement, referring, &JsValue::from_str(referenced), path)?;
                }
                set_field(&object, "propertyAgreement", &agreement, path)?;
            }
        }
        DocumentPropertyReferenceTarget::IdentityPublicKey { key_id_property } => {
            set_field(
                &object,
                "keyIdProperty",
                &JsValue::from_str(key_id_property),
                path,
            )?;
        }
    }

    Ok(object.into())
}

/// Collect every reference declaration of one document type, in schema
/// property order.
///
/// Walks `flattened_properties` rather than `properties` because that is
/// what both consensus validators walk, and because their error `path` is
/// built from its dotted key. Using the nested map would produce paths that
/// no consensus error matches, and would miss nested declarations entirely.
pub(crate) fn references_for_document_type(
    document_type: DocumentTypeRef<'_>,
    declaring_contract_id: Identifier,
) -> WasmDppResult<Array> {
    let references = Array::new();

    for (path, property) in document_type.flattened_properties() {
        if let DocumentPropertyType::IdentifierWithReference(target) = &property.property_type {
            references.push(&reference_to_js(path, target, declaring_contract_id)?);
        }
    }

    Ok(references)
}
