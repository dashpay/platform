//! `encryptedFor` declarations: how a byte array property's ciphertext was
//! produced, which a contract carries from protocol version 14 onward.
//!
//! `encryptedFor` annotates a byte array property with the recipient, the two
//! key ids and the scheme its bytes were encrypted under, so a wallet reads
//! the recipe from the contract instead of a side channel. Consensus checks
//! the shape of the bytes on every create and replace (code 10420) and
//! nothing more. What this module adds is the ability to *discover* the
//! declarations, "which properties of this document type are encrypted, and
//! with what?", without hand-parsing the contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::EncryptedFor;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PROPERTY_ENCRYPTION_TS: &'static str = r#"
/**
 * The scheme an `encryptedFor` property's bytes were produced under.
 *
 * `ecdh-secp256k1-aes256-cbc` is the scheme of the dashpay contact request:
 * the shared key is the libsecp256k1 ECDH of the sender's private key and
 * the recipient's public key (SHA256 of the product point's parity byte and
 * x coordinate), and the value is a random 16-byte IV followed by the
 * plaintext under AES-256-CBC with PKCS7 padding and that IV. A ciphertext
 * is therefore at least 32 bytes and always a multiple of 16.
 */
export type EncryptionScheme = 'ecdh-secp256k1-aes256-cbc';

/**
 * A single `encryptedFor` declaration on a document type.
 *
 * Mirrors the `encryptedFor` keyword of the v3 document meta-schema, which
 * is active from protocol version 14. The field names are the schema
 * keyword's own, so what `contract.toJSON()` shows under `encryptedFor` and
 * what these accessors return line up key for key.
 */
export type DocumentPropertyEncryption = {
  /**
   * Dotted path of the declaring byte array property within the document
   * type, for example `"encryptedMessage"`, or `"meta.blob"` for a nested
   * one. This is the string consensus reports in the `property` field of
   * the shape error (code 10420).
   */
  path: string;
  /**
   * Whose keys decrypt the bytes: the dotted path of an identifier property
   * of the same document type whose value is the recipient identity's id,
   * or `"$ownerId"` for a message the writer encrypts to themself.
   */
  recipient: string;
  /**
   * Dotted path of the integer property of the same document type carrying
   * the id of the recipient's key.
   */
  recipientKey: string;
  /**
   * Dotted path of the integer property of the same document type carrying
   * the id of the sender's key.
   */
  senderKey: string;
  /** How the bytes were produced. */
  scheme: EncryptionScheme;
};
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Array<DocumentPropertyEncryption>")]
    pub type DocumentPropertyEncryptionArrayJs;

    #[wasm_bindgen(typescript_type = "Map<string, Array<DocumentPropertyEncryption>>")]
    pub type DocumentPropertyEncryptionMapJs;
}

/// `Reflect::set` with the collection-getter error convention the `tokens`
/// and `groups` getters on `DataContract` already use.
fn set_field(target: &Object, key: &str, value: &JsValue, path: &str) -> WasmDppResult<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(|_| {
        WasmDppError::generic(format!(
            "unable to serialize the `{key}` field of the encryption declared at '{path}'"
        ))
    })?;
    Ok(())
}

/// Build the flat JS object for one declaration.
fn encryption_to_js(path: &str, encrypted_for: &EncryptedFor) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;
    set_field(
        &object,
        "recipient",
        &JsValue::from_str(encrypted_for.recipient.as_path()),
        path,
    )?;
    set_field(
        &object,
        "recipientKey",
        &JsValue::from_str(&encrypted_for.recipient_key),
        path,
    )?;
    set_field(
        &object,
        "senderKey",
        &JsValue::from_str(&encrypted_for.sender_key),
        path,
    )?;
    set_field(
        &object,
        "scheme",
        &JsValue::from_str(encrypted_for.scheme.as_str()),
        path,
    )?;
    Ok(object.into())
}

/// Collect every `encryptedFor` declaration of one document type, in schema
/// property order, keyed by the dotted path consensus reports.
pub(crate) fn encryptions_for_document_type(
    document_type: DocumentTypeRef<'_>,
) -> WasmDppResult<Array> {
    let encryptions = Array::new();

    for (path, encrypted_for) in document_type.encrypted_properties() {
        encryptions.push(&encryption_to_js(path, encrypted_for)?);
    }

    Ok(encryptions)
}
