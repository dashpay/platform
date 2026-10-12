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
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentReferenceLookup, DocumentTypeRef,
    IdentityKeyReferenceRequirements, KeyIdReference, LookupHashKey, LookupKeyParam,
    LookupKeySource, PropertyReference,
};
use dpp::document::property_names::ID;
use dpp::prelude::Identifier;
use js_sys::{Array, Object, Reflect};
use std::collections::BTreeMap;
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
 * accessors return line up key for key, with one addition: a reference
 * expression, which the schema declares by its `anyOf` or `allOf` key alone,
 * also carries `type: 'anyOf'` or `type: 'allOf'`, so every member of the
 * union is tagged by `type`.
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
       * How the referenced document is found when the property's value is
       * not its id. See {@link DocumentReferenceFindBy}. Absent when the
       * value is the referenced document's `$id`.
       */
      findBy?: DocumentReferenceFindBy;
      /**
       * What the referenced document must hold once found. See
       * {@link DocumentReferenceWhere}. Absent — not `{}`-valued — when the
       * declaration compares nothing.
       */
      where?: DocumentReferenceWhere;
      /**
       * Beside a `findBy` function only: how many blocks before the create
       * the document `findBy` finds must have been created, from its
       * `$createdAtBlockHeight`, so 1 means an earlier block (code 40142 when
       * it was not).
       */
      minimumAgeBlocks?: number;
      /**
       * A list the value must be in: the value (on a typed array, every
       * element) must be one of the identifiers the typed array `inList`
       * holds, a dotted path on `documentType`, on the document `findBy`
       * names by `{ $id: <property> }`, the identifier property holding its
       * id. Consensus fetches that document, checks `where` against it, and
       * refuses a value its list does not hold, or one set while that
       * property is not (code 40120). The list's document can never be
       * deleted and the list never changes, so a value accepted once stays an
       * element. To resolve it yourself, fetch the document by that id and
       * look in `inList`.
       */
      inList?: string;
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
       * referenced document must exist, and every `where` entry must
       * hold, when the referring document is written; it may be
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
       * How the referenced document is found when the property's value is
       * not its id. See {@link DocumentReferenceFindBy}. Absent when the
       * value is the referenced document's `$id`. With `findBy` the
       * reference means "a document with this key exists now": once the one
       * it found is deleted the key may find another, and every replace
       * re-validates it, unless a `findBy` function computes the key: that
       * commitment is judged when the document is created only.
       */
      findBy?: DocumentReferenceFindBy;
      /**
       * What the referenced document must hold once found, exactly as for
       * `permanentDocument`. Absent — not `{}`-valued — when the declaration
       * compares nothing.
       */
      where?: DocumentReferenceWhere;
      /**
       * Beside a `findBy` function only: how many blocks before the create
       * the document `findBy` finds must have been created, from its
       * `$createdAtBlockHeight`, so 1 means an earlier block (code 40142 when
       * it was not).
       */
      minimumAgeBlocks?: number;
      /**
       * Beside a `findBy` function only: the create deletes the document
       * `findBy` finds, the writer's own commitment, in the same state
       * transition.
       */
      consume?: true;
    }
  | {
      /**
       * A document of a type whose documents leave state only when the
       * contract's moderators remove them, each removal leaving a removal
       * record under the contract (`canBeDeleted: false`, no `ttl`, and
       * `moderatorAbilities.delete` keeping records). Between
       * `permanentDocument` and `deletableDocument`, disjoint from both. The
       * referenced document must exist, and every `where` entry must hold,
       * when the referring document is written; afterwards the reference
       * resolves to the document or, once a moderator removed it, to its
       * removal record, which a restore can bring the document back from. A
       * replace keeps it, re-validating it only when the value or a property
       * `where` reads changes (every replace for a `where` value `$ownerId`),
       * as for `permanentDocument`.
       */
      type: 'moderatedDocument';
      /**
       * The contract the referenced document type lives in. Always
       * present, resolved exactly as for `permanentDocument`.
       */
      contractId: Identifier;
      /**
       * Name of the referenced document type; its documents must leave
       * state only through a moderator's recorded removal.
       */
      documentType: string;
      /**
       * What the referenced document must hold, exactly as for
       * `permanentDocument`. Absent — not `{}`-valued — when the declaration
       * compares nothing.
       */
      where?: DocumentReferenceWhere;
    }
  | DocumentPropertyReferenceExpression;

/**
 * A reference expression, declared as `refersTo: { anyOf: [...] }` or
 * `refersTo: { allOf: [...] }`. The operands sit under the combinator's own
 * key, as in the schema, and `type` names the combinator, so the union stays
 * internally tagged like every other: `switch (reference.type)` sees
 * `'anyOf'` and `'allOf'` next to the target kinds. An `anyOf` holds when at
 * least one operand holds: consensus checks the operands in this order,
 * stops at the first that holds, and when none does refuses the write with
 * the error of the last. An `allOf` holds when every operand holds for the
 * same value: consensus stops at the first that fails and refuses the write
 * with its error. Operands nest, the other combinator inside, up to the
 * protocol's depth limit (4 from protocol version 14).
 */
export type DocumentPropertyReferenceExpression =
  | { type: 'anyOf'; anyOf: Array<DocumentPropertyReferenceOperand> }
  | { type: 'allOf'; allOf: Array<DocumentPropertyReferenceOperand> };

/**
 * One operand of a reference expression: a leaf, an `identity`, a
 * `permanentDocument` (by id, by `findBy` or with `inList`) or a
 * `deletableDocument` found by `findBy`, with its own fields, a `where`
 * belonging to its own leaf, or a nested expression.
 */
export type DocumentPropertyReferenceOperand =
  | Extract<
      DocumentPropertyReferenceTarget,
      { type: 'identity' | 'permanentDocument' | 'deletableDocument' }
    >
  | DocumentPropertyReferenceExpression;

/**
 * How a document reference finds the referenced document when the value is
 * not its id: every property of one unique index of the referenced document
 * type, by its name there (`$ownerId` among the system ones), mapped to where
 * its value comes from: a property path of the referring document type,
 * `'$ownerId'` for the referring document's owner, `'.'` for the value of the
 * property that carries the reference (exactly once), or a function computing
 * it (see {@link DocumentReferenceFindByFunction}). The index is the unique one
 * whose properties are exactly these, and the reference holds if the document
 * it finds exists (code 40120 when it does not). To resolve a reference
 * yourself, query the referenced type by those values: at most one document
 * matches. On a `permanentDocument` reference the key cannot move off the
 * document it found, so it keeps resolving; on a `deletableDocument` reference
 * it may find nothing, or a later document with the same key, once the one it
 * found is deleted. With `inList` it is `{ $id: <property> }`: the document
 * holding the list, by the id that property holds.
 */
export type DocumentReferenceFindBy = Record<string, string | DocumentReferenceFindByFunction>;

/**
 * What a referenced document must hold once found: `{ <referenced property>:
 * <referring value> }`, each an equality consensus enforces when the referring
 * document is written (code 40127 when it fails). The key is a property path
 * of the referenced document type, or its `$ownerId`, `$creatorId` or `$id`
 * (current owner, original creator, id) against an identifier on the
 * referring side. The value is a property path of the referring document
 * type, or `'$ownerId'`, the writer, which makes the entry a write gate: only
 * an identity equal to the referenced side may create the document, and every
 * replace re-checks it. Beside a `findBy` function the whole declaration is
 * judged when the document is created only, so no replace re-checks it.
 */
export type DocumentReferenceWhere = Record<string, string>;

/**
 * A `findBy` function, for a commit and reveal, keyed by the referenced
 * property: the document is found by that property holding the hash
 * `function` (`sys.hash.sha256d`, SHA-256 of SHA-256) of the `params`' bytes
 * joined in order with nothing between them. A param is a property path of the
 * document being created (a string's UTF-8, a byte array's bytes, an
 * identifier's 32 bytes), the property carrying the reference included,
 * `{ const }`, fixed UTF-8 text, or `'.'`, the value carrying the reference
 * where it has no path (each element of a typed array, the writer or the
 * creator). The document it finds is a commitment made earlier; the reference
 * is checked when the document is created only. To reveal one, compute the
 * same hash over the values you create the document with.
 */
export type DocumentReferenceFindByFunction = {
  function: 'sys.hash.sha256d';
  params: Array<string | { const: string }>;
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
   * The document type's `ownerRefersTo` declaration, whose value is the
   * document's `$ownerId`, the writer, is listed first with the path
   * `"$ownerId"`, which is not a property path: the value it checks is the
   * document's owner. Consensus checks it with the writer's id when a
   * document is created, and when a replace changes a property its `findBy`
   * or `where` reads (every replace for a `deletableDocument` found by
   * `findBy`, which gates the writer on its document still existing); in its
   * `findBy`, `'.'` is the writer. Its `type` is `identity`, a
   * `permanentDocument` or `deletableDocument` with `findBy`, or a
   * `permanentDocument` with `inList`, the targets a writer can be, on a document type whose
   * documents can be neither transferred nor traded. On a type whose documents can, a
   * `creatorRefersTo` declaration takes its place, listed first with the
   * path `"$creatorId"`: the same, with the document's creator, who never
   * changes, as the value, and a `deletableDocument` only with a `findBy`
   * function (see {@link DocumentReferenceFindByFunction}).
   *
   * This is the same string consensus reports in the `path` field of the
   * document-write reference errors (codes 40120-40125, 40131, 40135,
   * 40136 and 40142), except that a write error names the failing element by its
   * index (`"reasons[2]"` for
   * the third). Note that contract *registration* errors prefix it with the
   * document type name (`"<documentType>.<path>"`, `"<documentType>.reasons[]"`)
   * and name one leaf of a reference expression by where it sits
   * (`"<documentType>.<path>.anyOf[1].allOf[0]"`), while document *write*
   * errors do neither.
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

/// A `findBy` function as the schema spells it: `{ function, params }`.
fn find_by_function_to_js(key: &LookupHashKey, path: &str) -> WasmDppResult<JsValue> {
    let computed = Object::new();
    set_field(
        &computed,
        "function",
        &JsValue::from_str(key.function.as_str()),
        path,
    )?;
    let params = Array::new();
    for param in &key.params {
        let param_value = match param {
            LookupKeyParam::ReferenceValue => JsValue::from_str("."),
            LookupKeyParam::Property(property) => JsValue::from_str(property),
            LookupKeyParam::Const(text) => {
                let object = Object::new();
                set_field(&object, "const", &JsValue::from_str(text), path)?;
                object.into()
            }
        };
        params.push(&param_value);
    }
    set_field(&computed, "params", &params, path)?;
    Ok(computed.into())
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

/// The `where` field of a document reference: `{ referenced property:
/// referring property }`, a consensus-enforced equality at write time, keyed by
/// the referenced side as the schema spells it (the parsed model keys it the
/// other way). `find_by_entry` leaves out the entry `findBy` reads instead, a
/// list's `$id`. Absent, not `{}`-valued, when the declaration compares
/// nothing, matching the schema's own omission and the absent-field convention
/// of the other optional target fields.
fn set_where_field(
    object: &Object,
    property_agreement: &BTreeMap<String, String>,
    find_by_entry: Option<&str>,
    path: &str,
) -> WasmDppResult<()> {
    let entries: Vec<(&String, &String)> = property_agreement
        .iter()
        .filter(|(_, referenced)| Some(referenced.as_str()) != find_by_entry)
        .collect();
    if entries.is_empty() {
        return Ok(());
    }
    let where_object = Object::new();
    for (referring, referenced) in entries {
        set_field(
            &where_object,
            referenced,
            &JsValue::from_str(referring),
            path,
        )?;
    }
    set_field(object, "where", &where_object, path)
}

/// The `findBy` field of a document reference found by it: every property it
/// names mapped to its source as the schema spells it, `'.'`, `'$ownerId'`, a
/// property path, or a function as `{ function, params }`, and, beside a
/// function, `minimumAgeBlocks` and `consume` where declared.
fn set_find_by_fields(
    object: &Object,
    lookup: &DocumentReferenceLookup,
    path: &str,
) -> WasmDppResult<()> {
    let find_by = Object::new();
    for (index_property, source) in &lookup.keys {
        let value = match source {
            LookupKeySource::Hash(key) => find_by_function_to_js(key, path)?,
            source => JsValue::from_str(source.as_str()),
        };
        set_field(&find_by, index_property, &value, path)?;
    }
    set_field(object, "findBy", &find_by, path)?;
    if let Some(blocks) = lookup.minimum_age_blocks {
        set_field(object, "minimumAgeBlocks", &JsValue::from(blocks), path)?;
    }
    if lookup.consume {
        set_field(object, "consume", &JsValue::TRUE, path)?;
    }
    Ok(())
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
    // The operands sit under the combinator's own key, as in the schema,
    // each an object of its own, and `type` names the combinator, so the
    // union stays internally tagged (CONVENTIONS.md, "Tagged unions")
    if let Some((combinator, operands)) = target.combinator() {
        let objects = Array::new();
        for operand in operands.operands() {
            objects.push(&reference_target_to_js(
                operand,
                declaring_contract_id,
                path,
            )?);
        }
        let name = combinator.wire_name();
        set_field(object, "type", &JsValue::from_str(name), path)?;
        return set_field(object, name, &objects, path);
    }

    let kind = match target {
        DocumentPropertyReferenceTarget::Identity => "identity",
        DocumentPropertyReferenceTarget::Contract { .. } => "contract",
        DocumentPropertyReferenceTarget::Token => "token",
        // A list the value must be in is a `permanentDocument` with `inList`
        DocumentPropertyReferenceTarget::PermanentDocument { .. }
        | DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
        | DocumentPropertyReferenceTarget::ListElement(_) => "permanentDocument",
        DocumentPropertyReferenceTarget::IdentityPublicKey { .. } => "identityPublicKey",
        DocumentPropertyReferenceTarget::DeletableDocument { .. }
        | DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. } => "deletableDocument",
        DocumentPropertyReferenceTarget::ModeratedDocument { .. } => "moderatedDocument",
        DocumentPropertyReferenceTarget::AnyOf(_) | DocumentPropertyReferenceTarget::AllOf(_) => {
            return Err(WasmDppError::generic(format!(
                "the reference expression declared at '{path}' has no single target kind"
            )));
        }
    };
    set_field(object, "type", &JsValue::from_str(kind), path)?;

    match target {
        DocumentPropertyReferenceTarget::Identity | DocumentPropertyReferenceTarget::Token => {}
        // Handled above, before the kind
        DocumentPropertyReferenceTarget::AnyOf(_) | DocumentPropertyReferenceTarget::AllOf(_) => {}
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
        | DocumentPropertyReferenceTarget::PermanentDocumentLookup {
            contract_id,
            document_type_name,
            property_agreement,
            ..
        }
        | DocumentPropertyReferenceTarget::DeletableDocument {
            contract_id,
            document_type_name,
            property_agreement,
        }
        | DocumentPropertyReferenceTarget::DeletableDocumentLookup {
            contract_id,
            document_type_name,
            property_agreement,
            ..
        }
        | DocumentPropertyReferenceTarget::ModeratedDocument {
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
            // Present only on a reference found by `findBy`, absent when the
            // value is the referenced document's id, as the schema omits it
            if let DocumentPropertyReferenceTarget::PermanentDocumentLookup { lookup, .. }
            | DocumentPropertyReferenceTarget::DeletableDocumentLookup { lookup, .. } = target
            {
                set_find_by_fields(object, lookup, path)?;
            }
            set_where_field(object, property_agreement, None, path)?;
        }
        // A document found by its `$id` read from another property, whose list
        // the value must be in: the fields of `permanentDocument`, `findBy`
        // naming the `$id`, plus `inList`
        DocumentPropertyReferenceTarget::ListElement(reference) => {
            let effective = reference.contract_id.unwrap_or(declaring_contract_id);
            set_field(
                object,
                "contractId",
                &JsValue::from(IdentifierWasm::from(effective)),
                path,
            )?;
            set_field(
                object,
                "documentType",
                &JsValue::from_str(&reference.document_type_name),
                path,
            )?;
            if let Some(document_id_property) = reference.document_id_property() {
                let find_by = Object::new();
                set_field(&find_by, ID, &JsValue::from_str(document_id_property), path)?;
                set_field(object, "findBy", &find_by, path)?;
            }
            set_where_field(object, &reference.property_agreement, Some(ID), path)?;
            set_field(
                object,
                "inList",
                &JsValue::from_str(&reference.in_list),
                path,
            )?;
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

/// Collect every reference declaration of one document type: its
/// `ownerRefersTo` or `creatorRefersTo` first, listed at `$ownerId` or
/// `$creatorId`, the path consensus names it by, then in schema property
/// order an identifier property's own, and the
/// one the elements of a typed array of identifiers carry, listed at
/// `path[]`.
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

    for (holder, reference) in document_type.reference_declarations() {
        let path = holder.path();
        match reference {
            PropertyReference::KeyId(reference) => {
                references.push(&key_id_reference_to_js(path, reference)?);
            }
            PropertyReference::Value(target) | PropertyReference::Revealed(target) => {
                references.push(&reference_to_js(path, target, declaring_contract_id)?);
            }
            PropertyReference::Elements { target, .. } => {
                let element_path = format!("{path}[]");
                references.push(&reference_to_js(
                    &element_path,
                    target,
                    declaring_contract_id,
                )?);
            }
        }
    }

    Ok(references)
}
