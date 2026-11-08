//! Private documents and DashPay private contact requests (protocol version 15):
//! `WasmSdk.getPrivateDocumentEntries`, `WasmSdk.sealPrivateContactRequest` and
//! `WasmSdk.scanPrivateContactRequests`, thin bindings over `dash_sdk::platform`'s
//! `private_document_entries`, `private_document_scan` and `dashpay` modules.
//!
//! Appending a sealed entry is paid from the shielded pool, which needs an Orchard proof this
//! build cannot make: a sealed entry is appended by a wallet that can, or broadcast in a
//! transition built elsewhere.

use crate::encrypted_for::secret_key_from_options;
use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::identity::{Identity, KeyID};
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::secp256k1::SecretKey;
use dash_sdk::platform::dashpay::{
    private_contact_request_outgoing_viewing_key, PrivateContactRequest,
    PrivateContactRequestRejection, PrivateContactRequestScan,
};
use dash_sdk::platform::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntriesQuery,
};
use dash_sdk::platform::private_document_scan::PrivateDocumentScanKeys;
use dash_sdk::platform::{Fetch, Identifier};
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::identity::IdentityWasm;
use wasm_dpp2::serialization::to_object;
use wasm_dpp2::utils::{
    try_from_options_optional_with, try_from_options_with, try_to_array, try_to_bytes,
    try_to_fixed_bytes, try_to_string, try_to_u32, try_to_u64,
};
use wasm_dpp2::IdentitySignerWasm;

#[wasm_bindgen(typescript_custom_section)]
const PRIVATE_CONTACT_REQUESTS_TS: &'static str = r#"
/** A page of the entries of a private document type's store. */
export interface PrivateDocumentEntriesPage {
  /** The entries, from the page's first position, in append order. */
  entries: PrivateDocumentEntryInfo[];
  /** How many entries the store holds. */
  totalCount: bigint;
}

/** One entry of a private document type's store. */
export interface PrivateDocumentEntryInfo {
  position: bigint;
  /** The height of the block that appended it. */
  blockHeight: bigint;
  /** The time of the block that appended it, in milliseconds. */
  blockTimeMs: bigint;
  /** The encrypted entry, of the type's `entrySize`. */
  data: Uint8Array;
}

/** What a DashPay private contact request carries, in the clear inside its entry. */
export interface PrivateContactRequestFields {
  /** The DIP-15 account reference of the sender's account the recipient pays to. */
  accountReference: number;
  /** The 69-byte DIP-15 compact extended public key the recipient pays to. */
  extendedPublicKey: Uint8Array;
  /** At most 63 UTF-8 bytes. */
  accountLabel?: string;
  /** A DIP-15 auto-accept proof, 38 to 102 bytes. */
  autoAcceptProof?: Uint8Array;
  /** The 43-byte Orchard address the recipient pays this contact to privately. */
  shieldedAddress?: Uint8Array;
}

/** Options for `WasmSdk.sealPrivateContactRequest`. */
export interface SealPrivateContactRequestOptions {
  /** The sending identity. */
  senderIdentity: Identity;
  /**
   * The id of its enabled ECDSA_SECP256K1 ENCRYPTION key, which the entry names: the pair's
   * shared key and the sender's outgoing viewing key are derived from it.
   */
  senderEncryptionKeyId: number;
  /** That key's private key. */
  senderEncryptionPrivateKey: PrivateKey;
  /**
   * The signer the sender signs its transitions with. It signs the request with the first of
   * the sender's enabled AUTHENTICATION keys at HIGH or CRITICAL that it holds.
   */
  signer: IdentitySigner;
  /** The recipient identity. */
  recipientIdentity: Identity;
  /** The id of its enabled ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key to seal to. */
  recipientKeyId: number;
  request: PrivateContactRequestFields;
}

/** An identity of the wallet and the keys entries to it may be sealed to. */
export interface PrivateContactRequestScanRecipient {
  identityId: Identifier | Uint8Array | string;
  /** Its DECRYPTION and ENCRYPTION keys, tried in this order. */
  keys: { keyId: number; privateKey: PrivateKey }[];
}

/** An identity of the wallet and the encryption keys it sealed requests with. */
export interface PrivateContactRequestScanSender {
  identityId: Identifier | Uint8Array | string;
  /** Each DashPay encryption key it has used, disabled ones included. */
  encryptionKeys: { keyId: number; privateKey: PrivateKey }[];
}

/**
 * Options for `WasmSdk.scanPrivateContactRequests`. The scan fetches the identities the page's
 * requests are checked against right after the page, so a node serving both can tell which
 * identities the wallet looked up.
 */
export interface ScanPrivateContactRequestsOptions {
  /** The position to read from: 0, then the `nextPosition` of the previous scan. */
  startPosition: bigint | number;
  recipients?: PrivateContactRequestScanRecipient[];
  senders?: PrivateContactRequestScanSender[];
}

/** A request to one of the wallet's identities, its sender's signature checked. */
export interface ReceivedPrivateContactRequest {
  position: bigint;
  blockHeight: bigint;
  blockTimeMs: bigint;
  recipientId: Uint8Array;
  recipientKeyId: number;
  senderId: Uint8Array;
  /** The sender's encryption key it names. */
  senderKeyId: number;
  /** A copy of the entry appended again has the same commitment. */
  commitment: Uint8Array;
  request: PrivateContactRequestFields;
}

/** A request one of the wallet's identities sent, checked to be its own. */
export interface SentPrivateContactRequest {
  position: bigint;
  blockHeight: bigint;
  blockTimeMs: bigint;
  senderId: Uint8Array;
  /** The sender's encryption key it names. */
  senderKeyId: number;
  recipientId: Uint8Array;
  /** The compressed recipient key it is sealed to. */
  recipientPublicKey: Uint8Array;
  commitment: Uint8Array;
  request: PrivateContactRequestFields;
}

/** An entry that opened for the wallet but is not taken as a request. */
export interface RejectedPrivateContactRequest {
  position: bigint;
  blockHeight: bigint;
  /**
   * `envelope`, `anonymousSender`, `notSigned`, `senderNotFound`, `senderKeyNotUsable`,
   * `senderNotAuthenticated`, `recipientKeyNotUsable`, `invalidRequest` or `duplicate`.
   */
  reason: string;
  detail?: string;
}

/** What one page of DashPay's `privateContactRequest` store holds for the wallet. */
export interface PrivateContactRequestScanResult {
  received: ReceivedPrivateContactRequest[];
  sent: SentPrivateContactRequest[];
  rejected: RejectedPrivateContactRequest[];
  /** The position to scan from next, once what the page held is stored. */
  nextPosition: bigint;
  totalCount: bigint;
  /** Whether the scan reached the store's last entry. */
  isComplete: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "SealPrivateContactRequestOptions")]
    pub type SealPrivateContactRequestOptionsJs;

    #[wasm_bindgen(typescript_type = "ScanPrivateContactRequestsOptions")]
    pub type ScanPrivateContactRequestsOptionsJs;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryInfo {
    position: u64,
    block_height: u64,
    block_time_ms: u64,
    data: BinaryData,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntriesPage {
    entries: Vec<EntryInfo>,
    total_count: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestFields {
    account_reference: u32,
    extended_public_key: BinaryData,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_accept_proof: Option<BinaryData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shielded_address: Option<BinaryData>,
}

impl From<PrivateContactRequest> for RequestFields {
    fn from(request: PrivateContactRequest) -> Self {
        Self {
            account_reference: request.account_reference,
            extended_public_key: BinaryData::new(request.extended_public_key.to_vec()),
            account_label: request.account_label,
            auto_accept_proof: request.auto_accept_proof.map(BinaryData::new),
            shielded_address: request
                .shielded_address
                .map(|address| BinaryData::new(address.to_vec())),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Received {
    position: u64,
    block_height: u64,
    block_time_ms: u64,
    recipient_id: Identifier,
    recipient_key_id: KeyID,
    sender_id: Identifier,
    sender_key_id: KeyID,
    commitment: BinaryData,
    request: RequestFields,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Sent {
    position: u64,
    block_height: u64,
    block_time_ms: u64,
    sender_id: Identifier,
    sender_key_id: KeyID,
    recipient_id: Identifier,
    recipient_public_key: BinaryData,
    commitment: BinaryData,
    request: RequestFields,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Rejected {
    position: u64,
    block_height: u64,
    reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResult {
    received: Vec<Received>,
    sent: Vec<Sent>,
    rejected: Vec<Rejected>,
    next_position: u64,
    total_count: u64,
    is_complete: bool,
}

impl From<PrivateContactRequestScan> for ScanResult {
    fn from(scan: PrivateContactRequestScan) -> Self {
        let is_complete = scan.is_complete();
        Self {
            received: scan
                .received
                .into_iter()
                .map(|received| Received {
                    position: received.position,
                    block_height: received.block_height,
                    block_time_ms: received.block_time_ms,
                    recipient_id: received.recipient_id,
                    recipient_key_id: received.recipient_key_id,
                    sender_id: received.sender_id,
                    sender_key_id: received.sender_key_id,
                    commitment: BinaryData::new(received.commitment.to_vec()),
                    request: received.request.into(),
                })
                .collect(),
            sent: scan
                .sent
                .into_iter()
                .map(|sent| Sent {
                    position: sent.position,
                    block_height: sent.block_height,
                    block_time_ms: sent.block_time_ms,
                    sender_id: sent.sender_id,
                    sender_key_id: sent.sender_key_id,
                    recipient_id: sent.recipient_id,
                    recipient_public_key: BinaryData::new(
                        sent.recipient_public_key.serialize().to_vec(),
                    ),
                    commitment: BinaryData::new(sent.commitment.to_vec()),
                    request: sent.request.into(),
                })
                .collect(),
            rejected: scan
                .rejected
                .into_iter()
                .map(|rejected| {
                    let (reason, detail) = rejection(rejected.reason);
                    Rejected {
                        position: rejected.position,
                        block_height: rejected.block_height,
                        reason,
                        detail,
                    }
                })
                .collect(),
            next_position: scan.next_position,
            total_count: scan.total_count,
            is_complete,
        }
    }
}

fn rejection(reason: PrivateContactRequestRejection) -> (&'static str, Option<String>) {
    match reason {
        PrivateContactRequestRejection::Envelope(error) => ("envelope", Some(error.to_string())),
        PrivateContactRequestRejection::AnonymousSender => ("anonymousSender", None),
        PrivateContactRequestRejection::NotSigned => ("notSigned", None),
        PrivateContactRequestRejection::SenderNotFound => ("senderNotFound", None),
        PrivateContactRequestRejection::SenderKeyNotUsable(detail) => {
            ("senderKeyNotUsable", Some(detail))
        }
        PrivateContactRequestRejection::SenderNotAuthenticated => ("senderNotAuthenticated", None),
        PrivateContactRequestRejection::RecipientKeyNotUsable(detail) => {
            ("recipientKeyNotUsable", Some(detail))
        }
        PrivateContactRequestRejection::InvalidRequest(detail) => ("invalidRequest", Some(detail)),
        PrivateContactRequestRejection::Duplicate => ("duplicate", None),
    }
}

/// Secret keys read from the options, wiped when dropped, on every exit: `SecretKey` is `Copy`
/// and erases nothing itself.
#[derive(Default)]
struct ReadSecretKeys(Vec<(KeyID, SecretKey)>);

impl Drop for ReadSecretKeys {
    fn drop(&mut self) {
        for (_, secret_key) in &mut self.0 {
            secret_key.non_secure_erase();
        }
    }
}

fn request_from_options(options: &JsValue) -> Result<PrivateContactRequest, WasmSdkError> {
    let request = try_from_options_with(options, "request", |value| Ok(value.clone()))?;
    Ok(PrivateContactRequest {
        account_reference: try_from_options_with(&request, "accountReference", |value| {
            try_to_u32(value, "accountReference")
        })?,
        extended_public_key: try_from_options_with(&request, "extendedPublicKey", |value| {
            Ok(value.clone())
        })
        .map_err(WasmSdkError::from)
        .and_then(|value| Ok(try_to_fixed_bytes(value, "extendedPublicKey")?))?,
        account_label: try_from_options_optional_with(&request, "accountLabel", |value| {
            try_to_string(value, "accountLabel")
        })?,
        auto_accept_proof: try_from_options_optional_with(&request, "autoAcceptProof", |value| {
            try_to_bytes(value.clone(), "autoAcceptProof")
        })?,
        shielded_address: try_from_options_optional_with(&request, "shieldedAddress", |value| {
            Ok(value.clone())
        })?
        .map(|value| try_to_fixed_bytes(value, "shieldedAddress"))
        .transpose()?,
    })
}

fn scan_keys_from_options(options: &JsValue) -> Result<PrivateDocumentScanKeys, WasmSdkError> {
    let mut keys = PrivateDocumentScanKeys::new();
    let recipients = try_from_options_optional_with(options, "recipients", |value| {
        try_to_array(value, "recipients")
    })?;
    for recipient in recipients.iter().flat_map(|array| array.iter()) {
        let identity_id: Identifier =
            IdentifierWasm::try_from_options(&recipient, "identityId")?.into();
        let recipient_keys = try_from_options_with(&recipient, "keys", |value| {
            try_to_array(value, "recipients[].keys")
        })?;
        let mut read = ReadSecretKeys::default();
        for key in recipient_keys.iter() {
            let key_id = try_from_options_with(&key, "keyId", |value| {
                try_to_u32(value, "recipients[].keys[].keyId")
            })?;
            read.0
                .push((key_id, secret_key_from_options(&key, "privateKey")?));
        }
        keys = keys.with_recipient(identity_id, &read.0);
    }
    let senders =
        try_from_options_optional_with(options, "senders", |value| try_to_array(value, "senders"))?;
    for sender in senders.iter().flat_map(|array| array.iter()) {
        let identity_id: Identifier =
            IdentifierWasm::try_from_options(&sender, "identityId")?.into();
        let encryption_keys = try_from_options_with(&sender, "encryptionKeys", |value| {
            try_to_array(value, "senders[].encryptionKeys")
        })?;
        for key in encryption_keys.iter() {
            let key_id = try_from_options_with(&key, "keyId", |value| {
                try_to_u32(value, "senders[].encryptionKeys[].keyId")
            })?;
            let read = ReadSecretKeys(vec![(key_id, secret_key_from_options(&key, "privateKey")?)]);
            keys = keys.with_sender(
                identity_id,
                key_id,
                private_contact_request_outgoing_viewing_key(&read.0[0].1),
            );
        }
    }
    Ok(keys)
}

#[wasm_bindgen]
impl WasmSdk {
    /// Fetches the proved page of the store of the private document type
    /// `documentTypeName` of `dataContractId` from `startPosition`: up to the end of the
    /// store's chunk holding that position.
    #[wasm_bindgen(
        js_name = "getPrivateDocumentEntries",
        unchecked_return_type = "PrivateDocumentEntriesPage"
    )]
    pub async fn get_private_document_entries(
        &self,
        #[wasm_bindgen(js_name = "dataContractId")] data_contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
        #[wasm_bindgen(js_name = "startPosition")] start_position: u64,
    ) -> Result<JsValue, WasmSdkError> {
        let data_contract_id: Identifier = IdentifierWasm::try_from(data_contract_id)?.into();
        // The page's proof is checked against the contract, which the context provider holds
        // only once loaded.
        self.get_or_fetch_contract(data_contract_id).await?;
        let page = PrivateDocumentEntries::fetch(
            self.as_ref(),
            PrivateDocumentEntriesQuery::new(data_contract_id, document_type_name, start_position),
        )
        .await?
        .ok_or_else(|| WasmSdkError::not_found("no page of the private document type"))?;
        Ok(to_object(&EntriesPage {
            entries: page
                .entries
                .into_iter()
                .map(|entry| EntryInfo {
                    position: entry.position,
                    block_height: entry.block_height,
                    block_time_ms: entry.block_time_ms,
                    data: BinaryData::new(entry.data),
                })
                .collect(),
            total_count: page.total_count,
        })?)
    }

    /// Seals a DashPay private contact request: an entry of DashPay's `privateContactRequest`
    /// store only the recipient opens. It names the sender's encryption key and is signed by
    /// `signer` with one of the sender's signing keys, as its transitions are. The entry still
    /// has to be appended, paid from the shielded pool.
    ///
    /// @returns The entry, of the type's entry size.
    #[wasm_bindgen(js_name = "sealPrivateContactRequest")]
    pub async fn seal_private_contact_request(
        &self,
        options: SealPrivateContactRequestOptionsJs,
    ) -> Result<Vec<u8>, WasmSdkError> {
        let options: JsValue = options.into();
        let sender_identity: Identity =
            IdentityWasm::try_from_options(&options, "senderIdentity")?.into();
        let encryption_key_id =
            try_from_options_with(&options, "senderEncryptionKeyId", |value| {
                try_to_u32(value, "senderEncryptionKeyId")
            })?;
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;
        let recipient_identity: Identity =
            IdentityWasm::try_from_options(&options, "recipientIdentity")?.into();
        let recipient_key_id = try_from_options_with(&options, "recipientKeyId", |value| {
            try_to_u32(value, "recipientKeyId")
        })?;
        let request = request_from_options(&options)?;
        // Read last, and wiped on every exit.
        let encryption_key = ReadSecretKeys(vec![(
            encryption_key_id,
            secret_key_from_options(&options, "senderEncryptionPrivateKey")?,
        )]);

        let sealed = self
            .as_ref()
            .seal_private_contact_request(
                &sender_identity,
                encryption_key_id,
                &encryption_key.0[0].1,
                &signer,
                &recipient_identity,
                recipient_key_id,
                &request,
            )
            .await?;
        Ok(sealed.to_vec())
    }

    /// Reads the page of DashPay's `privateContactRequest` store from `startPosition` and
    /// returns what it holds for the wallet: the requests to its identities, each checked
    /// against its sender's signing keys and the recipient's key, and the requests its
    /// identities sent, recovered with their encryption keys and checked to be theirs.
    ///
    /// It fetches the identities the page's requests are checked against right after the page:
    /// the senders, and the wallet's own identities. A node serving both can tell which
    /// identities the wallet looked up, and so that someone it knows sent the wallet a request.
    #[wasm_bindgen(
        js_name = "scanPrivateContactRequests",
        unchecked_return_type = "PrivateContactRequestScanResult"
    )]
    pub async fn scan_private_contact_requests(
        &self,
        options: ScanPrivateContactRequestsOptionsJs,
    ) -> Result<JsValue, WasmSdkError> {
        let options: JsValue = options.into();
        let start_position = try_from_options_with(&options, "startPosition", |value| {
            try_to_u64(value, "startPosition")
        })?;
        let keys = scan_keys_from_options(&options)?;
        let scan = self
            .as_ref()
            .scan_private_contact_requests(start_position, &keys)
            .await?;
        Ok(to_object(&ScanResult::from(scan))?)
    }
}
