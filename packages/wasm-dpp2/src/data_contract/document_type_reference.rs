//! `refersTo` declarations — the document-reference metadata a contract
//! carries from protocol version 14 onward.
//!
//! `refersTo` annotates an identifier property with what it points at (or,
//! for an `identityPublicKey` reference with `identityProperty`, a key id
//! property with whose key it names), and consensus enforces that the target
//! exists whenever a document carrying it is written. It is a **write-time
//! constraint only**: nothing anywhere in the stack resolves a reference for
//! a reader. What this module adds is
//! the ability to *discover* the declarations — "which properties of this
//! document type are references, and to what?" — without hand-parsing the
//! contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentTypeRef, IdentityKeyReferenceRequirements,
    KeyIdReference, PropertyReference,
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
/**
 * What an `identityPublicKey` reference requires of the key it points at,
 * beyond its existence and its not being disabled.
 */
export type IdentityKeyReferenceRequirements = {
  purpose?: 'authentication' | 'encryption' | 'decryption' | 'transfer' | 'voting' | 'owner';
  boundTo?: string;
};

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
       * and last update times, and `owner: 'self'` requires the contract to
       * be owned by the writer of the referring document (its `$ownerId`),
       * `'other'` by anyone else; `readonly: true` requires a read-only
       * contract (one that can never be updated again), `keepsHistory: true`
       * one keeping its history, and `ownerProtected` an elected moderation
       * declaration whose owner protection flag has that value (code 40135
       * when any is unmet). Absent when the declaration carries no
       * requirement.
       */
      contractRequirements?: {
        moderation?: 'elected' | 'electionOpen';
        minimumAgeSeconds?: number;
        minimumSecondsSinceUpdate?: number;
        owner?: 'self' | 'other';
        readonly?: true;
        keepsHistory?: true;
        ownerProtected?: boolean;
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
      identityProperty?: never;
      /**
       * What the referenced key must be beyond existing and not being
       * disabled, checked by consensus when the referring document is
       * written against the key fetched for the existence check:
       * `purpose` requires the key's purpose to be the named one, and
       * `boundTo` requires the key's contract bounds to be exactly the
       * declaring contract and the named document type of it; a
       * whole-contract or contract group bound never meets it (code 40136
       * when either is unmet). Absent when the declaration carries no
       * requirement.
       */
      keyRequirements?: IdentityKeyReferenceRequirements;
    }
  | {
      /**
       * The inverse form, declared on the key id property itself: the
       * declaring property (an integer from 0 to 4294967295) carries the
       * key id, and `identityProperty` names whose key it is: `'$ownerId'`,
       * the document's owner, `'$creatorId'`, its creator (only on a
       * document type that records creator ids), or the dotted path of an
       * identifier property of the same document type. Consensus fetches
       * only the key (codes 40123 when it does not exist, 40124 when it is
       * disabled; 40125 when a key id is set while the named property is
       * not), and checks `keyRequirements` against it exactly as for the
       * identifier form.
       */
      type: 'identityPublicKey';
      identityProperty: '$ownerId' | '$creatorId' | (string & {});
      keyIdProperty?: never;
      /**
       * What the referenced key must be beyond existing and not being
       * disabled, checked by consensus when the referring document is
       * written against the key fetched for the existence check:
       * `purpose` requires the key's purpose to be the named one, and
       * `boundTo` requires the key's contract bounds to be exactly the
       * declaring contract and the named document type of it; a
       * whole-contract or contract group bound never meets it (code 40136
       * when either is unmet). Absent when the declaration carries no
       * requirement.
       */
      keyRequirements?: IdentityKeyReferenceRequirements;
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
   * example `"author"`, or `"meta.parentId"` for a nested one. A
   * declaration on the `items` of a typed array of identifiers, which every
   * element carries, is listed with the list path of its elements, for
   * example `"reasons[]"`; its `type` is never `identityPublicKey`, which
   * an element cannot declare.
   *
   * This is the same string consensus reports in the `path` field of the
   * document-write reference errors (codes 40120-40125, 40131, 40135 and
   * 40136), except that a write error names the failing element by its
   * index (`"reasons[2]"` for
   * the third). Note that contract *registration* errors prefix it with the
   * document type name (`"<documentType>.<path>"`, `"<documentType>.reasons[]"`)
   * while document *write* errors do not.
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

/// The flat, internally-tagged JS object for an `identityPublicKey`
/// declaration on the key id property: `identityProperty` names whose key
/// the property's value is.
fn key_id_reference_to_js(path: &str, reference: &KeyIdReference) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;
    set_field(
        &object,
        "type",
        &JsValue::from_str("identityPublicKey"),
        path,
    )?;
    set_field(
        &object,
        "identityProperty",
        &JsValue::from_str(reference.identity_property.as_str()),
        path,
    )?;
    set_key_requirements_field(&object, &reference.key_requirements, path)?;
    Ok(object.into())
}

/// The `keyRequirements` field of either `identityPublicKey` form. Absent,
/// not `{}`-valued, when the declaration requires nothing, matching the
/// schema's own omission.
fn set_key_requirements_field(
    object: &Object,
    key_requirements: &IdentityKeyReferenceRequirements,
    path: &str,
) -> WasmDppResult<()> {
    if key_requirements.is_empty() {
        return Ok(());
    }
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
    set_field(object, "keyRequirements", &fields, path)
}

/// Build the flat, internally-tagged JS object for one declaration.
fn reference_to_js(
    path: &str,
    target: &DocumentPropertyReferenceTarget,
    declaring_contract_id: Identifier,
) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;
    set_reference_target_fields(&object, target, declaring_contract_id, path)?;
    Ok(object.into())
}

/// The `DocumentPropertyReferenceTarget` of a declaration as its own JS
/// object, as a typed array element's `refersTo` reports it. `path` only
/// names the declaration in an error.
pub(crate) fn reference_target_to_js(
    target: &DocumentPropertyReferenceTarget,
    declaring_contract_id: Identifier,
    path: &str,
) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_reference_target_fields(&object, target, declaring_contract_id, path)?;
    Ok(object.into())
}

/// Set the fields of one `DocumentPropertyReferenceTarget` on `object`.
///
/// `declaring_contract_id` resolves the document variants'
/// absent `contract_id`, which consensus reads as "the declaring contract"
/// — it computes `contract_id.unwrap_or(contract.id())` and treats an
/// explicit self-id identically, so collapsing the two here loses nothing.
fn set_reference_target_fields(
    object: &Object,
    target: &DocumentPropertyReferenceTarget,
    declaring_contract_id: Identifier,
    path: &str,
) -> WasmDppResult<()> {
    let kind = match target {
        DocumentPropertyReferenceTarget::Identity => "identity",
        DocumentPropertyReferenceTarget::Contract { .. } => "contract",
        DocumentPropertyReferenceTarget::Token => "token",
        DocumentPropertyReferenceTarget::PermanentDocument { .. } => "permanentDocument",
        DocumentPropertyReferenceTarget::IdentityPublicKey { .. } => "identityPublicKey",
        DocumentPropertyReferenceTarget::DeletableDocument { .. } => "deletableDocument",
    };
    set_field(object, "type", &JsValue::from_str(kind), path)?;

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
                if let Some(owner) = contract_requirements.owner {
                    set_field(&fields, "owner", &JsValue::from_str(owner.as_str()), path)?;
                }
                for (name, flag) in [
                    ("readonly", contract_requirements.readonly),
                    ("keepsHistory", contract_requirements.keeps_history),
                    ("ownerProtected", contract_requirements.owner_protected),
                ] {
                    if let Some(flag) = flag {
                        set_field(&fields, name, &JsValue::from_bool(flag), path)?;
                    }
                }
                set_field(object, "contractRequirements", &fields, path)?;
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
                object,
                "contractId",
                &JsValue::from(IdentifierWasm::from(effective)),
                path,
            )?;
            set_field(
                object,
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
                set_field(object, "propertyAgreement", &agreement, path)?;
            }
        }
        DocumentPropertyReferenceTarget::IdentityPublicKey {
            key_id_property,
            key_requirements,
        } => {
            set_field(
                object,
                "keyIdProperty",
                &JsValue::from_str(key_id_property),
                path,
            )?;
            set_key_requirements_field(object, key_requirements, path)?;
        }
    }

    Ok(())
}

/// Collect every reference declaration of one document type, in schema
/// property order: an identifier property's own, and the one the elements
/// of a typed array of identifiers carry, listed at `path[]`.
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
        match property.property_type.reference() {
            Some(PropertyReference::KeyId(reference)) => {
                references.push(&key_id_reference_to_js(path, reference)?);
            }
            Some(PropertyReference::Value(target)) => {
                references.push(&reference_to_js(path, target, declaring_contract_id)?);
            }
            Some(PropertyReference::Elements { target, .. }) => {
                let element_path = format!("{path}[]");
                references.push(&reference_to_js(
                    &element_path,
                    target,
                    declaring_contract_id,
                )?);
            }
            None => {}
        }
    }

    Ok(references)
}
