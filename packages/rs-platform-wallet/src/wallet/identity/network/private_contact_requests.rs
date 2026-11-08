//! DashPay private contact requests (protocol version 15): send, sync, accept.
//!
//! A private contact request is an entry of DashPay's `privateContactRequest`
//! store, sealed to one of the recipient's DashPay keys and appended by a
//! transition paid from the shielded pool, so nothing on chain names either
//! side. The sender names itself and its encryption key inside, signs it with
//! the identity signer it signs transitions with, and seals with the outgoing
//! viewing key of the encryption key, so a wallet restored from seed recovers
//! what it sent.
//!
//! The wallet keeps a private request as it keeps a public one, a
//! [`ContactRequest`], so contacts establish, rotate and get their DashPay
//! accounts the same way whichever way a request came. The entry carries the
//! compact xpub and the label in the clear; the wallet encrypts them under the
//! pair's DIP-15 shared key, as a `contactRequest` document carries them, with
//! IVs from the entry's commitment so a rescan rebuilds the same record, and at
//! core height 0, which no `contactRequest` document has: that a request came
//! privately is the record's own ([`ContactRequest::came_privately`]), so it
//! holds across restarts. The contact's shielded address, which a
//! `ContactRequest` has no field for, is kept in memory and read with
//! [`PrivateContact`].
//!
//! The store is opened and requests are sealed inside the
//! [`ContactCryptoProvider`], which keeps the key material; requests are signed
//! by the identity signer. Checking an entry needs no secret key.
//!
//! [`PrivateContact`]: crate::wallet::identity::PrivateContact

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::dashpay::{
    private_contact_request_identities, private_contact_request_signature,
    private_contact_request_signing_data, private_contact_request_signing_keys,
    private_contact_requests_from_scan, recipient_key_purpose_is_valid, PrivateContactRequest,
    ProvedIdentity, RejectedPrivateContactRequest, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
};
use dash_sdk::platform::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntriesQuery,
};
use dash_sdk::platform::private_document_scan::PrivateDocumentScanKeys;
use dash_sdk::platform::Fetch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{RngCore, SeedableRng};
use dpp::secp256k1::{PublicKey, SecretKey};
use key_wallet::bip32::{DerivationPath, KeyDerivationType};
use platform_encryption::{encrypt_account_label, encrypt_extended_public_key, COMPACT_XPUB_LEN};
use zeroize::Zeroize;

use super::contact_requests::{
    ingest_received_requests, ingest_sent_requests, newest_received_per_sender,
    newest_sent_per_recipient, tear_down_stale_external_accounts, AutoAcceptProofSource,
    ContactCryptoProvider, PreparedContactRequest,
};
use super::signing_key::first_available;
use super::{dashpay_contract, DashPayView, IdentityWallet};
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::dashpay::contact_request::ContactRequest;
use crate::wallet::identity::types::dashpay::established_contact::EstablishedContact;
use crate::wallet::identity::ManagedIdentity;

/// The paths of the keys a wallet's identities read DashPay's private contact
/// request store with, for a [`ContactCryptoProvider`] to derive inside the
/// signer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivateDocumentKeyPaths {
    /// Each identity entries may be sealed to, with the path of each of its
    /// keys an entry may be sealed to, by key id.
    pub recipients: Vec<(Identifier, Vec<(KeyID, DerivationPath)>)>,
    /// Each identity that may have sealed entries, with the id and path of
    /// each of its encryption keys: one outgoing viewing key each.
    pub senders: Vec<(Identifier, KeyID, DerivationPath)>,
}

impl PrivateDocumentKeyPaths {
    /// The scan keys these paths derive to, `derive` deriving the secret key at
    /// a path: for a provider implementation to build inside the signer and
    /// drop there.
    pub fn scan_keys<E>(
        &self,
        derive: impl FnMut(&DerivationPath) -> Result<SecretKey, E>,
    ) -> Result<PrivateDocumentScanKeys, E> {
        PrivateDocumentScanKeys::derive(&self.recipients, &self.senders, derive)
    }
}

/// What a pass over DashPay's private contact request store did.
#[derive(Clone, Debug, Default)]
pub struct PrivateContactSyncReport {
    /// The store pages read.
    pub pages_read: usize,
    /// The requests ingested, received and sent, as the wallet records them.
    pub requests: Vec<ContactRequest>,
    /// Entries that opened for one of the wallet's identities but were
    /// refused.
    pub rejected: Vec<RejectedPrivateContactRequest>,
    /// The store position the pass stopped at.
    pub next_position: u64,
    /// Whether the pass read to the store's last entry.
    pub complete: bool,
    /// Identities whose ingest did not reach disk, or one of whose sent
    /// requests could not be rebuilt for want of its recipient: their cursor
    /// stays where it was, so the next pass reads their entries again.
    pub unpersisted_identities: Vec<Identifier>,
}

/// How many pages of the store a send reads back after paying: enough for an
/// identity that synced recently, whose new entry is in the last page or two.
/// A request not read back within them stays pending for a later sync.
const POST_SEND_SYNC_PAGES: usize = 4;

/// One of the wallet's identities, as a pass reads the store for it.
struct ScannedIdentity {
    identity_id: Identifier,
    identity_index: u32,
    /// Its cursor when the pass last advanced it; a change under the pass is a
    /// rewind, which the pass leaves alone.
    position: u64,
    /// Its ingest failed this pass: it is not advanced again until the next.
    failed: bool,
    /// Its cursor was rewound under the pass (unignoring a sender): it is not
    /// advanced again this pass, so the next reads from the rewound position.
    rewound: bool,
}

impl ScannedIdentity {
    /// Advance `managed`'s cursor to `next_position` once a page's ingest for
    /// it reached disk: unless it was rewound under the pass, which stops it
    /// advancing for the rest of the pass, as a later page's advance would
    /// otherwise overwrite the rewind.
    fn advance(&mut self, managed: &mut ManagedIdentity, next_position: u64) {
        if self.failed || self.rewound {
            return;
        }
        if managed.dashpay().private_contact_requests_position() != self.position {
            self.rewound = true;
            return;
        }
        managed.advance_private_contact_requests_position(self.position, next_position);
        self.position = managed.dashpay().private_contact_requests_position();
    }
}

/// The local record of a private request: a [`ContactRequest`] carrying the
/// request's compact xpub and label encrypted under `shared_key`, the pair's
/// DIP-15 shared key, as a `contactRequest` document carries them. The IVs are
/// the two halves of the entry's `commitment`, so a rescan of the entry
/// rebuilds the same record, and the wallet's comparisons of a contact's
/// encrypted xpub hold across passes. Its core height is 0, which marks it
/// private ([`ContactRequest::came_privately`]); our receiving account for the
/// contact is then scanned from the wallet's birth.
#[allow(clippy::too_many_arguments)]
pub(super) fn private_request_record(
    sender_id: Identifier,
    recipient_id: Identifier,
    sender_key_index: KeyID,
    recipient_key_index: KeyID,
    request: &PrivateContactRequest,
    commitment: &[u8; 32],
    shared_key: &[u8; 32],
    created_at: u64,
) -> ContactRequest {
    let mut xpub_iv = [0u8; 16];
    let mut label_iv = [0u8; 16];
    xpub_iv.copy_from_slice(&commitment[..16]);
    label_iv.copy_from_slice(&commitment[16..]);
    let mut record = ContactRequest::new(
        sender_id,
        recipient_id,
        sender_key_index,
        recipient_key_index,
        request.account_reference,
        encrypt_extended_public_key(shared_key, &xpub_iv, &request.extended_public_key),
        0,
        created_at,
    );
    record.encrypted_account_label = request
        .account_label
        .as_deref()
        .map(|label| encrypt_account_label(shared_key, &label_iv, label));
    record.auto_accept_proof = request.auto_accept_proof.clone();
    record
}

/// The public key of `identity`'s key `key_id`.
fn identity_public_key(
    identity: &Identity,
    key_id: KeyID,
) -> Result<PublicKey, PlatformWalletError> {
    let key = identity.public_keys().get(&key_id).ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "identity {} has no key {key_id}",
            identity.id()
        ))
    })?;
    PublicKey::from_slice(key.data().as_slice()).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "key {key_id} of identity {} is not a secp256k1 key: {e}",
            identity.id()
        ))
    })
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Send a private contact request from `sender_identity_id` to
    /// `recipient_identity_id`: the request a `contactRequest` document would
    /// carry (our receiving xpub for the pair, the masked account reference,
    /// the label and the auto-accept proof) plus our `shielded_address`,
    /// signed by `signer` with one of the sender's signing keys, sealed in the
    /// crypto provider to the recipient's DashPay key, then appended by `pay`,
    /// which pays for it from the shielded pool and returns the metadata of
    /// the proof that it was appended
    /// (`PlatformWallet::send_private_contact_request`).
    ///
    /// The keys and the account reference are chosen as for a document send,
    /// so a later request to the same recipient, either way, rotates past this
    /// one.
    ///
    /// One private request to a recipient is in flight at a time: it is
    /// reserved before sealing, and while it is pending another is refused
    /// with [`PlatformWalletError::PrivateContactRequestPending`], as the store
    /// keeps every entry appended and a second payment could execute too.
    ///
    /// The proof `pay` waits for shows the notes spent, not the entry
    /// appended, so a paid request is recorded only once read back: a private
    /// sync of a few pages follows the payment, and records the request from
    /// the store as it records any request the identity sent. If it is not
    /// read back yet, the send returns `PrivateContactRequestPending` and the
    /// request stays pending until a later sync reads it. An unconfirmed
    /// payment stays pending the same way; one that definitely failed
    /// releases it. The pending request is kept in memory only.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_private_contact_request<S, C, F, Fut>(
        &self,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        account_label: Option<String>,
        auto_accept_proof: AutoAcceptProofSource,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        pay: F,
    ) -> Result<ContactRequest, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        let reserved = self
            .with_managed(sender_identity_id, |managed| {
                managed.reserve_private_reply(*recipient_identity_id)
            })
            .await?;
        if !reserved {
            return Err(PlatformWalletError::PrivateContactRequestPending(
                *recipient_identity_id,
            ));
        }
        let sent = self
            .send_reserved_private_contact_request(
                sender_identity_id,
                recipient_identity_id,
                account_label,
                auto_accept_proof,
                shielded_address,
                signer,
                crypto,
                pay,
            )
            .await;
        // A payment that may have executed keeps the request pending.
        let keep_pending = matches!(
            &sent,
            Err(PlatformWalletError::PrivateContactRequestPending(_)
                | PlatformWalletError::ShieldedSpendUnconfirmed { .. })
        );
        if !keep_pending {
            self.with_managed(sender_identity_id, |managed| {
                managed.release_private_reply(recipient_identity_id)
            })
            .await?;
        }
        sent
    }

    /// Run `f` on the managed identity `identity_id` under the wallet lock.
    async fn with_managed<R>(
        &self,
        identity_id: &Identifier,
        f: impl FnOnce(&mut ManagedIdentity) -> R,
    ) -> Result<R, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .identity_manager
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(f(managed))
    }

    /// The body of [`Self::send_private_contact_request`], once the request
    /// to the recipient is reserved.
    #[allow(clippy::too_many_arguments)]
    async fn send_reserved_private_contact_request<S, C, F, Fut>(
        &self,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        account_label: Option<String>,
        auto_accept_proof: AutoAcceptProofSource,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        pay: F,
    ) -> Result<ContactRequest, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        let PreparedContactRequest {
            sender_identity,
            sender_key_index,
            sender_enc_path,
            recipient_identity,
            recipient_key_index,
            recipient_enc_pubkey,
            account_index,
            contact_xpub_ext,
            xpub_bytes,
            account_reference,
            auto_accept_proof,
        } = self
            .prepare_contact_request(
                sender_identity_id,
                recipient_identity_id,
                auto_accept_proof,
                crypto,
            )
            .await?;
        let extended_public_key: [u8; COMPACT_XPUB_LEN] =
            xpub_bytes.as_slice().try_into().map_err(|_| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "the receiving xpub is {} bytes, not the {COMPACT_XPUB_LEN}-byte compact form",
                    xpub_bytes.len()
                ))
            })?;
        let request = PrivateContactRequest {
            account_reference,
            extended_public_key,
            account_label,
            auto_accept_proof,
            shielded_address,
        };

        // The request is signed as a public request's transition is: by the
        // first signing key of the sender the signer holds, of those the SDK
        // lets sign it (level, type, contract bounds).
        let contract = dashpay_contract()?;
        let signing_key = first_available(
            private_contact_request_signing_keys(&contract, &sender_identity)?.into_iter(),
            signer,
        )?
        .cloned()
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "the sender identity has no enabled AUTHENTICATION key at HIGH or CRITICAL, \
                 bound to no other contract, to sign a private contact request with"
                    .to_string(),
            )
        })?;
        let signing_data = private_contact_request_signing_data(
            &contract,
            &sender_identity,
            sender_key_index,
            &recipient_identity,
            recipient_key_index,
            &request,
            self.sdk.version(),
        )?;
        let signature =
            private_contact_request_signature(signer, &signing_key, &signing_data).await?;

        let mut esk_seed = [0u8; 32];
        StdRng::try_from_os_rng()
            .map_err(|error| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "the OS RNG failed while sealing a private contact request: {error}"
                ))
            })?
            .fill_bytes(&mut esk_seed);
        let sealed = crypto
            .seal_private_contact_request(
                &sender_enc_path,
                &contract,
                &sender_identity,
                sender_key_index,
                signing_key.id(),
                &signature,
                &recipient_identity,
                recipient_key_index,
                &request,
                &esk_seed,
                self.sdk.version(),
            )
            .await;
        esk_seed.zeroize();
        let entry = sealed?;
        // An entry starts with its 32-byte commitment.
        let commitment: [u8; 32] = entry
            .as_slice()
            .get(..32)
            .and_then(|commitment| commitment.try_into().ok())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "the provider sealed an entry shorter than its commitment".to_string(),
                )
            })?;
        self.with_managed(sender_identity_id, |managed| {
            managed.note_private_reply_commitment(*recipient_identity_id, commitment)
        })
        .await?;
        // What the record read back must carry, computed before paying.
        let shared_secret = crypto
            .ecdh_shared_secret(&sender_enc_path, &recipient_enc_pubkey)
            .await?;
        let expected = private_request_record(
            *sender_identity_id,
            *recipient_identity_id,
            sender_key_index,
            recipient_key_index,
            &request,
            &commitment,
            &shared_secret,
            0,
        );

        pay(entry).await?;

        // Recorded only as the store proves it: read it back.
        if let Err(error) = self
            .sync_private_contact_requests(crypto, POST_SEND_SYNC_PAGES)
            .await
        {
            tracing::warn!(
                sender = %sender_identity_id,
                recipient = %recipient_identity_id,
                %error,
                "private contact request paid for; reading it back failed, a later sync reads it"
            );
        }
        let recorded = self
            .with_managed(sender_identity_id, |managed| {
                managed
                    .dashpay()
                    .outgoing_request(recipient_identity_id)
                    .filter(|tracked| {
                        tracked.account_reference == expected.account_reference
                            && tracked.encrypted_public_key == expected.encrypted_public_key
                    })
                    .cloned()
            })
            .await?;
        let Some(contact_request) = recorded else {
            return Err(PlatformWalletError::PrivateContactRequestPending(
                *recipient_identity_id,
            ));
        };
        if let Err(e) = self
            .register_contact_account(
                sender_identity_id,
                recipient_identity_id,
                account_index,
                contact_xpub_ext,
            )
            .await
        {
            tracing::warn!(
                sender = %sender_identity_id,
                recipient = %recipient_identity_id,
                error = %e,
                "private contact request sent; its receiving account is not registered yet, the \
                 next sync registers it"
            );
        }
        Ok(contact_request)
    }

    /// Accept a contact request that came privately, replying privately: as
    /// [`Self::accept_contact_request_with_external_signer`], with the reply
    /// sent by [`Self::send_private_contact_request`], signed by `signer`,
    /// `pay` paying for it and `shielded_address` the address the contact may
    /// pay us at privately.
    pub async fn accept_private_contact_request<S, C, F, Fut>(
        &self,
        request: &ContactRequest,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        pay: F,
    ) -> Result<EstablishedContact, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        self.accept_contact_request_replying_with(request, crypto, || async {
            self.send_private_contact_request(
                &request.recipient_id,
                &request.sender_id,
                None,
                AutoAcceptProofSource::None,
                shielded_address,
                signer,
                crypto,
                pay,
            )
            .await
            .map(|_| ())
        })
        .await
    }

    /// Accept `request` replying the way it came: privately, paid by `pay`,
    /// when it came privately and we have sent its sender no request yet
    /// ([`Self::accept_private_contact_request`]); otherwise with a
    /// `contactRequest` document, or by adopting the reply already sent
    /// ([`Self::accept_contact_request_with_external_signer`]). The one accept
    /// a host calls: which reply a request takes is the wallet's decision.
    /// `pay` runs only for a private reply.
    pub async fn accept_contact_request_replying_in_kind<S, C, F, Fut>(
        &self,
        request: &ContactRequest,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        pay: F,
    ) -> Result<EstablishedContact, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        if self.needs_private_reply(request).await {
            self.accept_private_contact_request(request, shielded_address, signer, crypto, pay)
                .await
        } else {
            self.accept_contact_request_with_external_signer(request, signer, crypto)
                .await
        }
    }

    /// The wallet's identities a pass reads the store for (those with an HD
    /// index, whose keys the signer can derive) and the paths of their keys:
    /// every `ECDSA_SECP256K1` decryption or encryption key, disabled ones
    /// included, as an entry sealed before a key was disabled still opens with
    /// it, and every such encryption key as a sender.
    async fn private_scan_identities(
        &self,
    ) -> Result<(Vec<ScannedIdentity>, PrivateDocumentKeyPaths), PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let mut identities = Vec::new();
        let mut paths = PrivateDocumentKeyPaths::default();
        for identity in info.identity_manager.all_identities() {
            let identity_id = identity.id();
            let Some(managed) = info.identity_manager.managed_identity(&identity_id) else {
                continue;
            };
            let Some(identity_index) = managed.identity_index else {
                continue;
            };
            let path = |key_id| {
                IdentityWallet::<B>::identity_auth_derivation_path(
                    self.sdk.network,
                    KeyDerivationType::ECDSA,
                    identity_index,
                    key_id,
                )
            };
            let mut recipient_paths = Vec::new();
            for (key_id, key) in managed.identity.public_keys() {
                if key.key_type() != KeyType::ECDSA_SECP256K1
                    || !recipient_key_purpose_is_valid(key.purpose())
                {
                    continue;
                }
                recipient_paths.push((*key_id, path(*key_id)?));
                if key.purpose() == Purpose::ENCRYPTION {
                    paths.senders.push((identity_id, *key_id, path(*key_id)?));
                }
            }
            if recipient_paths.is_empty() {
                continue;
            }
            paths.recipients.push((identity_id, recipient_paths));
            identities.push(ScannedIdentity {
                identity_id,
                identity_index,
                position: managed.dashpay().private_contact_requests_position(),
                failed: false,
                rewound: false,
            });
        }
        Ok((identities, paths))
    }

    /// Read DashPay's private contact request store for the wallet's
    /// identities, at most `max_pages` pages from the earliest of their
    /// cursors, and ingest what it holds for them: requests to them, each
    /// checked against the keys of its sender and of the identity it is to,
    /// and requests they sent, recovered from the store alone and checked to
    /// be theirs. Contacts establish, and their DashPay accounts are queued,
    /// as the document sync does. A request older than the one tracked with
    /// the same contact, either way and from either channel, is passed over.
    ///
    /// A request that came privately carries no auto-accept proof into the
    /// wallet: accepting it replies privately, which pays from the shielded
    /// pool, so it is accepted by hand
    /// ([`Self::accept_private_contact_request`]).
    pub async fn sync_private_contact_requests<C>(
        &self,
        crypto: &C,
        max_pages: usize,
    ) -> Result<PrivateContactSyncReport, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let (mut identities, paths) = self.private_scan_identities().await?;
        let mut report = PrivateContactSyncReport::default();
        let Some(start) = identities.iter().map(|identity| identity.position).min() else {
            report.complete = true;
            return Ok(report);
        };
        let identity_indexes: BTreeMap<Identifier, u32> = identities
            .iter()
            .map(|identity| (identity.identity_id, identity.identity_index))
            .collect();
        let key_path = |identity_id: &Identifier, key_id: KeyID| {
            let identity_index = identity_indexes
                .get(identity_id)
                .copied()
                .ok_or_else(|| PlatformWalletError::IdentityNotFound(*identity_id))?;
            IdentityWallet::<B>::identity_auth_derivation_path(
                self.sdk.network,
                KeyDerivationType::ECDSA,
                identity_index,
                key_id,
            )
        };
        let contract = dashpay_contract()?;
        // Fetched once a pass, and again when a later page's entries are newer
        // than the proof: the senders and the wallet's own identities, whose
        // keys a request is checked against as they were when it was appended.
        let mut proved_identities: BTreeMap<Identifier, ProvedIdentity> = BTreeMap::new();
        // The recipients of the requests the wallet's identities sent, read for
        // the key id of the key a request is sealed to, as they were when it
        // was appended or later.
        let mut recipients: BTreeMap<Identifier, ProvedIdentity> = BTreeMap::new();
        let mut position = start;
        report.next_position = start;

        while report.pages_read < max_pages {
            let query = PrivateDocumentEntriesQuery::new(
                contract.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                position,
            );
            let page = PrivateDocumentEntries::fetch(&self.sdk, query)
                .await?
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "no page of DashPay's private contact requests from position {position}"
                    ))
                })?;
            report.pages_read += 1;

            let scan = crypto
                .scan_private_document_page(
                    &paths,
                    contract.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    position,
                    &page,
                )
                .await?;
            let stale: BTreeMap<Identifier, u64> = private_contact_request_identities(&scan)
                .into_iter()
                .filter(|(identity_id, height)| {
                    proved_identities
                        .get(identity_id)
                        .is_none_or(|proved| proved.proved_at_height < *height)
                })
                .collect();
            proved_identities.extend(
                self.sdk
                    .fetch_private_contact_request_identities(&stale)
                    .await?,
            );
            let read = private_contact_requests_from_scan(
                &contract,
                scan,
                &proved_identities,
                self.sdk.version(),
            )?;
            // Each recipient is fetched on its own, so one that cannot be read
            // holds back only the identities whose sent requests it is for.
            let mut stale_recipients: BTreeMap<Identifier, u64> = BTreeMap::new();
            for sent in &read.sent {
                let height = stale_recipients.entry(sent.recipient_id).or_default();
                *height = (*height).max(sent.block_height);
            }
            stale_recipients.retain(|recipient_id, height| {
                recipients
                    .get(recipient_id)
                    .is_none_or(|proved| proved.proved_at_height < *height)
            });
            let mut unreadable_recipients = BTreeSet::new();
            for (recipient_id, height) in stale_recipients {
                match self
                    .sdk
                    .fetch_private_contact_request_identities(&BTreeMap::from([(
                        recipient_id,
                        height,
                    )]))
                    .await
                {
                    Ok(fetched) => recipients.extend(fetched),
                    Err(error) => {
                        tracing::warn!(
                            recipient = %recipient_id,
                            %error,
                            "the recipient of a sent private contact request could not be read; \
                             its sender's cursor is held"
                        );
                        unreadable_recipients.insert(recipient_id);
                    }
                }
            }
            let held: BTreeSet<Identifier> = read
                .sent
                .iter()
                .filter(|sent| unreadable_recipients.contains(&sent.recipient_id))
                .map(|sent| sent.sender_id)
                .collect();

            // The wallet's records of the page's requests, under the shared
            // keys the signer computes.
            let mut records: Vec<(Identifier, ContactRequest, bool, Option<[u8; 43]>)> = Vec::new();
            for received in &read.received {
                let Some(sender) = proved_identities
                    .get(&received.sender_id)
                    .and_then(|proved| proved.identity.as_ref())
                else {
                    continue;
                };
                let sender_key = identity_public_key(sender, received.sender_key_id)?;
                let shared_secret = crypto
                    .ecdh_shared_secret(
                        &key_path(&received.recipient_id, received.recipient_key_id)?,
                        &sender_key,
                    )
                    .await?;
                // Accepting replies privately, by hand: no auto-accept.
                let mut record = private_request_record(
                    received.sender_id,
                    received.recipient_id,
                    received.sender_key_id,
                    received.recipient_key_id,
                    &received.request,
                    &received.commitment,
                    &shared_secret,
                    received.block_time_ms,
                );
                record.auto_accept_proof = None;
                records.push((
                    received.recipient_id,
                    record,
                    true,
                    received.request.shielded_address,
                ));
            }
            for sent in &read.sent {
                if unreadable_recipients.contains(&sent.recipient_id) {
                    continue;
                }
                let recipient_key = sent.recipient_public_key.serialize();
                let recipient_key_id = recipients
                    .get(&sent.recipient_id)
                    .and_then(|proved| proved.identity.as_ref())
                    .and_then(|recipient| {
                        recipient
                            .public_keys()
                            .iter()
                            .find(|(_, key)| key.data().as_slice() == recipient_key.as_slice())
                            .map(|(key_id, _)| *key_id)
                    });
                let Some(recipient_key_id) = recipient_key_id else {
                    tracing::warn!(
                        sender = %sent.sender_id,
                        recipient = %sent.recipient_id,
                        position = sent.position,
                        "sent private contact request is sealed to a key its recipient does not \
                         hold; not recorded"
                    );
                    continue;
                };
                let shared_secret = crypto
                    .ecdh_shared_secret(
                        &key_path(&sent.sender_id, sent.sender_key_id)?,
                        &sent.recipient_public_key,
                    )
                    .await?;
                let record = private_request_record(
                    sent.sender_id,
                    sent.recipient_id,
                    sent.sender_key_id,
                    recipient_key_id,
                    &sent.request,
                    &sent.commitment,
                    &shared_secret,
                    sent.block_time_ms,
                );
                records.push((sent.sender_id, record, false, None));
            }
            for rejected in &read.rejected {
                tracing::debug!(
                    position = rejected.position,
                    reason = ?rejected.reason,
                    "private contact request refused"
                );
            }
            report.rejected.extend(read.rejected.iter().cloned());

            // Ingest what the page held for each identity, then advance its
            // cursor past the page if that reached disk.
            let mut candidates = Vec::new();
            {
                let mut wm = self.wallet_manager.write().await;
                let (wallet, info) =
                    wm.get_wallet_mut_and_info_mut(&self.wallet_id)
                        .ok_or_else(|| {
                            PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id))
                        })?;
                for identity in identities.iter_mut() {
                    let identity_id = identity.identity_id;
                    let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id)
                    else {
                        continue;
                    };
                    if held.contains(&identity_id) && !identity.failed {
                        identity.failed = true;
                        report.unpersisted_identities.push(identity_id);
                    }
                    let mut received = Vec::new();
                    let mut addresses = Vec::new();
                    let mut sent = Vec::new();
                    for (owner, record, is_received, shielded_address) in &records {
                        if *owner != identity_id {
                            continue;
                        }
                        if *is_received {
                            addresses.push((record.clone(), *shielded_address));
                            received.push(record.clone());
                        } else {
                            // Our receiving account for the contact is
                            // scanned from the wallet's birth.
                            managed.note_sent_request_core_height(record.recipient_id, 0);
                            sent.push(record.clone());
                        }
                    }
                    report.requests.extend(received.iter().cloned());
                    report.requests.extend(sent.iter().cloned());

                    let mut rotated = Vec::new();
                    let mut ingested = Vec::new();
                    let received_ok = ingest_received_requests(
                        managed,
                        &self.persister,
                        identity_id,
                        newest_received_per_sender(received),
                        &mut rotated,
                        &mut ingested,
                    );
                    let sent_ok = ingest_sent_requests(
                        managed,
                        &self.persister,
                        identity_id,
                        newest_sent_per_recipient(sent),
                    );
                    // The address of the request tracked from each contact,
                    // once ingest decided which that is.
                    for (record, shielded_address) in &addresses {
                        managed.note_private_shielded_address(record, *shielded_address);
                    }
                    if received_ok && sent_ok {
                        // A pending private request read back and recorded is
                        // settled.
                        for sent in read
                            .sent
                            .iter()
                            .filter(|sent| sent.sender_id == identity_id)
                        {
                            managed.settle_private_reply(&sent.recipient_id, &sent.commitment);
                        }
                        identity.advance(managed, read.next_position);
                    } else if !identity.failed {
                        identity.failed = true;
                        report.unpersisted_identities.push(identity_id);
                    }
                    tear_down_stale_external_accounts(wallet, info, &identity_id, rotated);
                    candidates.push((
                        identity_id,
                        Self::collect_account_build_candidates(info, &identity_id),
                    ));
                }
            }
            for (identity_id, identity_candidates) in candidates {
                for candidate in identity_candidates {
                    self.build_contact_accounts(&identity_id, candidate).await;
                }
            }

            report.next_position = read.next_position;
            position = read.next_position;
            if read.is_complete() {
                // A pass whose ingest did not reach disk for an identity is
                // not complete: that identity reads its entries again.
                report.complete = report.unpersisted_identities.is_empty();
                break;
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::identity::network::identity_auth_derivation_path_for_type;
    use crate::wallet::identity::network::SeedCryptoProvider;
    use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
    use crate::PlatformWalletManager;
    use dash_sdk::error::ContextProviderError;
    use dash_sdk::platform::private_document_entries::PrivateDocumentEntry;
    use dash_sdk::platform::ContextProvider;
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{IdentityPublicKey, SecurityLevel};
    use dpp::version::PlatformVersion;
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;
    use platform_encryption::{decrypt_account_label, decrypt_extended_public_key};
    use std::sync::{Arc, Mutex};

    const ALICE_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
                                abandon abandon abandon about";
    const BOB_PHRASE: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const ALICE: [u8; 32] = [0xA1; 32];
    const BOB: [u8; 32] = [0xB1; 32];
    /// The time of the block that appends the test entries.
    const BLOCK_TIME_MS: u64 = 1_700_000_000_000;
    /// The key ids of each test identity's signing key and DashPay keys.
    const SIGNING_KEY_ID: KeyID = 0;
    const ENCRYPTION_KEY_ID: KeyID = 1;
    const DECRYPTION_KEY_ID: KeyID = 2;

    #[derive(Default)]
    struct NoPersistence;

    impl PlatformWalletPersistence for NoPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// A context provider holding the bundled DashPay contract.
    struct DashpayContractProvider(Arc<DataContract>);

    impl ContextProvider for DashpayContractProvider {
        fn get_data_contract(
            &self,
            id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok((*id == self.0.id()).then(|| Arc::clone(&self.0)))
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Err(ContextProviderError::Config(
                "no quorum keys in this test".into(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
            Ok(1)
        }
    }

    fn seed(phrase: &str) -> [u8; 64] {
        *Mnemonic::from_phrase(phrase)
            .expect("test mnemonic")
            .to_seed("")
    }

    /// The secret key `key_id` of identity index 0 of `phrase`'s seed.
    fn identity_secret_key(phrase: &str, key_id: KeyID) -> SecretKey {
        let wallet = key_wallet::wallet::Wallet::from_seed_bytes(
            seed(phrase),
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("test wallet");
        let path = identity_auth_derivation_path_for_type(
            Network::Testnet,
            KeyDerivationType::ECDSA,
            0,
            key_id,
        )
        .expect("identity key path");
        let xprv = wallet
            .derive_extended_private_key(&path)
            .expect("identity key");
        SecretKey::from_secret_bytes(*xprv.private_key.to_bytes()).expect("secret key")
    }

    /// The identity `id` whose keys are those of identity index 0 of
    /// `phrase`'s seed: its HIGH signing key 0, encryption key 1 and
    /// decryption key 2.
    fn identity(phrase: &str, id: [u8; 32]) -> Identity {
        let key = |key_id: KeyID, purpose: Purpose, security_level: SecurityLevel| {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: key_id,
                purpose,
                security_level,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&identity_secret_key(phrase, key_id))
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            })
        };
        let keys = [
            key(SIGNING_KEY_ID, Purpose::AUTHENTICATION, SecurityLevel::HIGH),
            key(
                ENCRYPTION_KEY_ID,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
            ),
            key(
                DECRYPTION_KEY_ID,
                Purpose::DECRYPTION,
                SecurityLevel::MEDIUM,
            ),
        ];
        Identity::V0(IdentityV0 {
            id: Identifier::from(id),
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        })
    }

    /// The signer holding `identity`'s signing key, as a host's signer holds
    /// the keys it signs transitions with.
    fn signer(phrase: &str, identity: &Identity) -> simple_signer::signer::SimpleSigner {
        let mut signer = simple_signer::signer::SimpleSigner::default();
        signer.add_identity_public_key(
            identity
                .get_public_key_by_id(SIGNING_KEY_ID)
                .expect("the signing key")
                .clone(),
            identity_secret_key(phrase, SIGNING_KEY_ID).to_secret_bytes(),
        );
        signer
    }

    /// A wallet of `phrase` holding `own` at identity index 0, on a mock SDK that
    /// answers identity fetches with `known` and page 0 of DashPay's private
    /// contact request store with `entries`.
    async fn wallet(
        phrase: &str,
        own: &Identity,
        known: &[&Identity],
        entries: Vec<Vec<u8>>,
    ) -> (PlatformWalletManager<NoPersistence>, Arc<PlatformWallet>) {
        let contract = dashpay_contract().expect("bundled DashPay");
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_version(PlatformVersion::latest())
            .with_context_provider(DashpayContractProvider(Arc::clone(&contract)))
            .build()
            .expect("mock sdk");
        for identity in known {
            sdk.mock()
                .expect_fetch(identity.id(), Some((*identity).clone()))
                .await
                .expect("identity expectation");
        }
        let total_count = entries.len() as u64;
        let page = PrivateDocumentEntries {
            entries: entries
                .into_iter()
                .zip(0u64..)
                // The mock proves identities at height 0, which an entry's
                // identities must be proved at or after.
                .map(|(data, position)| PrivateDocumentEntry {
                    position,
                    block_height: 0,
                    block_time_ms: BLOCK_TIME_MS,
                    data,
                })
                .collect(),
            total_count,
        };
        sdk.mock()
            .expect_fetch(
                PrivateDocumentEntriesQuery::new(
                    contract.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                ),
                Some(page),
            )
            .await
            .expect("page expectation");

        let manager = PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoPersistence),
            Arc::new(NoopEventHandler),
        );
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed(phrase),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet");
        {
            let iw = wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet.wallet_id())
                .expect("wallet info");
            info.identity_manager
                .add_identity(own.clone(), 0, wallet.wallet_id(), &iw.persister)
                .expect("own identity");
        }
        (manager, wallet)
    }

    /// The page of DashPay's private store from `start`, holding `entries`.
    fn store_page(start: u64, entries: Vec<Vec<u8>>) -> PrivateDocumentEntries {
        let total_count = start + entries.len() as u64;
        PrivateDocumentEntries {
            entries: entries
                .into_iter()
                .zip(start..)
                .map(|(data, position)| PrivateDocumentEntry {
                    position,
                    block_height: 0,
                    block_time_ms: BLOCK_TIME_MS,
                    data,
                })
                .collect(),
            total_count,
        }
    }

    /// From now on, `wallet`'s mock SDK answers the page from `start` with
    /// `entries`: the store as it is once they are appended.
    async fn publish(wallet: &PlatformWallet, start: u64, entries: Vec<Vec<u8>>) {
        let contract = dashpay_contract().expect("bundled DashPay");
        let query = PrivateDocumentEntriesQuery::new(
            contract.id(),
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            start,
        );
        let mut sdk = (*wallet.identity().sdk).clone();
        let mut mock = sdk.mock();
        mock.remove_fetch_expectation::<PrivateDocumentEntries, _>(query.clone())
            .await;
        mock.expect_fetch(query, Some(store_page(start, entries)))
            .await
            .expect("page expectation");
    }

    /// A `pay` that appends the entry to the store `wallet` reads, after
    /// `before` from `start`, and keeps it.
    fn append_entry(
        wallet: &Arc<PlatformWallet>,
        start: u64,
        before: Vec<Vec<u8>>,
        kept: &Arc<Mutex<Vec<BinaryData>>>,
    ) -> impl FnOnce(
        BinaryData,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send>,
    > + Send {
        let wallet = Arc::clone(wallet);
        let kept = Arc::clone(kept);
        move |entry| {
            Box::pin(async move {
                let mut entries = before;
                entries.push(entry.to_vec());
                publish(&wallet, start, entries).await;
                kept.lock().expect("kept entries").push(entry);
                Ok(ResponseMetadata {
                    time_ms: BLOCK_TIME_MS,
                    ..Default::default()
                })
            })
        }
    }

    /// A `pay` that keeps the entry instead of paying for it, and answers as
    /// the proof of a block at `BLOCK_TIME_MS` would: the entry never reaches
    /// the store.
    fn keep_entry(
        kept: &Arc<Mutex<Vec<BinaryData>>>,
    ) -> impl FnOnce(BinaryData) -> std::future::Ready<Result<ResponseMetadata, PlatformWalletError>>
           + Send {
        let kept = Arc::clone(kept);
        move |entry| {
            kept.lock().expect("kept entries").push(entry);
            std::future::ready(Ok(ResponseMetadata {
                time_ms: BLOCK_TIME_MS,
                ..Default::default()
            }))
        }
    }

    async fn managed<R>(
        wallet: &PlatformWallet,
        identity_id: [u8; 32],
        read: impl FnOnce(&crate::wallet::identity::ManagedIdentity) -> R,
    ) -> R {
        let iw = wallet.identity();
        let wm = iw.wallet_manager.read().await;
        let managed = wm
            .get_wallet_info(&wallet.wallet_id())
            .expect("wallet info")
            .identity_manager
            .managed_identity(&Identifier::from(identity_id))
            .expect("managed identity");
        read(managed)
    }

    /// Alice sends Bob a private request; Bob reads it from the store with
    /// Alice checked as its sender, refuses to accept it with a public reply,
    /// and accepts it with a private one, which establishes the contact. The
    /// request Bob records carries Alice's receiving xpub and label as a
    /// document would, under the pair's shared key.
    #[tokio::test]
    async fn should_send_read_and_accept_a_private_contact_request() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let shielded_address = [0x55; 43];

        let (_alice_manager, alice_wallet) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        let sent = alice_wallet
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                Some("Alice".to_string()),
                AutoAcceptProofSource::None,
                Some(shielded_address),
                &alice_signer,
                &alice_crypto,
                append_entry(&alice_wallet, 0, vec![], &kept),
            )
            .await
            .expect("Alice sends the request, read back from the store");
        let entry = kept.lock().expect("kept entries").remove(0);
        assert_eq!(entry.len(), 589, "the entry takes DashPay's entry size");
        assert_eq!(sent.sender_key_index, ENCRYPTION_KEY_ID);
        assert_eq!(sent.recipient_key_index, DECRYPTION_KEY_ID);
        assert_eq!(sent.created_at, BLOCK_TIME_MS);
        assert!(sent.came_privately(), "recorded at core height 0");
        managed(&alice_wallet, ALICE, |managed| {
            assert_eq!(
                managed.dashpay().sent_contact_requests().get(&bob.id()),
                Some(&sent)
            );
            assert!(
                managed
                    .dashpay()
                    .private_contact(&bob.id())
                    .expect("the request went privately")
                    .sent_privately
            );
        })
        .await;
        // Alice's public request to Bob would publish what she sent privately.
        assert!(matches!(
            alice_wallet
                .identity()
                .dashpay()
                .send_contact_request_with_external_signer(
                    &alice.id(),
                    &bob.id(),
                    None,
                    AutoAcceptProofSource::None,
                    &alice_signer,
                    &alice_crypto,
                )
                .await,
            Err(PlatformWalletError::PrivateContactRequestNeedsPrivateReply(recipient))
                if recipient == bob.id()
        ));

        let (_bob_manager, bob_wallet) =
            wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry.to_vec()]).await;
        let bob_view = bob_wallet.identity().dashpay();
        let report = bob_view
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");
        assert_eq!(report.pages_read, 1);
        assert!(report.complete);
        assert_eq!(report.next_position, 1);
        assert_eq!(report.rejected, vec![]);
        let received = managed(&bob_wallet, BOB, |managed| {
            assert_eq!(managed.dashpay().private_contact_requests_position(), 1);
            let private = managed
                .dashpay()
                .private_contact(&alice.id())
                .expect("noted");
            assert!(private.received_privately);
            assert_eq!(private.shielded_address, Some(shielded_address));
            managed
                .dashpay()
                .incoming_contact_requests()
                .get(&alice.id())
                .cloned()
                .expect("Bob holds Alice's request")
        })
        .await;
        assert_eq!(received.sender_key_index, ENCRYPTION_KEY_ID);
        assert_eq!(received.recipient_key_index, DECRYPTION_KEY_ID);
        assert_eq!(received.account_reference, sent.account_reference);
        assert_eq!(received.created_at, BLOCK_TIME_MS);
        assert!(received.came_privately(), "recorded at core height 0");
        assert_eq!(received.auto_accept_proof, None);
        // The shared key Bob's decryption key and Alice's encryption key agree
        // on opens the xpub and label, as it opens a document's.
        let bob_key_path = identity_auth_derivation_path_for_type(
            Network::Testnet,
            KeyDerivationType::ECDSA,
            0,
            DECRYPTION_KEY_ID,
        )
        .expect("Bob's decryption key path");
        let shared_key = bob_crypto
            .ecdh_shared_secret(
                &bob_key_path,
                &identity_public_key(&alice, ENCRYPTION_KEY_ID).expect("Alice's key"),
            )
            .await
            .expect("shared key");
        assert_eq!(
            decrypt_extended_public_key(&shared_key, &received.encrypted_public_key)
                .expect("the xpub opens"),
            decrypt_extended_public_key(&shared_key, &sent.encrypted_public_key)
                .expect("Alice's record opens with the same key"),
        );
        assert_eq!(
            received.encrypted_public_key, sent.encrypted_public_key,
            "both sides record the same ciphertext"
        );
        assert_eq!(
            decrypt_account_label(
                &shared_key,
                received
                    .encrypted_account_label
                    .as_deref()
                    .expect("a label")
            )
            .expect("the label opens"),
            "Alice"
        );

        // A public reply would publish the relationship.
        assert!(matches!(
            bob_view
                .accept_contact_request_with_external_signer(&received, &bob_signer, &bob_crypto)
                .await,
            Err(PlatformWalletError::PrivateContactRequestNeedsPrivateReply(sender))
                if sender == alice.id()
        ));

        // The accept a host calls replies the way the request came.
        let reply = Arc::new(Mutex::new(Vec::new()));
        let established = bob_view
            .accept_contact_request_replying_in_kind(
                &received,
                None,
                &bob_signer,
                &bob_crypto,
                append_entry(&bob_wallet, 1, vec![], &reply),
            )
            .await
            .expect("Bob accepts privately");
        assert_eq!(established.contact_identity_id, alice.id());
        assert_eq!(established.incoming_request, received);
        assert_eq!(
            reply.lock().expect("reply").len(),
            1,
            "the reply is a private entry"
        );
        managed(&bob_wallet, BOB, |managed| {
            let private = managed
                .dashpay()
                .private_contact(&alice.id())
                .expect("noted");
            assert!(private.received_privately && private.sent_privately);
        })
        .await;
        // Bob can pay Alice: her sending account is registered from the xpub.
        {
            let iw = bob_wallet.identity();
            let wm = iw.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&bob_wallet.wallet_id())
                .expect("wallet info");
            assert!(info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .contains_key(&DashpayAccountKey {
                    index: 0,
                    user_identity_id: BOB,
                    friend_identity_id: ALICE,
                }));
        }

        // Un-ignoring a sender reads the store again from its start, where the
        // requests skipped while it was ignored are.
        {
            let iw = bob_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&bob_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&bob.id())
                .expect("Bob");
            managed.ignore_sender(&alice.id());
            // Past Alice's request and Bob's reply, which the accept read back.
            assert_eq!(managed.dashpay().private_contact_requests_position(), 2);
            managed.unignore_sender(&alice.id());
            assert_eq!(managed.dashpay().private_contact_requests_position(), 0);
        }
    }

    /// A public request from `sender` to `recipient` for `account_reference`,
    /// as a `contactRequest` document is recorded: with its core height.
    fn public_request(
        sender: &Identity,
        recipient: &Identity,
        account_reference: u32,
    ) -> ContactRequest {
        ContactRequest::new(
            sender.id(),
            recipient.id(),
            ENCRYPTION_KEY_ID,
            DECRYPTION_KEY_ID,
            account_reference,
            vec![7; 96],
            100,
            BLOCK_TIME_MS,
        )
    }

    /// The accept a host calls replies to a public request publicly, without
    /// paying: by adopting the reply already sent, or with a document whose
    /// broadcast error comes back as it is. A request the wallet does not
    /// hold is refused either way, and nothing is paid.
    #[tokio::test]
    async fn should_accept_a_public_request_replying_in_kind() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let incoming = public_request(&alice, &bob, 3);
        let add = |wallet: Arc<PlatformWallet>, requests: Vec<(bool, ContactRequest)>| async move {
            let iw = wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&Identifier::from(BOB))
                .expect("Bob");
            for (incoming, request) in requests {
                if incoming {
                    managed
                        .add_incoming_contact_request(request, &iw.persister)
                        .expect("incoming request");
                } else {
                    managed
                        .add_sent_contact_request(request, &iw.persister)
                        .expect("sent request");
                }
            }
        };

        // Already reciprocated: adopted, no broadcast and no payment.
        let (_manager, reciprocated) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![]).await;
        add(
            Arc::clone(&reciprocated),
            vec![
                (true, incoming.clone()),
                (false, public_request(&bob, &alice, 4)),
            ],
        )
        .await;
        let paid = Arc::new(Mutex::new(Vec::new()));
        let established = reciprocated
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &incoming,
                None,
                &bob_signer,
                &bob_crypto,
                keep_entry(&paid),
            )
            .await
            .expect("the reply already sent is adopted");
        assert_eq!(established.contact_identity_id, alice.id());
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");

        // Not reciprocated: a public reply, whose broadcast this mock refuses.
        let (_manager, fresh) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![]).await;
        add(Arc::clone(&fresh), vec![(true, incoming.clone())]).await;
        let paid = Arc::new(Mutex::new(Vec::new()));
        let result = fresh
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &incoming,
                None,
                &bob_signer,
                &bob_crypto,
                keep_entry(&paid),
            )
            .await;
        assert!(
            matches!(&result, Err(error)
                if !matches!(error, PlatformWalletError::PrivateContactRequestNeedsPrivateReply(_))),
            "the public reply's error comes back: {result:?}"
        );
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");

        // A private request from a sender the wallet holds no request from.
        let carol = identity(ALICE_PHRASE, [0xC1; 32]);
        let mut unknown = public_request(&carol, &bob, 5);
        unknown.core_height_created_at = 0;
        let paid = Arc::new(Mutex::new(Vec::new()));
        assert!(matches!(
            fresh
                .identity()
                .dashpay()
                .accept_contact_request_replying_in_kind(
                    &unknown,
                    None,
                    &bob_signer,
                    &bob_crypto,
                    keep_entry(&paid),
                )
                .await,
            Err(PlatformWalletError::ContactRequestNotFound(sender)) if sender == carol.id()
        ));
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");
    }

    /// A wallet restored from seed recovers the private requests it sent from
    /// the store alone, recording each as it recorded it when sending.
    #[tokio::test]
    async fn should_recover_a_sent_private_contact_request_from_the_store() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);

        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        let sent = alice_wallet
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                append_entry(&alice_wallet, 0, vec![], &kept),
            )
            .await
            .expect("Alice sends the request");
        let entry = kept.lock().expect("kept entries").remove(0);

        // The restored wallet checks its own identity's keys as on chain.
        let (_restored_manager, restored) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![entry.to_vec()]).await;
        let report = restored
            .identity()
            .dashpay()
            .sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("the restored wallet reads the store");
        assert_eq!(report.requests, vec![sent.clone()]);
        managed(&restored, ALICE, |managed| {
            assert_eq!(
                managed.dashpay().sent_contact_requests().get(&bob.id()),
                Some(&sent)
            );
        })
        .await;
    }

    /// One private request to a recipient is in flight at a time: one paid
    /// for but not read back, or whose payment is unconfirmed, keeps the next
    /// from being sent, until a sync reads it back; one whose payment failed
    /// does not.
    #[tokio::test]
    async fn should_send_one_private_request_to_a_recipient_at_a_time() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        macro_rules! send {
            ($pay:expr) => {
                view.send_private_contact_request(
                    &alice.id(),
                    &bob.id(),
                    None,
                    AutoAcceptProofSource::None,
                    None,
                    &alice_signer,
                    &alice_crypto,
                    $pay,
                )
                .await
            };
        }
        let pending = |result: &Result<ContactRequest, PlatformWalletError>| matches!(result, Err(PlatformWalletError::PrivateContactRequestPending(id)) if *id == bob.id());

        // A payment that definitely failed releases the recipient.
        let failed = send!(
            |_| std::future::ready(Err(PlatformWalletError::InvalidIdentityData(
                "no notes".to_string()
            )))
        );
        assert!(!pending(&failed), "{failed:?}");

        // Paid for, but the entry is not in the store yet: pending.
        let kept = Arc::new(Mutex::new(Vec::new()));
        let first = send!(keep_entry(&kept));
        assert!(pending(&first), "{first:?}");
        let entry = kept.lock().expect("kept").remove(0);

        // A second request is refused before anything is paid.
        let paid_again = Arc::new(Mutex::new(Vec::new()));
        let second = send!(keep_entry(&paid_again));
        assert!(pending(&second), "{second:?}");
        assert!(
            paid_again.lock().expect("paid").is_empty(),
            "nothing is paid twice"
        );

        // The store reads the entry back: settled and recorded.
        publish(&alice_wallet, 0, vec![entry.to_vec()]).await;
        view.sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("Alice reads the store");
        managed(&alice_wallet, ALICE, |managed| {
            assert!(!managed.dashpay().private_reply_pending(&bob.id()));
            assert!(managed.dashpay().outgoing_request(&bob.id()).is_some());
        })
        .await;
    }

    /// A payment whose execution is unconfirmed keeps the request pending.
    #[tokio::test]
    async fn should_keep_an_unconfirmed_private_request_pending() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let result = alice_wallet
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                |_| {
                    std::future::ready(Err(PlatformWalletError::ShieldedSpendUnconfirmed {
                        operation: "private document create",
                        reason: "the proof fetch timed out".to_string(),
                    }))
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
        ));
        managed(&alice_wallet, ALICE, |managed| {
            assert!(managed.dashpay().private_reply_pending(&bob.id()));
        })
        .await;
    }

    /// A cursor rewound under a pass is not advanced again that pass, so the
    /// next reads from the rewind.
    #[tokio::test]
    async fn should_keep_a_cursor_rewound_under_a_pass() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[], vec![]).await;
        let iw = alice_wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&alice_wallet.wallet_id())
            .expect("wallet info")
            .identity_manager
            .managed_identity_mut(&alice.id())
            .expect("Alice");
        let mut scanned = ScannedIdentity {
            identity_id: alice.id(),
            identity_index: 0,
            position: 0,
            failed: false,
            rewound: false,
        };
        scanned.advance(managed, 100);
        assert_eq!(managed.dashpay().private_contact_requests_position(), 100);
        // Unignoring a sender rewinds the cursor while a page is read.
        let carol = Identifier::from([0xC1; 32]);
        managed.ignore_sender(&carol);
        managed.unignore_sender(&carol);
        assert_eq!(managed.dashpay().private_contact_requests_position(), 0);
        scanned.advance(managed, 200);
        scanned.advance(managed, 300);
        assert_eq!(
            managed.dashpay().private_contact_requests_position(),
            0,
            "the rewind stands for the rest of the pass"
        );
        assert!(scanned.rewound);
    }

    /// The record of a private request rebuilds byte for byte from the same
    /// entry, and another entry's record encrypts under other IVs.
    #[test]
    fn should_record_a_private_request_the_same_way_every_time() {
        let request = PrivateContactRequest {
            account_reference: 7,
            extended_public_key: [0x33; COMPACT_XPUB_LEN],
            account_label: Some("label".to_string()),
            auto_accept_proof: None,
            shielded_address: None,
        };
        let record = |commitment: [u8; 32]| {
            private_request_record(
                Identifier::from(ALICE),
                Identifier::from(BOB),
                1,
                2,
                &request,
                &commitment,
                &[9; 32],
                BLOCK_TIME_MS,
            )
        };
        assert_eq!(record([1; 32]), record([1; 32]));
        assert!(record([1; 32]).came_privately());
        assert_ne!(
            record([1; 32]).encrypted_public_key,
            record([2; 32]).encrypted_public_key,
            "another entry encrypts under other IVs"
        );
        assert_eq!(
            decrypt_extended_public_key(&[9; 32], &record([1; 32]).encrypted_public_key)
                .expect("the xpub opens"),
            vec![0x33; COMPACT_XPUB_LEN]
        );
    }
}
