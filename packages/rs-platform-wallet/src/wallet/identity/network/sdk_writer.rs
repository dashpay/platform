//! Object-safe seam over the SDK's DashPay write/broadcast surface.
//!
//! The fetch half of the DashPay network layer is already testable
//! through the dash-sdk built-in mock (`SdkBuilder::new_mock` +
//! `expect_fetch`/`expect_fetch_many`, as the `identity_sync.rs` tests
//! demonstrate). The *write* half is not: the two operations
//! `IdentityWallet` performs over the SDK —
//! [`Sdk::send_contact_request`](dash_sdk::Sdk::send_contact_request)
//! and a document put — cannot be reached through the mock and cannot
//! be wrapped behind a `dyn` trait *as the SDK exposes them*.
//! `send_contact_request` is generic over **seven** type parameters
//! (the signer plus three ECDH/xpub closure pairs), so it is not
//! object-safe; the document put rides on the signer-generic
//! [`PutDocument`](dash_sdk::platform::transition::put_document::PutDocument)
//! trait.
//!
//! This module defines ONE object-safe trait,
//! [`DashPaySdkWriter`], exposing exactly those two concrete
//! operations. `IdentityWallet` keeps doing all of the derivation
//! (key-index resolution, ECDH-key derivation, xpub derivation,
//! avatar hashing, document construction); the seam receives the
//! already-derived primitives plus a borrowed `&dyn
//! Signer<IdentityPublicKey>` and performs the final SDK call.
//! Production wallets hold the default [`SdkWriter`] (an `Arc<Sdk>`
//! wrapper); tests substitute a recording / stubbing implementation
//! to assert the broadcast inputs without a live network.
//!
//! Keeping the trait to two methods is deliberate: this is a test
//! seam, not a refactor. Everything the SDK call needs that
//! `IdentityWallet` can compute up front travels in by value so the
//! trait stays `dyn`-compatible.

use std::sync::Arc;

use async_trait::async_trait;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::Document;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};

use dash_sdk::platform::dashpay::{
    ContactRequestInput, EcdhProvider, RecipientIdentity, SendContactRequestInput,
    SendContactRequestResult,
};

use crate::error::PlatformWalletError;

// Borrowed-signer adapter — same pattern used by `contact_requests.rs`
// / `profile.rs`. Lets a `&dyn Signer<IdentityPublicKey>` satisfy the
// owned, `Sized` `S: Signer<IdentityPublicKey>` bound the SDK input
// types require, so the trait method below can stay object-safe while
// still threading the host's signer to the SDK.
struct SignerRef<'a, S: ?Sized>(&'a S);

impl<'a, S: ?Sized> std::fmt::Debug for SignerRef<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerRef")
    }
}

#[async_trait]
impl<'a, K, S> Signer<K> for SignerRef<'a, S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
        self.0.sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
        self.0.sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

/// Pre-derived inputs for a single contact-request broadcast.
///
/// Every field is resolved by `IdentityWallet` before the seam is
/// called: key indices come from the sender / recipient identities,
/// `ecdh_private_key` is derived from the wallet seed, `xpub_bytes` is
/// the DashPay receiving-account xpub to share, and
/// `signing_public_key` is the HIGH/CRITICAL authentication key the
/// document state transition is signed with. The seam only assembles
/// the SDK `EcdhProvider` + xpub closure and dispatches.
pub(crate) struct SendContactRequestParams<'a> {
    /// Sender (owner) identity — already loaded from local state.
    pub sender_identity: Identity,
    /// Recipient identity — already fetched from Platform.
    pub recipient_identity: Identity,
    /// Sender encryption-key id used for ECDH.
    pub sender_key_index: u32,
    /// Recipient decryption-key id used for ECDH.
    pub recipient_key_index: u32,
    /// DashPay account reference (currently `0`).
    pub account_reference: u32,
    /// Optional unencrypted account label (SDK encrypts it).
    pub account_label: Option<String>,
    /// Optional unencrypted auto-accept proof.
    pub auto_accept_proof: Option<Vec<u8>>,
    /// Sender ECDH private key derived from the wallet seed.
    pub ecdh_private_key: dashcore::secp256k1::SecretKey,
    /// DashPay receiving-account xpub to share with the recipient, in the
    /// **69-byte DIP-15 compact form** (`parentFingerprint ‖ chainCode ‖
    /// pubKey`) — NOT `ExtendedPubKey::encode()`. The SDK validates len == 69
    /// before encrypting.
    pub xpub_bytes: Vec<u8>,
    /// HIGH/CRITICAL authentication key the transition is signed with.
    pub signing_public_key: IdentityPublicKey,
    /// Borrowed host signer for the document state transition.
    pub signer: &'a (dyn Signer<IdentityPublicKey> + Send + Sync),
}

/// Pre-built inputs for a single DashPay document put.
///
/// `IdentityWallet` builds the [`Document`] (profile create/update) and
/// resolves the signing key + document type; the seam performs the
/// `put_to_platform_and_wait_for_response` broadcast.
pub(crate) struct PutDocumentParams<'a> {
    /// Fully-built document to broadcast.
    pub document: Document,
    /// Owned document type for the target document.
    pub document_type: DocumentType,
    /// HIGH/CRITICAL authentication key the transition is signed with.
    pub signing_public_key: IdentityPublicKey,
    /// Borrowed host signer for the document state transition.
    pub signer: &'a (dyn Signer<IdentityPublicKey> + Send + Sync),
}

/// Object-safe seam over the SDK's DashPay write operations.
///
/// Held as a field on [`IdentityWallet`](super::IdentityWallet),
/// defaulting to the [`SdkWriter`] `Arc<Sdk>` wrapper so public
/// construction paths and the FFI are untouched. Tests inject a
/// stub/recording implementation.
///
/// The returned futures are `Send` (the default `#[async_trait]`
/// boxing): the write paths this seam serves
/// (`send_contact_request_with_external_signer`, profile create/update)
/// are driven through the FFI's `block_on_worker`, which requires
/// `Future: Send`. (The DashPay *read*/sync path, which is `!Send` and
/// runs on a dedicated thread, does not go through this seam.)
#[async_trait]
pub(crate) trait DashPaySdkWriter: std::fmt::Debug + Send + Sync {
    /// Build the ECDH provider + xpub closure from the pre-derived
    /// inputs and broadcast the contact-request document.
    async fn send_contact_request(
        &self,
        params: SendContactRequestParams<'_>,
    ) -> Result<SendContactRequestResult, PlatformWalletError>;

    /// Broadcast a pre-built DashPay document and wait for the
    /// confirmation proof.
    async fn put_document(
        &self,
        params: PutDocumentParams<'_>,
    ) -> Result<Document, PlatformWalletError>;
}

/// Default [`DashPaySdkWriter`] backed by a live [`Sdk`](dash_sdk::Sdk).
///
/// This is the production implementation; it simply forwards to the
/// real SDK. Holding it behind the trait is what lets the network-layer
/// tests substitute a mock writer.
#[derive(Clone)]
pub(crate) struct SdkWriter {
    sdk: Arc<dash_sdk::Sdk>,
}

impl SdkWriter {
    /// Wrap an SDK handle as the default writer.
    pub(crate) fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self { sdk }
    }
}

impl std::fmt::Debug for SdkWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdkWriter").finish()
    }
}

#[async_trait]
impl DashPaySdkWriter for SdkWriter {
    async fn send_contact_request(
        &self,
        params: SendContactRequestParams<'_>,
    ) -> Result<SendContactRequestResult, PlatformWalletError> {
        let SendContactRequestParams {
            sender_identity,
            recipient_identity,
            sender_key_index,
            recipient_key_index,
            account_reference,
            account_label,
            auto_accept_proof,
            ecdh_private_key,
            xpub_bytes,
            signing_public_key,
            signer,
        } = params;

        let contact_request_input = ContactRequestInput {
            sender_identity,
            recipient: RecipientIdentity::Identity(recipient_identity),
            sender_key_index,
            recipient_key_index,
            account_reference,
            account_label,
            auto_accept_proof,
        };

        let send_input = SendContactRequestInput {
            contact_request: contact_request_input,
            identity_public_key: signing_public_key,
            signer: SignerRef(signer),
        };

        // SDK-side ECDH: hand back the pre-derived sender private key,
        // guarding that the SDK asks for the encryption key we resolved.
        let expected_key_id = sender_key_index;
        let ecdh_provider: EcdhProvider<
            _,
            _,
            fn(
                &dashcore::secp256k1::PublicKey,
            ) -> std::future::Ready<Result<[u8; 32], dash_sdk::Error>>,
            _,
        > = EcdhProvider::SdkSide {
            get_private_key: move |key: &IdentityPublicKey, _index: u32| {
                use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
                let pk = ecdh_private_key;
                let actual_key_id = key.id();
                async move {
                    if actual_key_id != expected_key_id {
                        return Err(dash_sdk::Error::Generic(format!(
                            "ECDH key mismatch: expected key {}, got {}",
                            expected_key_id, actual_key_id
                        )));
                    }
                    Ok(pk)
                }
            },
        };

        let xpub_bytes_clone = xpub_bytes.clone();
        self.sdk
            .send_contact_request(send_input, ecdh_provider, |_account_ref: u32| async move {
                Ok::<Vec<u8>, dash_sdk::Error>(xpub_bytes_clone)
            })
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to send contact request: {e}"
                ))
            })
    }

    async fn put_document(
        &self,
        params: PutDocumentParams<'_>,
    ) -> Result<Document, PlatformWalletError> {
        use dash_sdk::platform::transition::put_document::PutDocument;

        let PutDocumentParams {
            document,
            document_type,
            signing_public_key,
            signer,
        } = params;

        document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                document_type,
                None,
                signing_public_key,
                None,
                &SignerRef(signer),
                None,
            )
            .await
            .map_err(PlatformWalletError::Sdk)
    }
}
