//! Encrypting and decrypting the byte properties a document type declares `encryptedFor`
//! (`WasmSdk.encryptDocumentProperty`, `WasmSdk.decryptDocumentProperty`,
//! `WasmSdk.encryptedPropertyEnvelope`): thin bindings over
//! `dash_sdk::platform::encrypted_for`, which reads the declaration from the contract.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::document::DocumentV0Getters;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::platform::encrypted_for::{
    decrypt_property, encrypt_property, EncryptedPropertyEnvelope, EncryptionKeys,
};
use dash_sdk::platform::{Document, Identifier};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::public_key::IdentityPublicKeyWasm;
use wasm_dpp2::serialization::conversions::platform_value_to_object;
use wasm_dpp2::utils::{try_from_options_with, try_to_bytes, try_to_string};
use wasm_dpp2::{DataContractWasm, PrivateKeyWasm};

#[wasm_bindgen(typescript_custom_section)]
const ENCRYPTED_FOR_TS: &'static str = r#"
/**
 * Options for `WasmSdk.encryptDocumentProperty`: encrypt a message into a byte property whose
 * schema declares `encryptedFor`, the way the declaration says.
 */
export interface EncryptDocumentPropertyOptions {
  /** The contract that declares the document type. */
  dataContract: DataContract;
  /** The document type. */
  documentTypeName: string;
  /** The property declaring `encryptedFor`, such as `encryptedMessage`; dotted when nested. */
  property: string;
  /** The message. A string is encoded as UTF-8. */
  plaintext: Uint8Array | string;
  /** The writer's key; its id goes into the declaration's `senderKey` property. */
  senderKey: IdentityPublicKey;
  /** The private half of `senderKey`. */
  senderPrivateKey: PrivateKey;
  /** The recipient's key; its id goes into the declaration's `recipientKey` property. */
  recipientKey: IdentityPublicKey;
}

/**
 * Options for `WasmSdk.decryptDocumentProperty`. The recipient decrypts with its private key
 * and the sender's key; the sender can read its own message back with its private key and the
 * recipient's key.
 */
export interface DecryptDocumentPropertyOptions {
  /** The contract that declares the document's type. */
  dataContract: DataContract;
  /** The document holding the ciphertext; its type is `document.documentTypeName`. */
  document: Document;
  /** The property declaring `encryptedFor`. */
  property: string;
  /** The private half of the recipient's key, the one `recipientKey` names. */
  recipientPrivateKey: PrivateKey;
  /** The sender's key, the one `senderKey` names (see `encryptedPropertyEnvelope`). */
  senderKey: IdentityPublicKey;
}

/** Options for `WasmSdk.encryptedPropertyEnvelope`. */
export interface EncryptedPropertyEnvelopeOptions {
  /** The contract that declares the document's type. */
  dataContract: DataContract;
  /** The document; its type is `document.documentTypeName`. */
  document: Document;
  /** The property declaring `encryptedFor`. */
  property: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "EncryptDocumentPropertyOptions")]
    pub type EncryptDocumentPropertyOptionsJs;

    #[wasm_bindgen(typescript_type = "DecryptDocumentPropertyOptions")]
    pub type DecryptDocumentPropertyOptionsJs;

    #[wasm_bindgen(typescript_type = "EncryptedPropertyEnvelopeOptions")]
    pub type EncryptedPropertyEnvelopeOptionsJs;
}

/// Whose keys an encrypted property of a stored document is under: what a reader fetches to
/// decrypt it.
#[wasm_bindgen(js_name = "EncryptedPropertyEnvelope")]
#[derive(Clone)]
pub struct EncryptedPropertyEnvelopeWasm(EncryptedPropertyEnvelope);

#[wasm_bindgen(js_class = EncryptedPropertyEnvelope)]
impl EncryptedPropertyEnvelopeWasm {
    /// The identity the bytes are encrypted for.
    #[wasm_bindgen(getter = recipientId)]
    pub fn recipient_id(&self) -> IdentifierWasm {
        self.0.recipient_id.into()
    }

    /// The id of the recipient's key, from the declaration's `recipientKey` property.
    #[wasm_bindgen(getter = recipientKeyId)]
    pub fn recipient_key_id(&self) -> u32 {
        self.0.recipient_key_id
    }

    /// The identity whose key `senderKeyId` names: the document owner, the writer that
    /// encrypted the bytes. A document whose owner may have changed since (transferred or
    /// sold) is refused rather than given a sender that did not encrypt it.
    #[wasm_bindgen(getter = senderId)]
    pub fn sender_id(&self) -> IdentifierWasm {
        self.0.sender_id.into()
    }

    /// The id of the sender's key, from the declaration's `senderKey` property.
    #[wasm_bindgen(getter = senderKeyId)]
    pub fn sender_key_id(&self) -> u32 {
        self.0.sender_key_id
    }
}

/// The message of an options object's `field`: a Uint8Array, or a string encoded as UTF-8.
pub(crate) fn message_from_options(
    options: &JsValue,
    field: &str,
) -> Result<Vec<u8>, WasmSdkError> {
    Ok(try_from_options_with(options, field, |value| {
        if value.is_string() {
            try_to_string(value, field).map(String::into_bytes)
        } else {
            try_to_bytes(value.clone(), field)
        }
    })?)
}

/// The secp256k1 public key of an identity key, refused unless it is an `ECDSA_SECP256K1` key,
/// the only type the scheme's ECDH takes.
fn secp256k1_public_key(key: &IdentityPublicKey, field: &str) -> Result<PublicKey, WasmSdkError> {
    if key.key_type() != KeyType::ECDSA_SECP256K1 {
        return Err(WasmSdkError::invalid_argument(format!(
            "{field} must be an ECDSA_SECP256K1 key, not {:?}",
            key.key_type()
        )));
    }
    PublicKey::from_slice(key.data().as_slice())
        .map_err(|e| WasmSdkError::invalid_argument(format!("{field}: {e}")))
}

fn secret_key_from_options(options: &JsValue, field: &str) -> Result<SecretKey, WasmSdkError> {
    Ok(PrivateKeyWasm::try_from_options(options, field)?
        .inner()
        .inner)
}

/// `document` with its properties coerced to the types its document type declares: a document
/// built in JavaScript without its contract holds a byte array or an identifier as a list of
/// numbers.
fn sanitized(document_type: DocumentTypeRef<'_>, mut document: Document) -> Document {
    document_type.sanitize_document_properties(document.properties_mut());
    document
}

fn document_type_name_of(
    contract: &DataContract,
    document: &DocumentWasm,
) -> Result<String, WasmSdkError> {
    if Identifier::from(document.data_contract_id()) != contract.id() {
        return Err(WasmSdkError::invalid_argument(
            "document is not a document of dataContract",
        ));
    }
    Ok(document.document_type_name())
}

#[wasm_bindgen]
impl WasmSdk {
    /// Encrypts a message into a byte property whose schema declares `encryptedFor`, the way
    /// the declaration says (the scheme dashpay contact requests use: ECDH on secp256k1, a
    /// random 16-byte IV, AES-256-CBC).
    ///
    /// @returns The properties to set on the document: the ciphertext at `property` and the
    /// two key ids at the declaration's `recipientKey` and `senderKey` paths, nested like the
    /// paths. The recipient property is the caller's to set.
    #[wasm_bindgen(
        js_name = "encryptDocumentProperty",
        unchecked_return_type = "Record<string, unknown>"
    )]
    pub fn encrypt_document_property(
        options: EncryptDocumentPropertyOptionsJs,
    ) -> Result<JsValue, WasmSdkError> {
        let options: JsValue = options.into();
        let contract: DataContract =
            DataContractWasm::try_from_options(&options, "dataContract")?.into();
        let document_type_name: String =
            try_from_options_with(&options, "documentTypeName", |value| {
                try_to_string(value, "documentTypeName")
            })?;
        let property: String = try_from_options_with(&options, "property", |value| {
            try_to_string(value, "property")
        })?;
        let plaintext = message_from_options(&options, "plaintext")?;
        let sender_key: IdentityPublicKey =
            IdentityPublicKeyWasm::try_from_options(&options, "senderKey")?.into();
        let sender_private_key = secret_key_from_options(&options, "senderPrivateKey")?;
        let recipient_key: IdentityPublicKey =
            IdentityPublicKeyWasm::try_from_options(&options, "recipientKey")?.into();

        // A private key that is not the sender key's would write a message nobody can read
        let sender_public_key = secp256k1_public_key(&sender_key, "senderKey")?;
        if PublicKey::from_secret_key(&Secp256k1::signing_only(), &sender_private_key)
            != sender_public_key
        {
            return Err(WasmSdkError::invalid_argument(
                "senderPrivateKey is not the private half of senderKey",
            ));
        }
        let keys = EncryptionKeys {
            sender_key_id: sender_key.id(),
            sender_private_key: &sender_private_key,
            recipient_key_id: recipient_key.id(),
            recipient_public_key: secp256k1_public_key(&recipient_key, "recipientKey")?,
        };

        let document_type = contract
            .document_type_for_name(&document_type_name)
            .map_err(|e| WasmSdkError::invalid_argument(e.to_string()))?;
        let mut properties = BTreeMap::new();
        encrypt_property(document_type, &property, &plaintext, &keys, &mut properties)?;
        Ok(platform_value_to_object(&Value::from(properties))?)
    }

    /// Decrypts a byte property whose schema declares `encryptedFor`.
    ///
    /// The scheme carries no authentication tag: a wrong key is caught only by the padding
    /// check, which it passes about once in 256 attempts, returning garbage.
    ///
    /// @returns The message.
    #[wasm_bindgen(js_name = "decryptDocumentProperty")]
    pub fn decrypt_document_property(
        options: DecryptDocumentPropertyOptionsJs,
    ) -> Result<Vec<u8>, WasmSdkError> {
        let options: JsValue = options.into();
        let contract: DataContract =
            DataContractWasm::try_from_options(&options, "dataContract")?.into();
        let document = DocumentWasm::try_from_options(&options, "document")?;
        let property: String = try_from_options_with(&options, "property", |value| {
            try_to_string(value, "property")
        })?;
        let recipient_private_key = secret_key_from_options(&options, "recipientPrivateKey")?;
        let sender_key: IdentityPublicKey =
            IdentityPublicKeyWasm::try_from_options(&options, "senderKey")?.into();

        let document_type_name = document_type_name_of(&contract, &document)?;
        let document_type = contract
            .document_type_for_name(&document_type_name)
            .map_err(|e| WasmSdkError::invalid_argument(e.to_string()))?;
        let document = sanitized(document_type, document.into());
        Ok(decrypt_property(
            document_type,
            &property,
            document.properties(),
            &recipient_private_key,
            &secp256k1_public_key(&sender_key, "senderKey")?,
        )?)
    }

    /// Reads whose keys an encrypted property of a document is under: the recipient and the
    /// sender identities and the ids of their keys, which a reader fetches to decrypt it. A
    /// document whose owner may have changed since it was written is refused: its sender key
    /// id may name a previous owner's key.
    #[wasm_bindgen(js_name = "encryptedPropertyEnvelope")]
    pub fn encrypted_property_envelope(
        options: EncryptedPropertyEnvelopeOptionsJs,
    ) -> Result<EncryptedPropertyEnvelopeWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let contract: DataContract =
            DataContractWasm::try_from_options(&options, "dataContract")?.into();
        let document = DocumentWasm::try_from_options(&options, "document")?;
        let property: String = try_from_options_with(&options, "property", |value| {
            try_to_string(value, "property")
        })?;

        let document_type_name = document_type_name_of(&contract, &document)?;
        let document_type = contract
            .document_type_for_name(&document_type_name)
            .map_err(|e| WasmSdkError::invalid_argument(e.to_string()))?;
        let document = sanitized(document_type, document.into());
        Ok(
            EncryptedPropertyEnvelope::read(document_type, &property, &document)
                .map(EncryptedPropertyEnvelopeWasm)?,
        )
    }
}
