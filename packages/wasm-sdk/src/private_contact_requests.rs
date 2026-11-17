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
use dash_sdk::dpp::system_data_contracts::SystemDataContract;
use dash_sdk::platform::dashpay::{
    private_contact_request_outgoing_viewing_key, PrivateContactRequest,
    PrivateContactRequestRejection, PrivateContactRequestScan,
    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
};
use dash_sdk::platform::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntriesQuery,
};
use dash_sdk::platform::private_document_scan::PrivateDocumentScanKeys;
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use serde::Serialize;
use std::sync::Arc;
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

/// The identities and keys a scan's options name: each recipient with the keys entries to it
/// may be sealed to, each sender with the encryption keys it sealed with. The secret keys are
/// wiped when it is dropped.
#[derive(Default)]
struct ScanKeyOptions {
    recipients: Vec<(Identifier, ReadSecretKeys)>,
    senders: Vec<(Identifier, ReadSecretKeys)>,
}

impl ScanKeyOptions {
    /// The scan keys these give: a sender's outgoing viewing key is derived from each of its
    /// encryption keys.
    fn scan_keys(&self) -> PrivateDocumentScanKeys {
        let mut keys = PrivateDocumentScanKeys::new();
        for (identity_id, read) in &self.recipients {
            keys = keys.with_recipient(*identity_id, &read.0);
        }
        for (identity_id, read) in &self.senders {
            for (key_id, secret_key) in &read.0 {
                keys = keys.with_sender(
                    *identity_id,
                    *key_id,
                    private_contact_request_outgoing_viewing_key(secret_key),
                );
            }
        }
        keys
    }
}

/// The keys of `entries`, an array of `{ keyId, privateKey }`, named `field` in errors. The
/// vector is sized for them first: growing it would move secret keys and leave copies behind.
fn secret_keys_from_array(
    entries: &js_sys::Array,
    field: &str,
) -> Result<ReadSecretKeys, WasmSdkError> {
    let mut read = ReadSecretKeys(Vec::with_capacity(entries.length() as usize));
    for key in entries.iter() {
        let key_id = try_from_options_with(&key, "keyId", |value| {
            try_to_u32(value, &format!("{field}[].keyId"))
        })?;
        read.0
            .push((key_id, secret_key_from_options(&key, "privateKey")?));
    }
    Ok(read)
}

fn scan_key_options(options: &JsValue) -> Result<ScanKeyOptions, WasmSdkError> {
    let mut read = ScanKeyOptions::default();
    let recipients = try_from_options_optional_with(options, "recipients", |value| {
        try_to_array(value, "recipients")
    })?;
    for recipient in recipients.iter().flat_map(|array| array.iter()) {
        let identity_id: Identifier =
            IdentifierWasm::try_from_options(&recipient, "identityId")?.into();
        let keys = try_from_options_with(&recipient, "keys", |value| {
            try_to_array(value, "recipients[].keys")
        })?;
        read.recipients.push((
            identity_id,
            secret_keys_from_array(&keys, "recipients[].keys")?,
        ));
    }
    let senders =
        try_from_options_optional_with(options, "senders", |value| try_to_array(value, "senders"))?;
    for sender in senders.iter().flat_map(|array| array.iter()) {
        let identity_id: Identifier =
            IdentifierWasm::try_from_options(&sender, "identityId")?.into();
        let keys = try_from_options_with(&sender, "encryptionKeys", |value| {
            try_to_array(value, "senders[].encryptionKeys")
        })?;
        read.senders.push((
            identity_id,
            secret_keys_from_array(&keys, "senders[].encryptionKeys")?,
        ));
    }
    Ok(read)
}

fn scan_keys_from_options(options: &JsValue) -> Result<PrivateDocumentScanKeys, WasmSdkError> {
    Ok(scan_key_options(options)?.scan_keys())
}

impl WasmSdk {
    /// The contract `data_contract_id` with its private type `document_type_name`, as the
    /// SDK's context provider must hold it to verify the type's pages
    /// ([`dash_sdk::Sdk::fetch_private_document_contract`]): one retained from before the type
    /// was declared is replaced by the current one, which is also stored for the next page
    /// load, as the contract cache stores every contract it fetches. Stored once a version
    /// ([`WasmSdk::mark_contract_version_persisted`]): a store walk reads the contract for
    /// every page, and a page whose contract is unchanged neither copies nor stores it.
    pub(crate) async fn private_document_contract(
        &self,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Arc<DataContract>, WasmSdkError> {
        let contract = self
            .as_ref()
            .fetch_private_document_contract(data_contract_id, document_type_name)
            .await?;
        if self.mark_contract_version_persisted(&contract) {
            self.cache_contract((*contract).clone());
        }
        Ok(contract)
    }

    /// DashPay, with its `privateContactRequest` type ([`Self::private_document_contract`]).
    async fn private_contact_request_contract(&self) -> Result<Arc<DataContract>, WasmSdkError> {
        self.private_document_contract(
            SystemDataContract::Dashpay.id(),
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        )
        .await
    }

    /// The proved page of the store of the private type `document_type_name` of
    /// `data_contract_id` from `start_position`, verified against the contract with the type.
    pub(crate) async fn private_document_page(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        start_position: u64,
    ) -> Result<PrivateDocumentEntries, WasmSdkError> {
        self.private_document_contract(data_contract_id, &document_type_name)
            .await?;
        PrivateDocumentEntries::fetch(
            self.as_ref(),
            PrivateDocumentEntriesQuery::new(data_contract_id, document_type_name, start_position),
        )
        .await?
        .ok_or_else(|| WasmSdkError::not_found("no page of the private document type"))
    }
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
        let page = self
            .private_document_page(data_contract_id, document_type_name, start_position)
            .await?;
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

        self.private_contact_request_contract().await?;
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
        self.private_contact_request_contract().await?;
        let scan = self
            .as_ref()
            .scan_private_contact_requests(start_position, &keys)
            .await?;
        Ok(to_object(&ScanResult::from(scan))?)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::context_provider::WasmTrustedContext;
    use crate::contract_store::{self, Slot};
    use dash_sdk::dpp::dashcore::Network;
    use dash_sdk::dpp::data_contract::accessors::v0::{
        DataContractV0Getters, DataContractV0Setters,
    };
    use dash_sdk::dpp::system_data_contracts::load_system_data_contract;
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::platform::private_document_entries::PrivateDocumentEntry;
    use dash_sdk::{Sdk, SdkBuilder};

    /// DashPay as `platform_version` bundles it, under an id no system contract has, so the
    /// trusted context does not resolve it itself.
    fn dashpay_at(platform_version: &PlatformVersion) -> DataContract {
        let mut contract = load_system_data_contract(SystemDataContract::Dashpay, platform_version)
            .expect("DashPay loads");
        contract.set_id(Identifier::new([0x5D; 32]));
        contract
    }

    /// The version of `contract_id` stored for the next page load on testnet, if any.
    fn stored_version(contract_id: Identifier) -> Option<u32> {
        contract_store::load_all(Slot::trusted(Network::Testnet), PlatformVersion::latest())
            .into_iter()
            .find(|contract| contract.id() == contract_id)
            .map(|contract| contract.version())
    }

    /// A store walk reads the contract for every page, and stores it once a version: the
    /// first page and the one after an upgrade store it, and unchanged pages neither copy
    /// nor store it, which a stored entry removed behind the SDK's back shows by staying
    /// absent. A removal through the SDK stores the same version again on the next page.
    #[tokio::test]
    async fn should_store_a_private_type_contract_once_a_version() {
        let current = dashpay_at(PlatformVersion::latest());
        let contract_id = current.id();
        let slot = Slot::trusted(Network::Testnet);
        let context = WasmTrustedContext::for_testing(vec![]);
        let mut inner: Sdk = SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock SDK");
        inner
            .set_context_provider(context.clone())
            .expect("the context provider is set");
        inner
            .mock()
            .expect_fetch(contract_id, Some(current.clone()))
            .await
            .expect("the contract fetch is expected");
        let sdk = WasmSdk::new_for_testing(inner, Some(context.clone()));
        let read = || {
            sdk.private_document_contract(contract_id, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME)
        };

        read().await.expect("the first page's contract");
        assert_eq!(stored_version(contract_id), Some(current.version()));

        contract_store::remove(slot, &contract_id);
        read().await.expect("an unchanged page's contract");
        read().await.expect("another unchanged page's contract");
        assert_eq!(
            stored_version(contract_id),
            None,
            "unchanged pages store nothing"
        );

        assert!(sdk.remove_cached_contract(&contract_id));
        read().await.expect("the same version, fetched again");
        assert_eq!(
            stored_version(contract_id),
            Some(current.version()),
            "stored again once removed through the SDK"
        );

        let mut upgraded = current.clone();
        upgraded.set_version(current.version() + 1);
        context.add_known_contract(upgraded.clone());
        read().await.expect("the upgraded contract");
        assert_eq!(stored_version(contract_id), Some(upgraded.version()));

        contract_store::remove(slot, &contract_id);
        read().await.expect("an unchanged page's contract");
        assert_eq!(stored_version(contract_id), None);
    }

    /// A context provider that retains a contract from before its private type was declared,
    /// as one restored from a page's storage may: the page is read once the current contract
    /// is fetched and given to it, and the context then holds the current one.
    #[tokio::test]
    async fn should_read_a_private_page_once_a_retained_contract_without_the_type_is_replaced() {
        let stale = dashpay_at(PlatformVersion::get(14).expect("protocol version 14"));
        let current = dashpay_at(PlatformVersion::latest());
        let contract_id = current.id();
        assert!(stale
            .document_type_for_name(PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME)
            .is_err());
        let context = WasmTrustedContext::for_testing(vec![]);
        context.add_known_contract(stale);

        let mut inner: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock SDK");
        inner
            .set_context_provider(context.clone())
            .expect("the context provider is set");
        inner
            .mock()
            .expect_fetch(contract_id, Some(current.clone()))
            .await
            .expect("the contract fetch is expected");
        let page = PrivateDocumentEntries {
            chunk_power: None,
            entries: vec![PrivateDocumentEntry {
                position: 0,
                block_height: 1,
                block_time_ms: 2,
                data: vec![7; 4],
            }],
            total_count: 1,
        };
        inner
            .mock()
            .expect_fetch(
                PrivateDocumentEntriesQuery::new(
                    contract_id,
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                ),
                Some(page.clone()),
            )
            .await
            .expect("the page fetch is expected");
        let sdk = WasmSdk::new_for_testing(inner, Some(context.clone()));

        let read = sdk
            .private_document_page(
                contract_id,
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME.to_string(),
                0,
            )
            .await
            .expect("the page reads");
        assert_eq!(read, page);
        let held = context
            .get_known_contract(&contract_id)
            .expect("the context holds the contract");
        assert!(held
            .document_type_for_name(PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME)
            .is_ok());
        assert_eq!(held.version(), current.version());
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod binding_tests {
    use super::*;
    use dash_sdk::dpp::dashcore::{Network, PrivateKey};
    use dash_sdk::dpp::secp256k1::PublicKey;
    use dash_sdk::platform::dashpay::{
        ReceivedPrivateContactRequest, RejectedPrivateContactRequest, SentPrivateContactRequest,
    };
    use js_sys::{Array, BigInt, Object, Reflect, Uint8Array};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use wasm_dpp2::PrivateKeyWasm;

    /// 2^53 + 1: the first integer a JavaScript number cannot hold.
    const BEYOND_SAFE_INTEGER: u64 = 9_007_199_254_740_993;

    fn object(fields: &[(&str, JsValue)]) -> JsValue {
        let object = Object::new();
        for (key, value) in fields {
            Reflect::set(&object, &JsValue::from_str(key), value).expect("fixture property");
        }
        object.into()
    }

    fn get(value: &JsValue, key: &str) -> JsValue {
        Reflect::get(value, &JsValue::from_str(key)).expect("a property")
    }

    fn bytes(fill: u8, len: usize) -> JsValue {
        Uint8Array::from(vec![fill; len].as_slice()).into()
    }

    fn secret_key(byte: u8) -> SecretKey {
        SecretKey::from_secret_bytes([byte; 32]).expect("a secret key")
    }

    /// `secret_key(byte)` as the `PrivateKey` wrapper JavaScript passes.
    fn private_key(byte: u8) -> JsValue {
        PrivateKeyWasm::from(
            PrivateKey::from_byte_array(&[byte; 32], Network::Testnet).expect("a private key"),
        )
        .into()
    }

    fn key(key_id: u32, byte: u8) -> JsValue {
        object(&[("keyId", key_id.into()), ("privateKey", private_key(byte))])
    }

    fn request_options(fields: &[(&str, JsValue)]) -> JsValue {
        let mut request = vec![
            ("accountReference", JsValue::from(7u32)),
            ("extendedPublicKey", bytes(0x33, 69)),
        ];
        request.extend(fields.iter().cloned());
        object(&[("request", object(&request))])
    }

    fn big(value: &JsValue) -> String {
        assert!(value.is_bigint(), "a bigint: {value:?}");
        String::from(
            value
                .clone()
                .unchecked_into::<BigInt>()
                .to_string(10)
                .expect("decimal"),
        )
    }

    fn byte_array(value: &JsValue) -> Vec<u8> {
        assert!(
            value.is_instance_of::<Uint8Array>(),
            "a Uint8Array: {value:?}"
        );
        value.clone().unchecked_into::<Uint8Array>().to_vec()
    }

    #[wasm_bindgen_test]
    fn should_read_a_request_with_and_without_its_optional_fields() {
        let bare = request_from_options(&request_options(&[])).expect("the required fields");
        assert_eq!(
            bare,
            PrivateContactRequest {
                account_reference: 7,
                extended_public_key: [0x33; 69],
                account_label: None,
                auto_accept_proof: None,
                shielded_address: None,
            }
        );
        let absent = request_from_options(&request_options(&[
            ("accountLabel", JsValue::UNDEFINED),
            ("autoAcceptProof", JsValue::NULL),
            ("shieldedAddress", JsValue::UNDEFINED),
        ]))
        .expect("absent optional fields");
        assert_eq!(absent, bare);

        let full = request_from_options(&request_options(&[
            ("accountLabel", "Savings".into()),
            ("autoAcceptProof", bytes(0x44, 38)),
            ("shieldedAddress", bytes(0x55, 43)),
        ]))
        .expect("every field");
        assert_eq!(full.account_label.as_deref(), Some("Savings"));
        assert_eq!(full.auto_accept_proof, Some(vec![0x44; 38]));
        assert_eq!(full.shielded_address, Some([0x55; 43]));
    }

    #[wasm_bindgen_test]
    fn should_refuse_a_fixed_size_request_field_of_another_size() {
        for (name, len) in [
            ("extendedPublicKey", 68),
            ("extendedPublicKey", 70),
            ("shieldedAddress", 42),
            ("shieldedAddress", 44),
        ] {
            let options = if name == "extendedPublicKey" {
                object(&[(
                    "request",
                    object(&[
                        ("accountReference", 7u32.into()),
                        ("extendedPublicKey", bytes(0x33, len)),
                    ]),
                )])
            } else {
                request_options(&[(name, bytes(0x55, len))])
            };
            assert!(
                request_from_options(&options).is_err(),
                "{name} of {len} bytes"
            );
        }
        assert!(
            request_from_options(&object(&[("request", object(&[]))])).is_err(),
            "the required fields are missing"
        );
    }

    #[wasm_bindgen_test]
    fn should_read_scan_keys_from_the_wrappers_javascript_passes() {
        let alice = Identifier::new([0xA1; 32]);
        let bob = Identifier::new([0xB1; 32]);
        let options = object(&[
            (
                "recipients",
                Array::of2(
                    &object(&[
                        ("identityId", IdentifierWasm::from(alice).into()),
                        ("keys", Array::of2(&key(2, 0x12), &key(3, 0x13)).into()),
                    ]),
                    &object(&[
                        (
                            "identityId",
                            Uint8Array::from(bob.as_bytes().as_slice()).into(),
                        ),
                        ("keys", Array::new().into()),
                    ]),
                )
                .into(),
            ),
            (
                "senders",
                Array::of1(&object(&[
                    ("identityId", IdentifierWasm::from(alice).to_base58().into()),
                    ("encryptionKeys", Array::of1(&key(1, 0x11)).into()),
                ]))
                .into(),
            ),
        ]);
        let read = scan_key_options(&options).expect("the scan keys");
        type ReadKeys = Vec<(KeyID, [u8; 32])>;
        let recipients: Vec<(Identifier, ReadKeys)> = read
            .recipients
            .iter()
            .map(|(identity_id, keys)| {
                (
                    *identity_id,
                    keys.0
                        .iter()
                        .map(|(key_id, secret_key)| (*key_id, secret_key.to_secret_bytes()))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            recipients,
            vec![
                (alice, vec![(2, [0x12; 32]), (3, [0x13; 32])]),
                (bob, vec![]),
            ]
        );
        assert_eq!(read.senders.len(), 1);
        assert_eq!(read.senders[0].0, alice);
        assert_eq!(read.senders[0].1 .0.len(), 1);
        assert_eq!(read.senders[0].1 .0[0].0, 1);
        assert_eq!(read.senders[0].1 .0[0].1.to_secret_bytes(), [0x11; 32]);

        let none = scan_key_options(&object(&[])).expect("no identities");
        assert!(none.recipients.is_empty() && none.senders.is_empty());
    }

    #[wasm_bindgen_test]
    fn should_refuse_malformed_scan_keys() {
        let alice: JsValue = IdentifierWasm::from(Identifier::new([0xA1; 32])).into();
        for (case, recipient) in [
            (
                "an identity id of 31 bytes",
                object(&[
                    ("identityId", bytes(0xA1, 31)),
                    ("keys", Array::of1(&key(2, 0x12)).into()),
                ]),
            ),
            (
                "a key without its id",
                object(&[
                    ("identityId", alice.clone()),
                    (
                        "keys",
                        Array::of1(&object(&[("privateKey", private_key(0x12))])).into(),
                    ),
                ]),
            ),
            (
                "a private key that is not one",
                object(&[
                    ("identityId", alice.clone()),
                    (
                        "keys",
                        Array::of1(&object(&[
                            ("keyId", 2u32.into()),
                            ("privateKey", 12u32.into()),
                        ]))
                        .into(),
                    ),
                ]),
            ),
            (
                "keys that are not an array",
                object(&[("identityId", alice.clone()), ("keys", 2u32.into())]),
            ),
        ] {
            let options = object(&[("recipients", Array::of1(&recipient).into())]);
            assert!(scan_key_options(&options).is_err(), "{case}");
        }
    }

    #[wasm_bindgen_test]
    fn should_hand_a_scan_back_with_exact_bigints_and_byte_arrays() {
        let request = PrivateContactRequest {
            account_reference: 7,
            extended_public_key: [0x33; 69],
            account_label: None,
            auto_accept_proof: Some(vec![0x44; 38]),
            shielded_address: Some([0x55; 43]),
        };
        let recipient_public_key = PublicKey::from_secret_key(&secret_key(2));
        let reasons = [
            (
                PrivateContactRequestRejection::AnonymousSender,
                "anonymousSender",
                None,
            ),
            (PrivateContactRequestRejection::NotSigned, "notSigned", None),
            (
                PrivateContactRequestRejection::SenderNotFound,
                "senderNotFound",
                None,
            ),
            (
                PrivateContactRequestRejection::SenderKeyNotUsable("disabled".into()),
                "senderKeyNotUsable",
                Some("disabled"),
            ),
            (
                PrivateContactRequestRejection::SenderNotAuthenticated,
                "senderNotAuthenticated",
                None,
            ),
            (
                PrivateContactRequestRejection::RecipientKeyNotUsable("another point".into()),
                "recipientKeyNotUsable",
                Some("another point"),
            ),
            (
                PrivateContactRequestRejection::InvalidRequest("too long".into()),
                "invalidRequest",
                Some("too long"),
            ),
            (PrivateContactRequestRejection::Duplicate, "duplicate", None),
        ];
        let scan = PrivateContactRequestScan {
            received: vec![ReceivedPrivateContactRequest {
                position: BEYOND_SAFE_INTEGER,
                block_height: BEYOND_SAFE_INTEGER + 1,
                block_time_ms: u64::MAX,
                recipient_id: Identifier::new([0xB1; 32]),
                recipient_key_id: 2,
                sender_id: Identifier::new([0xA1; 32]),
                sender_key_id: 1,
                commitment: [0xC0; 32],
                request: request.clone(),
            }],
            sent: vec![SentPrivateContactRequest {
                position: BEYOND_SAFE_INTEGER + 2,
                block_height: 3,
                block_time_ms: 4,
                sender_id: Identifier::new([0xA1; 32]),
                sender_key_id: 1,
                recipient_id: Identifier::new([0xB1; 32]),
                recipient_public_key,
                commitment: [0xC1; 32],
                request,
            }],
            rejected: reasons
                .iter()
                .zip(0u64..)
                .map(|((reason, _, _), offset)| RejectedPrivateContactRequest {
                    position: BEYOND_SAFE_INTEGER + 3 + offset,
                    block_height: 5,
                    reason: reason.clone(),
                })
                .collect(),
            next_position: u64::MAX - 1,
            total_count: u64::MAX,
        };

        let result = to_object(&ScanResult::from(scan)).expect("the result converts");
        let received = get(&get(&result, "received"), "0");
        assert_eq!(big(&get(&received, "position")), "9007199254740993");
        assert_eq!(big(&get(&received, "blockHeight")), "9007199254740994");
        assert_eq!(big(&get(&received, "blockTimeMs")), u64::MAX.to_string());
        assert_eq!(byte_array(&get(&received, "recipientId")), vec![0xB1; 32]);
        assert_eq!(byte_array(&get(&received, "senderId")), vec![0xA1; 32]);
        assert_eq!(get(&received, "recipientKeyId").as_f64(), Some(2.0));
        assert_eq!(byte_array(&get(&received, "commitment")), vec![0xC0; 32]);
        let fields = get(&received, "request");
        assert_eq!(get(&fields, "accountReference").as_f64(), Some(7.0));
        assert_eq!(
            byte_array(&get(&fields, "extendedPublicKey")),
            vec![0x33; 69]
        );
        assert!(get(&fields, "accountLabel").is_undefined());
        assert_eq!(byte_array(&get(&fields, "autoAcceptProof")), vec![0x44; 38]);
        assert_eq!(byte_array(&get(&fields, "shieldedAddress")), vec![0x55; 43]);

        let sent = get(&get(&result, "sent"), "0");
        assert_eq!(big(&get(&sent, "position")), "9007199254740995");
        assert_eq!(
            byte_array(&get(&sent, "recipientPublicKey")),
            recipient_public_key.serialize().to_vec()
        );
        assert_eq!(byte_array(&get(&sent, "commitment")), vec![0xC1; 32]);

        let rejected: Array = get(&result, "rejected").unchecked_into();
        assert_eq!(rejected.length() as usize, reasons.len());
        for ((_, reason, detail), (index, entry)) in reasons.iter().zip(rejected.iter().enumerate())
        {
            assert_eq!(
                big(&get(&entry, "position")),
                (BEYOND_SAFE_INTEGER + 3 + index as u64).to_string()
            );
            assert_eq!(get(&entry, "reason").as_string().as_deref(), Some(*reason));
            assert_eq!(
                get(&entry, "detail").as_string().as_deref(),
                *detail,
                "{reason}"
            );
        }

        assert_eq!(
            big(&get(&result, "nextPosition")),
            (u64::MAX - 1).to_string()
        );
        assert_eq!(big(&get(&result, "totalCount")), u64::MAX.to_string());
        assert_eq!(get(&result, "isComplete").as_bool(), Some(false));
    }
}
