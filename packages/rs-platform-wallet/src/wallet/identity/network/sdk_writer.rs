//! Concrete helper over the SDK's DashPay write/broadcast surface.
//!
//! The two operations `IdentityWallet` performs over the SDK —
//! [`Sdk::send_contact_request`](dash_sdk::Sdk::send_contact_request)
//! and a document put — cannot be expressed as plain method calls at
//! the call sites: `send_contact_request` is generic over **seven**
//! type parameters (the signer plus three ECDH/xpub closure pairs) and
//! the document put rides on the signer-generic
//! [`PutDocument`](dash_sdk::platform::transition::put_document::PutDocument)
//! trait. [`SdkWriter`] erases those generics behind two concrete
//! methods that take by-value, already-derived inputs plus a borrowed
//! `&dyn Signer<IdentityPublicKey>`.
//!
//! `IdentityWallet` keeps doing all of the derivation (key-index
//! resolution, ECDH-key derivation, xpub derivation, avatar hashing,
//! document construction); this helper receives the already-derived
//! primitives and performs the final SDK call.

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
// types require, so the helper methods below can take a borrowed signer
// while still threading the host's signer to the SDK.
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
/// Every field is resolved by `IdentityWallet` before the helper is
/// called: key indices come from the sender / recipient identities,
/// `shared_secret` is the ECDH secret already derived against the
/// recipient's encryption key, `xpub_bytes` is the DashPay
/// receiving-account xpub to share, and `signing_public_key` is the
/// HIGH/CRITICAL authentication key the document state transition is
/// signed with. The helper only assembles the SDK `EcdhProvider` + xpub
/// closure and dispatches.
///
/// The ECDH is performed by the caller (client-side), so the helper hands
/// the SDK the finished shared secret via [`EcdhProvider::ClientSide`] —
/// no private key crosses into the SDK. Carrying the precomputed secret
/// (rather than a derivation closure) lets the caller source it from
/// either the resident seed or the Keychain signer without the helper
/// knowing which.
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
    /// ECDH shared secret, already derived by the caller against the
    /// recipient's encryption key (client-side ECDH). The SDK encrypts
    /// the shared xpub with this directly; no private key crosses into
    /// the helper. Held in [`zeroize::Zeroizing`] so it is scrubbed on
    /// drop — it is only dereferenced when handed to the SDK's `ClientSide` closure.
    pub shared_secret: zeroize::Zeroizing<[u8; 32]>,
    /// The recipient encryption public key the `shared_secret` was
    /// derived against. The helper's `ClientSide` closure asserts the SDK
    /// asks for ECDH against this exact key before handing back the
    /// secret — the client-side equivalent of the old SdkSide key-id
    /// guard, so a recipient-key mismatch fails loudly instead of
    /// silently encrypting with the wrong secret.
    pub expected_recipient_pubkey: dashcore::secp256k1::PublicKey,
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
/// resolves the signing key + document type; the helper performs the
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

/// Concrete DashPay write helper backed by a live [`Sdk`](dash_sdk::Sdk).
///
/// Held as a field on [`IdentityWallet`](super::IdentityWallet),
/// forwarding to the real SDK. Its two methods erase the SDK's
/// generic write signatures (`send_contact_request`'s seven type
/// params, the signer-generic `PutDocument`) behind concrete, by-value
/// inputs so the call sites in `contact_requests.rs` / `profile.rs` /
/// `contact_info.rs` stay simple.
///
/// The methods' returned futures are `Send`: the write paths this
/// helper serves (`send_contact_request_with_external_signer`, profile
/// create/update) are driven through the FFI's `block_on_worker`, which
/// requires `Future: Send`. (The DashPay *read*/sync path, which is
/// `!Send` and runs on a dedicated thread, does not go through this
/// helper.)
#[derive(Clone)]
pub(crate) struct SdkWriter {
    sdk: Arc<dash_sdk::Sdk>,
}

impl std::fmt::Debug for SdkWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdkWriter").finish()
    }
}

impl SdkWriter {
    /// Wrap an SDK handle.
    pub(crate) fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self { sdk }
    }

    /// Build the ECDH provider + xpub closure from the pre-derived
    /// inputs and broadcast the contact-request document.
    pub(crate) async fn send_contact_request(
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
            shared_secret,
            expected_recipient_pubkey,
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

        // Client-side ECDH: the caller already derived the shared secret
        // against `expected_recipient_pubkey`; hand it back, guarding that
        // the SDK asks for ECDH against that exact recipient key (the
        // client-side equivalent of the old SdkSide key-id guard). The
        // `F`/`Fut` (SdkSide) type params are unused here, so a never-called
        // `fn` placeholder satisfies their bounds.
        // Aliased so the annotation stays under the type-complexity lint.
        type UnusedSdkSideEcdh = fn(
            &IdentityPublicKey,
            u32,
        ) -> std::future::Ready<
            Result<dashcore::secp256k1::SecretKey, dash_sdk::Error>,
        >;
        let ecdh_provider: EcdhProvider<UnusedSdkSideEcdh, _, _, _> = EcdhProvider::ClientSide {
            get_shared_secret: move |peer: &dashcore::secp256k1::PublicKey| {
                let peer_matches = *peer == expected_recipient_pubkey;
                async move {
                    if !peer_matches {
                        return Err(dash_sdk::Error::Generic(
                            "ECDH recipient-key mismatch: the SDK resolved a recipient \
                             encryption key different from the one the shared secret was \
                             derived against"
                                .to_string(),
                        ));
                    }
                    Ok(*shared_secret)
                }
            },
        };

        let xpub_bytes_clone = xpub_bytes.clone();
        self.sdk
            .send_contact_request(send_input, ecdh_provider, |_account_ref: u32| async move {
                Ok::<Vec<u8>, dash_sdk::Error>(xpub_bytes_clone)
            })
            .await
            .map_err(PlatformWalletError::Sdk)
    }

    /// Broadcast a pre-built DashPay document and wait for the
    /// confirmation proof.
    pub(crate) async fn put_document(
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
