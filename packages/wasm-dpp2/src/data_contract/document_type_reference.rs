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
  | {
      type: 'contract';
      /**
       * What the referenced contract must declare beyond existing, checked
       * by consensus when the referring document is written against the
       * contract fetched for the existence check and the block time:
       * `moderation: 'elected'` requires an elected moderation team and
       * `'electionOpen'` one whose own `electionDelay` has passed since the
       * contract's creation (or which declares none),
       * `minimumAgeSeconds` requires the contract's recorded creation time
       * to be at least that many seconds before the block time of the write,
       * and `minimumSecondsSinceUpdate` the same of the later of its creation
       * and last update times (code 40135 when any is unmet). Absent when
       * the declaration carries no requirement.
       */
      contractRequirements?: {
        moderation?: 'elected' | 'electionOpen';
        minimumAgeSeconds?: number;
        minimumSecondsSinceUpdate?: number;
      };
    }
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
       * the referenced document's property (code 40127). The referenced
       * side may also be `$ownerId` or `$creatorId`, the referenced
       * document's current owner or original creator, against an
       * identifier property on the referring side. The referring side may
       * be the writer's own `$ownerId`, which makes the pair a write gate:
       * only an identity equal to the referenced side may create the
       * document, and every replace re-checks it. Absent — not
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
      /**
       * What the referenced key must be beyond existing and not being
       * disabled, checked by consensus when the referring document is
       * written against the key fetched for the existence check:
       * `purpose` requires the key's purpose to be the named one, and
       * `boundTo` requires the key's contract bounds to be exactly the
       * declaring contract and the named document type of it (code 40136
       * when either is unmet). Absent when the declaration carries no
       * requirement.
       */
      keyRequirements?: {
        purpose?:
          | 'authentication'
          | 'encryption'
          | 'decryption'
          | 'transfer'
          | 'voting'
          | 'owner';
        boundTo?: string;
      };
    }
  | {
      /**
       * A document of a type whose documents CAN be deleted (the
       * counterpart of `permanentDocument`, disjoint from it). The
       * referenced document must exist, and every `propertyAgreement`
       * pair must hold, when the referring document is written; it may be
       * deleted afterwards, so a reader must expect the reference to
       * resolve to nothing. Every replace of the referring document
       * re-validates it: a dead reference has to be repointed at an
       * existing document or cleared.
       */
      type: 'deletableDocument';
      /**
       * The contract the referenced document type lives in. Always
       * present, resolved exactly as for `permanentDocument`.
       */
      contractId: Identifier;
      /** Name of the referenced document type; it must allow deletion. */
      documentType: string;
      /**
       * Write-time equality bindings, exactly as for `permanentDocument`.
       * Absent — not `{}`-valued — when the declaration carries none.
       */
      propertyAgreement?: Record<string, string>;
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
   * document-write reference errors (codes 40120-40125, 40131, 40135 and
   * 40136). Note that contract
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
/// `declaring_contract_id` resolves the document variants'
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
        DocumentPropertyReferenceTarget::Contract { .. } => "contract",
        DocumentPropertyReferenceTarget::Token => "token",
        DocumentPropertyReferenceTarget::PermanentDocument { .. } => "permanentDocument",
        DocumentPropertyReferenceTarget::IdentityPublicKey { .. } => "identityPublicKey",
        DocumentPropertyReferenceTarget::DeletableDocument { .. } => "deletableDocument",
    };
    set_field(&object, "type", &JsValue::from_str(kind), path)?;

    match target {
        DocumentPropertyReferenceTarget::Identity | DocumentPropertyReferenceTarget::Token => {}
        DocumentPropertyReferenceTarget::Contract {
            contract_requirements,
        } => {
            // Absent, not `{}`-valued, when the declaration requires nothing,
            // matching the schema's own omission.
            if !contract_requirements.is_empty() {
                let fields = Object::new();
                if let Some(moderation) = contract_requirements.moderation {
                    set_field(
                        &fields,
                        "moderation",
                        &JsValue::from_str(moderation.as_str()),
                        path,
                    )?;
                }
                if let Some(seconds) = contract_requirements.minimum_age_seconds {
                    set_field(
                        &fields,
                        "minimumAgeSeconds",
                        &JsValue::from_f64(f64::from(seconds)),
                        path,
                    )?;
                }
                if let Some(seconds) = contract_requirements.minimum_seconds_since_update {
                    set_field(
                        &fields,
                        "minimumSecondsSinceUpdate",
                        &JsValue::from_f64(f64::from(seconds)),
                        path,
                    )?;
                }
                set_field(&object, "contractRequirements", &fields, path)?;
            }
        }
        DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id,
            document_type_name,
            property_agreement,
        }
        | DocumentPropertyReferenceTarget::DeletableDocument {
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
        DocumentPropertyReferenceTarget::IdentityPublicKey {
            key_id_property,
            key_requirements,
        } => {
            set_field(
                &object,
                "keyIdProperty",
                &JsValue::from_str(key_id_property),
                path,
            )?;
            // Absent, not `{}`-valued, when the declaration requires nothing,
            // matching the schema's own omission.
            if !key_requirements.is_empty() {
                let fields = Object::new();
                if let Some(purpose) = key_requirements.purpose {
                    set_field(
                        &fields,
                        "purpose",
                        &JsValue::from_str(purpose.wire_name()),
                        path,
                    )?;
                }
                if let Some(document_type_name) = &key_requirements.bound_to {
                    set_field(
                        &fields,
                        "boundTo",
                        &JsValue::from_str(document_type_name),
                        path,
                    )?;
                }
                set_field(&object, "keyRequirements", &fields, path)?;
            }
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
