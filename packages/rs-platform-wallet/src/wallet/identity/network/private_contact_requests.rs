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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::dashpay::{
    private_contact_request_identities, private_contact_request_signature,
    private_contact_request_signing_data, private_contact_request_signing_keys,
    private_contact_requests_from_scan, recipient_key_purpose_is_valid, PrivateContactRequest,
    ProvedIdentity, RejectedPrivateContactRequest, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
};
use dash_sdk::platform::private_document_entries::{PrivateDocumentEntries, PrivateDocumentEntry};
use dash_sdk::platform::private_document_scan::{
    PrivateDocumentScanKeys, MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::decode_private_document_payload;
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
use futures::StreamExt;
use key_wallet::bip32::{DerivationPath, KeyDerivationType};
use key_wallet_manager::WalletManager;
use platform_encryption::{encrypt_account_label, encrypt_extended_public_key, COMPACT_XPUB_LEN};
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use super::contact_requests::{
    ingest_received_requests, ingest_sent_requests, newest_received_per_sender,
    newest_sent_per_recipient, predates, records_or_supersedes, tear_down_stale_external_accounts,
    AutoAcceptProofSource, ContactCryptoProvider, PreparedContactRequest, ReplyOutcome,
};
use super::signing_key::first_available;
use super::{dashpay_contract, DashPayView, IdentityWallet};
use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::{ContactChangeSet, PrivateContactRequestsPosition};
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::dashpay::contact_request::ContactRequest;
use crate::wallet::identity::types::dashpay::established_contact::EstablishedContact;
use crate::wallet::identity::ManagedIdentity;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

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

/// What a contact request from one of the wallet's identities waits for, as
/// the wallet decides it ([`DashPayView::private_contact_readiness`]), so a
/// host shows it rather than inferring it from the refusals it got.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrivateContactReadiness {
    /// Whether the reservations of the wallet's private payments from an
    /// earlier process are restored for the identity. While false, its public
    /// contact requests, sent or accepted, are refused with
    /// [`PlatformWalletError::PrivateContactPaymentsNotRestored`]: the wallet
    /// bound a shielded account before it was loaded, or the identity was
    /// loaded marking a private payment that may be unresolved. Configuring
    /// the shielded store, or binding the wallet's shielded account, restores
    /// them; no private sync does. A build without the `shielded` feature
    /// never does.
    pub private_payments_restored: bool,
    /// Whether a private sync read DashPay's private contact request store to
    /// its end for the identity since load. Until then its first send or
    /// accept reads it first, and is refused with
    /// [`PlatformWalletError::PrivateContactRequestPending`] while that read
    /// cannot finish; a private sync sets it.
    pub private_store_read: bool,
    /// Whether a private contact request the identity paid for may still be
    /// unresolved: the marker its position record stores
    /// (`PrivateContactRequestsPosition::private_payments_unresolved`), set
    /// while it holds a private request it is paying for or has not read back.
    /// A contact's own pending request is
    /// [`DashPayState::private_reply_pending`](crate::wallet::identity::DashPayState::private_reply_pending).
    pub unresolved_private_payment: bool,
}

/// What a pass over DashPay's private contact request store did.
#[derive(Clone, Debug, Default)]
pub struct PrivateContactSyncReport {
    /// The store pages read.
    pub pages_read: usize,
    /// The requests this pass recorded, received and sent, as the wallet
    /// records them: new ones and rotations. Not one already tracked, an
    /// ignored sender's, a replay older than the tracked one, or one whose
    /// store failed.
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

/// How many pages of the store a send reads back after paying: one window of
/// pages fetched at once, enough for an identity that synced recently, whose
/// new entry is in the last page or two. A request not read back within them
/// stays pending for a later sync.
const POST_SEND_SYNC_PAGES: usize = MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES;

/// How many pages of the store each private sync reads that a private send
/// runs, before it seals anything, when the store was not yet read to its end
/// since load: sixteen windows of pages fetched at once. With the read
/// position kept across restarts, the read is from there to the end.
const PRIVATE_STORE_READ_PAGES: usize = 16 * MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES;

/// How many fetched pages a sync scans at once, one key derivation for them
/// all: at most the pages in flight.
const PRIVATE_SCAN_BATCH_PAGES: usize = MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES;

/// How many such syncs a private send runs before it refuses with
/// [`PlatformWalletError::PrivateContactRequestPending`]: a later send goes on
/// from where they stopped.
const PRIVATE_STORE_READ_PASSES: usize = 4;

/// One of the wallet's identities, as a pass reads the store for it.
struct ScannedIdentity {
    identity_id: Identifier,
    identity_index: u32,
    /// Its cursor when the pass last advanced it; a change under the pass is a
    /// rewind, which the pass leaves alone.
    position: u64,
    /// Its ingest failed this pass: it is not advanced again until the next.
    failed: bool,
    /// Its cursor was rewound under the pass (unignoring a sender, or its
    /// scanning keys changing): it is not advanced again this pass, so the
    /// next reads from the rewound position.
    rewound: bool,
    /// The fingerprint of the keys the pass scans with for it
    /// (`ManagedIdentity::private_scan_key_fingerprint`): progress made
    /// with them holds only while they are still the identity's.
    key_fingerprint: [u8; 32],
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
        // Keys added under the pass opened nothing it read.
        if !self.keys_current(managed) {
            self.rewound = true;
            return;
        }
        if managed.dashpay().private_contact_requests_position() != self.position {
            self.rewound = true;
            return;
        }
        managed.advance_private_contact_requests_position(self.position, next_position);
        self.position = managed.dashpay().private_contact_requests_position();
    }

    /// Whether the keys the pass scans with are still the identity's, and
    /// the ones its read position holds for.
    fn keys_current(&self, managed: &ManagedIdentity) -> bool {
        managed.private_scan_key_fingerprint() == self.key_fingerprint
            && managed
                .private_contact_requests_position_record()
                .key_fingerprint
                == self.key_fingerprint
    }
}

/// Handed to a private request's `pay`, which marks it right before anything
/// it does may reach the network. Until it is marked, a send dropped (a
/// cancelled accept, a timeout) releases its reservation, as nothing was paid
/// and no sync could settle it; once marked, the reservation stays for a
/// sync to settle. A `pay` that awaits anything first, such as resolving its
/// payment source, a spend key or a proof, marks it only after those.
///
/// It also names the request it pays for ([`Self::account_reference`]), so
/// the payment's record can be saved with it and a restart restores the
/// reservation without opening the entry.
#[derive(Clone, Debug, Default)]
pub struct PaymentStarted {
    marked: Arc<AtomicBool>,
    account_reference: Option<u32>,
}

impl PaymentStarted {
    /// The payment may reach the network from now on.
    pub fn mark(&self) {
        self.marked.store(true, Ordering::Release);
    }

    /// Whether [`Self::mark`] was called.
    pub fn is_marked(&self) -> bool {
        self.marked.load(Ordering::Acquire)
    }

    /// The account reference of the private request this payment is for,
    /// when the send handed it over.
    pub fn account_reference(&self) -> Option<u32> {
        self.account_reference
    }

    /// The same token, naming the request with `account_reference`.
    fn for_request(&self, account_reference: u32) -> Self {
        Self {
            marked: Arc::clone(&self.marked),
            account_reference: Some(account_reference),
        }
    }
}

/// The reservation one send attempt holds on the private request to a
/// recipient that may be in flight. Dropped before its payment is marked
/// started ([`PaymentStarted`]), it releases the reservation: a send
/// cancelled before it pays leaves nothing pending. Once the payment may
/// have started, or the send kept it, the reservation stays for the sync.
struct PrivateReplyReservation {
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: WalletPersister,
    wallet_id: WalletId,
    sender: Identifier,
    recipient: Identifier,
    attempt: u64,
    payment_started: PaymentStarted,
    kept: bool,
}

impl PrivateReplyReservation {
    /// Leave the reservation for the private sync to settle.
    fn keep(&mut self) {
        self.kept = true;
    }

    /// Release the reservation now, kept or not. Dropped while it waits for
    /// the wallet lock, the guard releases it.
    async fn release(mut self) {
        self.kept = false;
        self.payment_started = PaymentStarted::default();
        let mut wm = self.wallet_manager.write().await;
        release_reservation(
            &mut wm,
            &self.wallet_id,
            &self.sender,
            &self.recipient,
            self.attempt,
        );
        store_private_payment_marker(&self.persister, &wm, &self.wallet_id, &self.sender);
        self.kept = true;
    }
}

impl Drop for PrivateReplyReservation {
    fn drop(&mut self) {
        if self.kept || self.payment_started.is_marked() {
            return;
        }
        let (wallet_id, sender, recipient, attempt) =
            (self.wallet_id, self.sender, self.recipient, self.attempt);
        let persister = self.persister.clone();
        apply_detached(
            &self.wallet_manager,
            "a cancelled private contact request's reservation",
            move |wm| {
                release_reservation(wm, &wallet_id, &sender, &recipient, attempt);
                store_private_payment_marker(&persister, wm, &wallet_id, &sender);
            },
        );
    }
}

/// Run `apply` on the wallet manager, for a guard released on drop, which
/// cannot wait for the lock: now when the lock is free, otherwise on a
/// spawned task. Outside a runtime it is logged, and `what` stays until
/// restart.
pub(super) fn apply_detached(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    what: &'static str,
    apply: impl FnOnce(&mut WalletManager<PlatformWalletInfo>) + Send + 'static,
) {
    if let Ok(mut wm) = wallet_manager.try_write() {
        apply(&mut wm);
        return;
    }
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        tracing::warn!(
            what,
            "released on drop outside a runtime; it stays until restart"
        );
        return;
    };
    let wallet_manager = Arc::clone(wallet_manager);
    runtime.spawn(async move {
        let mut wm = wallet_manager.write().await;
        apply(&mut wm);
    });
}

/// Release the reservation `attempt` holds on `sender`'s private request to
/// `recipient`.
fn release_reservation(
    wm: &mut WalletManager<PlatformWalletInfo>,
    wallet_id: &WalletId,
    sender: &Identifier,
    recipient: &Identifier,
    attempt: u64,
) {
    if let Some(managed) = wm
        .get_wallet_info_mut(wallet_id)
        .and_then(|info| info.identity_manager.managed_identity_mut(sender))
    {
        managed.release_private_reply(recipient, attempt);
    }
}

/// Store `owner`'s private read position record again, for a backend that
/// keeps it, as whether a private payment of `owner` may be unresolved
/// (`PrivateContactRequestsPosition::private_payments_unresolved`) may have
/// changed: a reservation was released or settled. Best effort: a failed
/// store leaves the marker set, which holds the identity after a restart
/// until a restoration clears it, never the other way round.
fn store_private_payment_marker(
    persister: &WalletPersister,
    wm: &WalletManager<PlatformWalletInfo>,
    wallet_id: &WalletId,
    owner: &Identifier,
) {
    if !persister.persists_private_contact_scan_state() {
        return;
    }
    if let Some(managed) = wm
        .get_wallet_info(wallet_id)
        .and_then(|info| info.identity_manager.managed_identity(owner))
    {
        store_private_position(
            persister,
            *owner,
            managed.private_contact_requests_position_record(),
        );
    }
}

/// The changeset recording, for each contact one of `addresses` came from, the
/// shielded address `owner` keeps for it now, or its removal.
fn shielded_address_changeset(
    managed: &ManagedIdentity,
    owner: Identifier,
    addresses: &[(ContactRequest, Option<[u8; 43]>)],
) -> ContactChangeSet {
    let mut changeset = ContactChangeSet::default();
    for (record, _) in addresses {
        let key = (owner, record.sender_id);
        match managed
            .dashpay()
            .private_shielded_address(&record.sender_id)
        {
            Some(address) => {
                changeset.private_shielded_addresses.insert(key, address);
            }
            None => {
                changeset.removed_private_shielded_addresses.insert(key);
            }
        }
    }
    changeset
}

/// Store `owner`'s private read position. A failed store leaves the cursor
/// advanced in memory: the next launch reads from the last position stored,
/// and ingest is a fixpoint.
fn store_private_position(
    persister: &WalletPersister,
    owner: Identifier,
    position: PrivateContactRequestsPosition,
) {
    let mut changeset = ContactChangeSet::default();
    changeset
        .private_contact_requests_positions
        .insert(owner, position);
    if let Err(error) = persister.store(changeset.into()) {
        tracing::warn!(
            owner = %owner, position = position.position, %error,
            "private read position not stored; the next launch reads from the last one stored"
        );
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
    /// keeps every entry appended and a second payment could execute too. A
    /// public request to the recipient is refused the same way.
    ///
    /// The proof `pay` waits for shows the notes spent, not the entry
    /// appended, so a paid request is recorded only once read back: a private
    /// sync of a few pages follows the payment, and records the request from
    /// the store as it records any request the identity sent. If it is not
    /// read back yet, the send returns `PrivateContactRequestPending` and the
    /// request stays pending until a later sync reads it. An unconfirmed
    /// payment stays pending the same way; one that definitely failed
    /// releases it, and so does a send dropped before its payment started:
    /// `pay` gets a [`PaymentStarted`] it marks right before anything it does
    /// may reach the network, after any resolving it awaits first. `pay` itself
    /// may return `PrivateContactRequestPending` to keep the request pending
    /// unpaid. The pending request is kept in memory only; after a restart,
    /// [`Self::restore_pending_private_contact_requests`] restores those whose
    /// payment is unconfirmed.
    ///
    /// A request paid for and confirmed but not read back leaves nothing
    /// that survives a restart. So the first private send or accept from an
    /// identity since load reads the store to its end first, which records
    /// such a request; while that read cannot finish, the send is refused
    /// with `PrivateContactRequestPending`, nothing sealed or paid.
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
        F: FnOnce(BinaryData, PaymentStarted) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        self.send_private_request(
            sender_identity_id,
            recipient_identity_id,
            account_label,
            auto_accept_proof,
            shielded_address,
            signer,
            crypto,
            false,
            pay,
        )
        .await?
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(
                "no private contact request was sent: only a reply adopts one already sent"
                    .to_string(),
            )
        })
    }

    /// Reply privately to `request`, paid by `pay`: `AlreadyReplied`, with
    /// nothing sealed or paid, when a request to its sender is tracked by the
    /// time the pair is reserved (another accept's reply, recorded in
    /// between), which the accept then adopts.
    pub(super) async fn send_private_reply<S, C, F, Fut>(
        &self,
        request: &ContactRequest,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        pay: F,
    ) -> Result<ReplyOutcome, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData, PaymentStarted) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        Ok(
            match self
                .send_private_request(
                    &request.recipient_id,
                    &request.sender_id,
                    None,
                    AutoAcceptProofSource::None,
                    shielded_address,
                    signer,
                    crypto,
                    true,
                    pay,
                )
                .await?
            {
                Some(_) => ReplyOutcome::Sent,
                None => ReplyOutcome::AlreadyReplied,
            },
        )
    }

    /// [`Self::send_private_contact_request`], for a reply when
    /// `adopt_reply`: then a request to the recipient tracked once the store
    /// was read and the pair is reserved, in the same lock, is adopted
    /// rather than rotated past, and `None` comes back with nothing paid.
    #[allow(clippy::too_many_arguments)]
    async fn send_private_request<S, C, F, Fut>(
        &self,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        account_label: Option<String>,
        auto_accept_proof: AutoAcceptProofSource,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        adopt_reply: bool,
        pay: F,
    ) -> Result<Option<ContactRequest>, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData, PaymentStarted) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        self.read_private_store_before_sending(sender_identity_id, recipient_identity_id, crypto)
            .await?;
        self.release_rejected_private_requests().await?;
        let reserved = self
            .with_managed(sender_identity_id, |managed| {
                if adopt_reply
                    && managed
                        .dashpay()
                        .outgoing_request(recipient_identity_id)
                        .is_some()
                {
                    return None;
                }
                Some(managed.reserve_private_reply(*recipient_identity_id))
            })
            .await?;
        let Some(reserved) = reserved else {
            return Ok(None);
        };
        let attempt = reserved.ok_or(PlatformWalletError::PrivateContactRequestPending(
            *recipient_identity_id,
        ))?;
        let mut reservation = PrivateReplyReservation {
            wallet_manager: Arc::clone(&self.wallet_manager),
            persister: self.persister.clone(),
            wallet_id: self.wallet_id,
            sender: *sender_identity_id,
            recipient: *recipient_identity_id,
            attempt,
            payment_started: PaymentStarted::default(),
            kept: false,
        };
        let sent = self
            .send_reserved_private_contact_request(
                sender_identity_id,
                recipient_identity_id,
                account_label,
                auto_accept_proof,
                shielded_address,
                signer,
                crypto,
                &mut reservation,
                pay,
            )
            .await;
        // A payment that may have executed keeps the request pending.
        let keep_pending = matches!(
            &sent,
            Err(PlatformWalletError::PrivateContactRequestPending(_)
                | PlatformWalletError::ShieldedSpendUnconfirmed { .. })
        );
        if keep_pending {
            reservation.keep();
        } else {
            reservation.release().await;
        }
        sent.map(Some)
    }

    /// Make sure DashPay's private contact request store was read to its end
    /// for `identity_id` since load, before a private request from it to
    /// `recipient` is sealed or paid for: a request paid for in an earlier
    /// process may be in the store unread, and the reservation that kept a
    /// second one from being paid did not survive the restart. Until it was,
    /// this runs private syncs of [`PRIVATE_STORE_READ_PAGES`] pages, at most
    /// [`PRIVATE_STORE_READ_PASSES`] of them, and refuses with
    /// [`PlatformWalletError::PrivateContactRequestPending`] when they stop
    /// advancing or run out first. A sync error comes back as it is. Once
    /// read, it reads nothing. An identity the private sync does not read,
    /// with no HD index or no key an entry can be sealed to or from, sends no
    /// private request, so it is not held.
    async fn read_private_store_before_sending<C>(
        &self,
        identity_id: &Identifier,
        recipient: &Identifier,
        crypto: &C,
    ) -> Result<(), PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        // Keys added since the store was read clear the flag first.
        self.reconcile_private_scan_keys().await?;
        if self.private_store_read(identity_id).await? {
            return Ok(());
        }
        let (identities, _) = self.private_scan_identities().await?;
        if !identities
            .iter()
            .any(|identity| identity.identity_id == *identity_id)
        {
            return Ok(());
        }
        let mut position = identities
            .iter()
            .map(|identity| identity.position)
            .min()
            .unwrap_or_default();
        for _ in 0..PRIVATE_STORE_READ_PASSES {
            let report = self
                .sync_private_contact_requests(crypto, PRIVATE_STORE_READ_PAGES)
                .await?;
            if self.private_store_read(identity_id).await? {
                return Ok(());
            }
            if report.next_position <= position {
                break;
            }
            position = report.next_position;
        }
        Err(PlatformWalletError::PrivateContactRequestPending(
            *recipient,
        ))
    }

    /// [`Self::read_private_store_before_sending`], before a public request
    /// or reply from `identity_id` to `recipient` is chosen or sent, so a
    /// private request paid for in an earlier process holds the pair here
    /// too. A network that serves no private documents yet holds none, so
    /// the SDK's refusal of the read reads as nothing to read, but only once
    /// a verified answer shows the network below the protocol version that
    /// serves them (`Sdk::network_proven_below`): a refusal alone does not
    /// say whether the network or this SDK is below it, and an SDK pinned
    /// below it refuses a store the network has. Without that evidence the
    /// refusal comes back as it is, and nothing is sent.
    ///
    /// Refused first, with nothing read, while the wallet's unconfirmed
    /// private payments from an earlier process may be unrestored
    /// ([`PlatformWalletError::PrivateContactPaymentsNotRestored`]): such a
    /// payment's entry is not in the store until it executes, so no read
    /// shows the pair it holds; only the shielded store's record of it does.
    /// That is so for a wallet a shielded build loads with a persisted
    /// viewing key, and, in every build, for an identity loaded with its
    /// position record marking a private payment that may be unresolved
    /// (`PrivateContactRequestsPosition::private_payments_unresolved`).
    pub(super) async fn read_private_store_before_deciding<C>(
        &self,
        identity_id: &Identifier,
        recipient: &Identifier,
        crypto: &C,
    ) -> Result<(), PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        if self.persister.private_payments_unrestored()
            || self.private_payments_unrestored(identity_id).await?
        {
            return Err(PlatformWalletError::PrivateContactPaymentsNotRestored);
        }
        match self
            .read_private_store_before_sending(identity_id, recipient, crypto)
            .await
        {
            Err(PlatformWalletError::Sdk(
                refusal @ dash_sdk::Error::ProtocolVersionNotActive { required, .. },
            )) => match self.sdk.network_proven_below(required).await {
                Ok(true) => Ok(()),
                Ok(false) => Err(PlatformWalletError::Sdk(refusal)),
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "the network's protocol version could not be verified; the private \
                         store is not taken as empty"
                    );
                    Err(PlatformWalletError::Sdk(refusal))
                }
            },
            read => read,
        }
    }

    /// Rewind the private read position and the read flag of each of the
    /// wallet's identities whose scanning keys changed since they were set
    /// (`ManagedIdentity::reconcile_private_scan_keys`), storing the rewind
    /// for a backend that keeps the position. A refresh that folds in a key
    /// another device added is caught here, before the next scan or send.
    pub(crate) async fn reconcile_private_scan_keys(&self) -> Result<(), PlatformWalletError> {
        let persists_scan_state = self.persister.persists_private_contact_scan_state();
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let identity_ids: Vec<Identifier> = info
            .identity_manager
            .all_identities()
            .iter()
            .map(|identity| identity.id())
            .collect();
        for identity_id in identity_ids {
            let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) else {
                continue;
            };
            if let Some(position) = managed.reconcile_private_scan_keys() {
                if persists_scan_state {
                    store_private_position(&self.persister, identity_id, position);
                }
            }
        }
        Ok(())
    }

    /// Store `owner`'s position record, marking it as holding a private payment
    /// that may be unresolved, before anything is paid, for a backend that
    /// keeps it. A failed store refuses the send, with nothing paid: a payment
    /// a restart could not see would leave the pair unheld.
    fn store_private_payment_marker_before_paying(
        &self,
        owner: &Identifier,
        marker: PrivateContactRequestsPosition,
    ) -> Result<(), PlatformWalletError> {
        if !self.persister.persists_private_contact_scan_state() {
            return Ok(());
        }
        let mut changeset = ContactChangeSet::default();
        changeset
            .private_contact_requests_positions
            .insert(*owner, marker);
        self.persister.store(changeset.into()).map_err(|error| {
            PlatformWalletError::Persistence(format!(
                "the private contact request's unresolved payment was not stored, so it is not \
                 paid for: {error}"
            ))
        })
    }

    /// Whether `identity_id` was loaded with a private contact request payment
    /// that may be unresolved and no restoration has restored it since
    /// ([`DashPayState::private_payments_unrestored`]).
    ///
    /// [`DashPayState::private_payments_unrestored`]: crate::wallet::identity::DashPayState::private_payments_unrestored
    pub(crate) async fn private_payments_unrestored(
        &self,
        identity_id: &Identifier,
    ) -> Result<bool, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let managed = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .identity_manager
            .managed_identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(managed.dashpay().private_payments_unrestored())
    }

    /// What a contact request from `identity_id` waits for, as the wallet
    /// decides it ([`PrivateContactReadiness`]). A pure read.
    pub async fn private_contact_readiness(
        &self,
        identity_id: &Identifier,
    ) -> Result<PrivateContactReadiness, PlatformWalletError> {
        let wallet_held = self.persister.private_payments_unrestored();
        let wm = self.wallet_manager.read().await;
        let dashpay = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .identity_manager
            .managed_identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
            .dashpay();
        Ok(PrivateContactReadiness {
            private_payments_restored: !wallet_held && !dashpay.private_payments_unrestored(),
            private_store_read: dashpay.private_contact_requests_read(),
            unresolved_private_payment: dashpay.private_payments_unresolved(),
        })
    }

    /// Whether any of the wallet's identities was loaded with a private
    /// contact request payment that may be unresolved and no restoration has
    /// restored it since.
    pub(crate) async fn any_private_payments_unrestored(&self) -> bool {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id).is_some_and(|info| {
            info.identity_manager
                .all_identities()
                .iter()
                .filter_map(|identity| info.identity_manager.managed_identity(&identity.id()))
                .any(|managed| managed.dashpay().private_payments_unrestored())
        })
    }

    /// The reservations of the wallet's private payments from an earlier
    /// process are restored from the shielded store: release the hold a load
    /// set on each identity, and store the record each one changed for a
    /// backend that keeps it.
    #[cfg(feature = "shielded")]
    pub(crate) async fn note_private_payments_restored(&self) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let identity_ids: Vec<Identifier> = info
            .identity_manager
            .all_identities()
            .iter()
            .map(|identity| identity.id())
            .collect();
        for identity_id in identity_ids {
            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                if managed.note_private_payments_restored()
                    && self.persister.persists_private_contact_scan_state()
                {
                    store_private_position(
                        &self.persister,
                        identity_id,
                        managed.private_contact_requests_position_record(),
                    );
                }
            }
        }
        Ok(())
    }

    /// Release the pending private requests whose payment the shielded
    /// coordinator settled as never executing, for an entry a definitive
    /// rejection or a pruned anchor left unappendable
    /// (`WalletPersister::note_rejected_private_entries`): run before a
    /// private request is reserved, before a public one claims its pair,
    /// and before a private payment, so a retry needs no restart. A
    /// reservation is released only on that word, never for a missing
    /// unconfirmed spend: a request paid and not yet read back has none.
    pub(crate) async fn release_rejected_private_requests(
        &self,
    ) -> Result<(), PlatformWalletError> {
        if !self.persister.has_rejected_private_entries() {
            return Ok(());
        }
        // Taken only once the wallet is locked and found, and applied with
        // nothing awaited in between: a call dropped while it waits for the
        // lock leaves the notices for the next one.
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let rejected = self.persister.take_rejected_private_entries();
        let identity_ids: Vec<Identifier> = info
            .identity_manager
            .all_identities()
            .iter()
            .map(|identity| identity.id())
            .collect();
        for identity_id in identity_ids {
            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                let unresolved = managed.dashpay().private_payments_unresolved();
                managed.release_rejected_private_replies(&rejected);
                if self.persister.persists_private_contact_scan_state()
                    && managed.dashpay().private_payments_unresolved() != unresolved
                {
                    store_private_position(
                        &self.persister,
                        identity_id,
                        managed.private_contact_requests_position_record(),
                    );
                }
            }
        }
        Ok(())
    }

    /// Whether a private sync read DashPay's private contact request store
    /// to its end for `identity_id` since load.
    async fn private_store_read(
        &self,
        identity_id: &Identifier,
    ) -> Result<bool, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let managed = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .identity_manager
            .managed_identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(managed.dashpay().private_contact_requests_read())
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
    /// to the recipient is reserved by `reservation`, which it keeps right
    /// before paying.
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
        reservation: &mut PrivateReplyReservation,
        pay: F,
    ) -> Result<ContactRequest, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        F: FnOnce(BinaryData, PaymentStarted) -> Fut + Send,
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

        // Wiped on drop, so a send dropped while the seal is awaited wipes it too.
        let mut esk_seed = Zeroizing::new([0u8; 32]);
        StdRng::try_from_os_rng()
            .map_err(|error| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "the OS RNG failed while sealing a private contact request: {error}"
                ))
            })?
            .fill_bytes(&mut *esk_seed);
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
        drop(esk_seed);
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
        let marker = self
            .with_managed(sender_identity_id, |managed| {
                managed.note_private_reply_commitment(
                    *recipient_identity_id,
                    reservation.attempt,
                    commitment,
                );
                managed.private_contact_requests_position_record()
            })
            .await?;
        // Before anything is paid, the identity is marked as holding a private
        // payment that may be unresolved: a process reopening the wallet before
        // the payment's reservation is restored, or in a build that cannot
        // restore it, holds its public requests until then.
        self.store_private_payment_marker_before_paying(sender_identity_id, marker)?;
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

        // `pay` marks the payment started right before it may reach the
        // network: from then on the reservation is the sync's to settle,
        // whatever happens to this send.
        let paid = pay(
            entry,
            reservation.payment_started.for_request(account_reference),
        )
        .await;
        reservation.payment_started.mark();
        paid?;

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
    ///
    /// As for the send, the store is read to its end first when it was not
    /// since load, so a reply paid for in an earlier process is adopted
    /// rather than paid for again; while that read cannot finish, the accept
    /// is refused with [`PlatformWalletError::PrivateContactRequestPending`].
    /// A reply whose payment is unconfirmed since before a restart holds the
    /// pair from the moment the wallet binds to its shielded coordinator.
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
        F: FnOnce(BinaryData, PaymentStarted) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        self.read_private_store_before_sending(&request.recipient_id, &request.sender_id, crypto)
            .await?;
        self.accept_contact_request_replying_with(request, crypto, || {
            self.send_private_reply(request, shielded_address, signer, crypto, pay)
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
    ///
    /// The store is read to its end first if it was not since load, before
    /// the reply is chosen, whatever the request: a reply paid for in an
    /// earlier process, to a public request too, is then adopted, with no
    /// payment and no public reply. While that read cannot finish, the accept
    /// is refused with [`PlatformWalletError::PrivateContactRequestPending`].
    /// On a network that serves no private documents yet, nothing is read.
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
        F: FnOnce(BinaryData, PaymentStarted) -> Fut + Send,
        Fut: Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send,
    {
        self.read_private_store_before_deciding(&request.recipient_id, &request.sender_id, crypto)
            .await?;
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
                key_fingerprint: managed.private_scan_key_fingerprint(),
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
    ///
    /// Refused with [`PlatformWalletError::SeedMismatch`], before any page
    /// is read and with no position moved, when `crypto` does not resolve
    /// this wallet's seed ([`IdentityWallet::verify_seed_binds`]): another
    /// seed derives valid but unrelated keys, which open nothing, so its pass
    /// would record as read pages holding requests it never saw, and nothing
    /// rewinds them once the provider is fixed.
    pub async fn sync_private_contact_requests<C>(
        &self,
        crypto: &C,
        max_pages: usize,
    ) -> Result<PrivateContactSyncReport, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        self.reconcile_private_scan_keys().await?;
        let (mut identities, paths) = self.private_scan_identities().await?;
        let mut report = PrivateContactSyncReport::default();
        let Some(start) = identities.iter().map(|identity| identity.position).min() else {
            report.complete = true;
            return Ok(report);
        };
        // Before anything is read or recorded: once a pass, as the keys it
        // scans with all come from the same provider.
        self.verify_seed_binds(crypto).await?;
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
        report.next_position = start;
        let persists_scan_state = self.persister.persists_private_contact_scan_state();

        // The pages are fetched many at once while earlier ones are processed,
        // and scanned in batches of those ready, one key derivation a batch.
        let mut batches = std::pin::pin!(self
            .sdk
            .private_document_pages(
                contract.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                start,
                MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
            )
            .take(max_pages)
            .ready_chunks(PRIVATE_SCAN_BATCH_PAGES));
        while let Some(batch) = batches.next().await {
            // The pages before a failed fetch are processed, then the error
            // ends the pass.
            let mut fetched = Vec::with_capacity(batch.len());
            let mut failure = None;
            for page in batch {
                match page {
                    Ok(page) => fetched.push(page),
                    Err(error) => {
                        failure = Some(error);
                        break;
                    }
                }
            }
            let scans = if fetched.is_empty() {
                Vec::new()
            } else {
                let pages: Vec<(u64, &PrivateDocumentEntries)> = fetched
                    .iter()
                    .map(|page| (page.start_position, &page.entries))
                    .collect();
                crypto
                    .scan_private_document_pages(
                        &paths,
                        contract.id(),
                        PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                        &pages,
                    )
                    .await?
            };
            if scans.len() != fetched.len() {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "the crypto provider sorted {} pages into {} scans",
                    fetched.len(),
                    scans.len()
                )));
            }
            let mut reached_end = false;
            for scan in scans {
                report.pages_read += 1;
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
                let mut records: Vec<(Identifier, ContactRequest, bool, Option<[u8; 43]>)> =
                    Vec::new();
                // The entry commitment of each sent request's record: a pending
                // request is settled by it once that record is reconciled.
                let mut sent_commitments: Vec<([u8; 32], ContactRequest)> = Vec::new();
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
                    sent_commitments.push((sent.commitment, record.clone()));
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
                    let (wallet, info) = wm
                        .get_wallet_mut_and_info_mut(&self.wallet_id)
                        .ok_or_else(|| {
                            PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id))
                        })?;
                    for identity in identities.iter_mut() {
                        let identity_id = identity.identity_id;
                        let Some(managed) =
                            info.identity_manager.managed_identity_mut(&identity_id)
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
                            &mut ingested,
                        );
                        report.requests.extend(ingested);
                        // The address of the request tracked from each contact,
                        // once ingest decided which that is.
                        for (record, shielded_address) in &addresses {
                            managed.note_private_shielded_address(record, *shielded_address);
                        }
                        // The addresses are stored before the read position
                        // that covers the entries they came from.
                        let mut stored = received_ok && sent_ok;
                        if stored && persists_scan_state && !addresses.is_empty() {
                            let addresses_changeset =
                                shielded_address_changeset(managed, identity_id, &addresses);
                            if let Err(error) = self.persister.store(addresses_changeset.into()) {
                                tracing::error!(
                                    owner = %identity_id, %error,
                                    "private contacts' shielded addresses not stored; leaving \
                                     the private cursor for retry"
                                );
                                stored = false;
                            }
                        }
                        if stored {
                            let unresolved = managed.dashpay().private_payments_unresolved();
                            // A pending private request is settled once its
                            // entry's record is reconciled: the request tracked
                            // to the recipient is that record, carries its
                            // account reference (another copy of it), or is
                            // newer. An entry whose record was not built, its
                            // recipient unreadable, settles nothing.
                            for (commitment, record) in sent_commitments
                                .iter()
                                .filter(|(_, record)| record.sender_id == identity_id)
                            {
                                let reconciled = managed
                                    .dashpay()
                                    .outgoing_request(&record.recipient_id)
                                    .is_some_and(|tracked| {
                                        tracked.account_reference == record.account_reference
                                            || predates(record, tracked)
                                    });
                                if reconciled {
                                    managed.settle_private_reply(&record.recipient_id, commitment);
                                }
                            }
                            let before = managed.dashpay().private_contact_requests_position();
                            identity.advance(managed, read.next_position);
                            let after = managed.dashpay().private_contact_requests_position();
                            // Stored once it moved, or once the last private
                            // payment it held settled.
                            if persists_scan_state
                                && (after != before
                                    || managed.dashpay().private_payments_unresolved()
                                        != unresolved)
                            {
                                store_private_position(
                                    &self.persister,
                                    identity_id,
                                    managed.private_contact_requests_position_record(),
                                );
                            }
                        } else if !identity.failed {
                            identity.failed = true;
                            report.unpersisted_identities.push(identity_id);
                        }
                        // Read to the store's end with nothing held back this
                        // pass: every entry was ingested for this identity.
                        // Not once its cursor was rewound under the pass (a
                        // sender unignored): the entries before it were read
                        // with the sender ignored, and the next pass reads
                        // them again.
                        if read.is_complete()
                            && !identity.failed
                            && !identity.rewound
                            && identity.keys_current(managed)
                        {
                            managed.note_private_contact_requests_read();
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
                if read.is_complete() {
                    // A pass whose ingest did not reach disk for an identity is
                    // not complete: that identity reads its entries again.
                    report.complete = report.unpersisted_identities.is_empty();
                    reached_end = true;
                    break;
                }
            }
            if let Some(error) = failure {
                return Err(error.into());
            }
            if reached_end {
                break;
            }
        }
        Ok(report)
    }

    /// Restore, after a restart, the pending private requests whose payment
    /// is unconfirmed: `entries` are those of the signed transitions the
    /// shielded store keeps for unconfirmed spends to DashPay's
    /// `privateContactRequest` store
    /// (`PlatformWallet::restore_pending_private_contact_requests` reads
    /// them). Each opens with the keys of the wallet's identities, and every
    /// one an identity sent reserves its recipient again under the entry's
    /// commitment, so no second request to it is sent, privately or
    /// publicly, until a private sync reads the entry back and settles it.
    /// A recipient already reserved keeps its reservation, which gets the
    /// entry's commitment only when it has none yet. Returns the (sender,
    /// recipient) pairs restored.
    ///
    /// An entry is discharged once its own request or a superseding one is
    /// recorded: it is left alone when the request the sender tracks to its
    /// recipient has the entry's account reference, or a later version. So an
    /// unconfirmed rotation is restored over the request it rotates, and an
    /// entry read back while its spend still waits for confirmation is not.
    ///
    /// An entry whose request does not decode is left alone: no sync could
    /// record it, so its reservation would never settle. A record saved with
    /// the request it pays for is restored without this call when the wallet
    /// binds to its shielded coordinator
    /// (`PlatformWallet::restore_private_contact_payments`); this call stays
    /// for records saved before that.
    pub async fn restore_pending_private_contact_requests<C>(
        &self,
        entries: &[BinaryData],
        crypto: &C,
    ) -> Result<Vec<(Identifier, Identifier)>, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        self.restore_private_reservations(entries, crypto, None)
            .await
    }

    /// Reserve again, after a restart, the recipients of the private
    /// requests whose payment is unconfirmed, from what the payment's record
    /// was saved with: (sender, recipient, account reference, the entry's
    /// commitment). Nothing is opened, so no keys are needed. A payment whose
    /// request the sender's tracked request to that recipient records or
    /// supersedes is discharged and left alone, as
    /// [`Self::restore_pending_private_contact_requests`] leaves it; a
    /// sender that is not one of the wallet's identities is skipped. So is a
    /// payment the shielded coordinator settled as never executing in this
    /// session, though its record was read before that
    /// (`WalletPersister::private_entry_rejected`, checked under the wallet
    /// lock). Returns the (sender, recipient) pairs restored.
    #[cfg(feature = "shielded")]
    pub(crate) async fn restore_private_payments(
        &self,
        payments: &[(Identifier, Identifier, u32, [u8; 32])],
    ) -> Result<Vec<(Identifier, Identifier)>, PlatformWalletError> {
        if payments.is_empty() {
            return Ok(Vec::new());
        }
        let mut restored = Vec::new();
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        for &(sender, recipient, account_reference, commitment) in payments {
            // Settled as never executing, though its record was read before.
            if self.persister.private_entry_rejected(&commitment) {
                continue;
            }
            let Some(managed) = info.identity_manager.managed_identity_mut(&sender) else {
                continue;
            };
            if managed
                .dashpay()
                .outgoing_request(&recipient)
                .is_some_and(|tracked| records_or_supersedes(tracked, account_reference))
            {
                continue;
            }
            managed.restore_private_reply(recipient, commitment);
            if !restored.contains(&(sender, recipient)) {
                restored.push((sender, recipient));
            }
        }
        Ok(restored)
    }

    /// Whether a request from `sender` to `recipient` among `entries`, paid
    /// for before a restart and unconfirmed, may still execute, neither it nor
    /// a later request being recorded (a rotation over the tracked request
    /// counts): then the recipient stays reserved for that entry instead of
    /// the one about to be paid, which must not be. The other entries are
    /// restored as [`Self::restore_pending_private_contact_requests`] restores
    /// them.
    #[cfg(any(feature = "shielded", test))]
    pub(crate) async fn hold_for_restored_private_contact_request<C>(
        &self,
        entries: &[BinaryData],
        crypto: &C,
        sender: &Identifier,
        recipient: &Identifier,
    ) -> Result<bool, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let pair = (*sender, *recipient);
        Ok(self
            .restore_private_reservations(entries, crypto, Some(pair))
            .await?
            .contains(&pair))
    }

    /// Open `entries` as one page of the store and reserve the recipient of
    /// each the wallet's identities sent whose request is neither recorded
    /// nor superseded yet; the pair `paying` is retargeted to its restored
    /// entry instead. An entry whose payment the shielded coordinator settled
    /// as never executing in this session is left alone, though the
    /// settlement came while the entries were opened
    /// (`WalletPersister::private_entry_rejected`, checked under the wallet
    /// lock).
    async fn restore_private_reservations<C>(
        &self,
        entries: &[BinaryData],
        crypto: &C,
        paying: Option<(Identifier, Identifier)>,
    ) -> Result<Vec<(Identifier, Identifier)>, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        let (_, paths) = self.private_scan_identities().await?;
        if paths.senders.is_empty() {
            return Ok(Vec::new());
        }
        let contract = dashpay_contract()?;
        let document_type = contract
            .document_type_for_name(PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME)
            .map_err(|error| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "the bundled DashPay has no {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME} type: \
                     {error}"
                ))
            })?;
        let page = PrivateDocumentEntries {
            chunk_power: None,
            entries: entries
                .iter()
                .zip(0u64..)
                .map(|(data, position)| PrivateDocumentEntry {
                    position,
                    block_height: 0,
                    block_time_ms: 0,
                    data: data.to_vec(),
                })
                .collect(),
            total_count: entries.len() as u64,
        };
        let scan = crypto
            .scan_private_document_page(
                &paths,
                contract.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                0,
                &page,
            )
            .await?;

        let mut restored = Vec::new();
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        for sent in scan.sent {
            let sender = sent.sender_identity_id;
            // The wallet's own entries name the identity that sealed them.
            if Identifier::from(*sent.entry.sender_identity_id()) != sender {
                continue;
            }
            let recipient = Identifier::from(*sent.entry.recipient_identity_id());
            let request = decode_private_document_payload(
                document_type,
                sent.entry.payload(),
                self.sdk.version(),
            )
            .map_err(|error| error.to_string())
            .and_then(|properties| {
                PrivateContactRequest::from_properties(properties)
                    .map_err(|error| error.to_string())
            });
            let request = match request {
                Ok(request) => request,
                Err(error) => {
                    tracing::warn!(
                        sender = %sender,
                        recipient = %recipient,
                        %error,
                        "an unconfirmed private contact request does not decode; not restored"
                    );
                    continue;
                }
            };
            let Some(managed) = info.identity_manager.managed_identity_mut(&sender) else {
                continue;
            };
            // Recorded, or superseded by a later request: discharged.
            if managed
                .dashpay()
                .outgoing_request(&recipient)
                .is_some_and(|tracked| records_or_supersedes(tracked, request.account_reference))
            {
                continue;
            }
            let commitment = *sent.entry.commitment();
            // Settled as never executing while its entry was opened.
            if self.persister.private_entry_rejected(&commitment) {
                continue;
            }
            if paying == Some((sender, recipient)) {
                managed.retarget_private_reply(recipient, commitment);
            } else {
                managed.restore_private_reply(recipient, commitment);
            }
            if !restored.contains(&(sender, recipient)) {
                restored.push((sender, recipient));
            }
        }
        Ok(restored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, ClientWalletStartState, IdentityManagerStartState,
        PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::identity::network::identity_auth_derivation_path_for_type;
    use crate::wallet::identity::network::SeedCryptoProvider;
    use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
    #[cfg(feature = "shielded")]
    use crate::wallet::shielded::operations::{
        arm_redrive_before_broadcast, note_rejected_private_documents,
    };
    #[cfg(feature = "shielded")]
    use crate::wallet::shielded::{
        FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, PrivateContactPayment,
        SubwalletId,
    };
    use crate::PlatformWalletManager;
    use dash_sdk::error::ContextProviderError;
    use dash_sdk::platform::private_document_entries::{
        PrivateDocumentEntriesQuery, PrivateDocumentEntry,
    };
    use dash_sdk::platform::private_document_scan::PrivateDocumentScan;
    use dash_sdk::platform::ContextProvider;
    use dpp::address_funds::AddressWitness;
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{IdentityPublicKey, SecurityLevel};
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;
    use platform_encryption::{
        account_reference_version, decrypt_account_label, decrypt_extended_public_key,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;

    const ALICE_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
                                abandon abandon abandon about";
    const BOB_PHRASE: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const ALICE: [u8; 32] = [0xA1; 32];
    const BOB: [u8; 32] = [0xB1; 32];
    const CAROL_PHRASE: &str =
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage above";
    const CAROL: [u8; 32] = [0xC2; 32];
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

    /// A backend recording every changeset stored, attesting `capabilities`.
    #[derive(Default)]
    struct RecordingPersistence {
        capabilities: PersistenceCapabilities,
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl RecordingPersistence {
        fn attesting(capabilities: PersistenceCapabilities) -> Arc<Self> {
            Arc::new(Self {
                capabilities,
                stored: Mutex::new(Vec::new()),
            })
        }

        /// The contact changesets stored, in order.
        fn contacts(&self) -> Vec<ContactChangeSet> {
            self.stored
                .lock()
                .expect("stored")
                .iter()
                .filter_map(|changeset| changeset.contacts.clone())
                .collect()
        }
    }

    impl PlatformWalletPersistence for RecordingPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored.lock().expect("stored").push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }

        fn persistence_capabilities(&self) -> PersistenceCapabilities {
            self.capabilities
        }
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
        wallet_on(Arc::new(NoPersistence), phrase, own, known, entries).await
    }

    /// [`wallet`] on the backend `persistence`.
    async fn wallet_on<P: PlatformWalletPersistence + 'static>(
        persistence: Arc<P>,
        phrase: &str,
        own: &Identity,
        known: &[&Identity],
        entries: Vec<Vec<u8>>,
    ) -> (PlatformWalletManager<P>, Arc<PlatformWallet>) {
        wallet_at(
            persistence,
            phrase,
            own,
            known,
            entries,
            PlatformVersion::latest(),
            None,
        )
        .await
    }

    /// [`wallet_on`], with its SDK pinned to `platform_version` and holding
    /// DashPay as that version bundles it, which a DashPay fetch also
    /// answers; `verified_network` is the network's version a verified
    /// answer showed, when one did.
    async fn wallet_at<P: PlatformWalletPersistence + 'static>(
        persistence: Arc<P>,
        phrase: &str,
        own: &Identity,
        known: &[&Identity],
        entries: Vec<Vec<u8>>,
        platform_version: &'static PlatformVersion,
        verified_network: Option<u32>,
    ) -> (PlatformWalletManager<P>, Arc<PlatformWallet>) {
        let sdk = dashpay_sdk(known, entries, platform_version, verified_network).await;
        let manager =
            PlatformWalletManager::new(Arc::new(sdk), persistence, Arc::new(NoopEventHandler));
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

    /// The mock SDK of [`wallet_at`]: pinned to `platform_version`, holding
    /// DashPay as that version bundles it, answering fetches of `known` and
    /// of the private store's first page, holding `entries`.
    async fn dashpay_sdk(
        known: &[&Identity],
        entries: Vec<Vec<u8>>,
        platform_version: &'static PlatformVersion,
        verified_network: Option<u32>,
    ) -> dash_sdk::Sdk {
        let contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                platform_version,
            )
            .expect("bundled DashPay"),
        );
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_version(platform_version)
            .with_context_provider(DashpayContractProvider(Arc::clone(&contract)))
            .build()
            .expect("mock sdk");
        if let Some(verified) = verified_network {
            sdk.mock_verified_protocol_version(verified)
                .expect("a mock SDK");
        }
        sdk.mock()
            .expect_fetch(contract.id(), Some((*contract).clone()))
            .await
            .expect("DashPay expectation");
        for identity in known {
            sdk.mock()
                .expect_fetch(identity.id(), Some((*identity).clone()))
                .await
                .expect("identity expectation");
        }
        let total_count = entries.len() as u64;
        let page = PrivateDocumentEntries {
            chunk_power: None,
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
        sdk
    }

    /// A backend handing back, on every load, a wallet as [`snapshot`] or
    /// [`scan_state_snapshot`] took it: the relaunch of a wallet from an
    /// earlier process, with the viewing key its shielded account 0 bound
    /// with, when it bound one. It keeps what the relaunched wallet stores.
    struct SnapshotPersistence {
        wallet: key_wallet::Wallet,
        core: key_wallet::wallet::ManagedWalletInfo,
        identities: BTreeMap<u32, ManagedIdentity>,
        capabilities: PersistenceCapabilities,
        #[cfg(feature = "shielded")]
        viewing_key: Option<[u8; 96]>,
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl SnapshotPersistence {
        /// Whether the last position record stored for `owner` marks a private
        /// payment that may be unresolved, if one was stored.
        #[cfg(feature = "shielded")]
        fn last_marker(&self, owner: &Identifier) -> Option<bool> {
            last_marker(&self.stored.lock().expect("stored"), owner)
        }
    }

    /// Whether the last of `stored` to carry `owner`'s position record marks a
    /// private payment that may be unresolved.
    fn last_marker(stored: &[PlatformWalletChangeSet], owner: &Identifier) -> Option<bool> {
        stored
            .iter()
            .rev()
            .filter_map(|changeset| changeset.contacts.as_ref())
            .find_map(|contacts| contacts.private_contact_requests_positions.get(owner))
            .map(|position| position.private_payments_unresolved)
    }

    impl PlatformWalletPersistence for SnapshotPersistence {
        fn persistence_capabilities(&self) -> PersistenceCapabilities {
            self.capabilities
        }

        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored.lock().expect("stored").push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            let wallet_id = self.wallet.compute_wallet_id();
            let mut wallet_identities = BTreeMap::new();
            wallet_identities.insert(wallet_id, self.identities.clone());
            let mut wallets = BTreeMap::new();
            wallets.insert(
                wallet_id,
                ClientWalletStartState {
                    wallet: self.wallet.clone(),
                    wallet_info: self.core.clone(),
                    identity_manager: IdentityManagerStartState {
                        wallet_identities,
                        ..Default::default()
                    },
                    unused_asset_locks: BTreeMap::new(),
                    unconfirmed_outgoing_txs: Vec::new(),
                    dashpay_backfill: Default::default(),
                },
            );
            #[allow(unused_mut)]
            let mut state = ClientStartState {
                wallets,
                ..Default::default()
            };
            #[cfg(feature = "shielded")]
            if let Some(viewing_key) = self.viewing_key {
                state
                    .shielded
                    .viewing_keys
                    .insert(SubwalletId::new(wallet_id, 0), viewing_key.to_vec());
            }
            Ok(state)
        }
    }

    /// `wallet`, from `phrase`, with its identity `owner`, as a backend keeps
    /// it once its shielded account 0 is bound.
    #[cfg(feature = "shielded")]
    async fn snapshot(
        wallet: &PlatformWallet,
        owner: &Identifier,
        phrase: &str,
    ) -> SnapshotPersistence {
        let iw = wallet.identity();
        let wm = iw.wallet_manager.read().await;
        let wallet_id = wallet.wallet_id();
        let info = wm.get_wallet_info(&wallet_id).expect("wallet info");
        let mut identities = BTreeMap::new();
        identities.insert(
            0,
            info.identity_manager
                .managed_identity(owner)
                .expect("the owner")
                .clone(),
        );
        SnapshotPersistence {
            wallet: wm.get_wallet(&wallet_id).expect("the wallet").clone(),
            core: info.core_wallet.clone(),
            identities,
            capabilities: PersistenceCapabilities::SHIELDED_FVK_RESTART,
            viewing_key: Some(
                OrchardKeySet::from_seed(&seed(phrase), Network::Testnet, 0)
                    .expect("shielded keys")
                    .viewing_keys()
                    .to_fvk_bytes(),
            ),
            stored: Mutex::default(),
        }
    }

    /// `wallet`, with its identity `owner`, as a backend keeping the private
    /// contact request scan state keeps it, with no shielded viewing key: the
    /// identity's position record marks a private payment that may be
    /// unresolved when `unresolved`, as a process that paid for one and ended
    /// before it resolved stored it.
    async fn scan_state_snapshot(
        wallet: &PlatformWallet,
        owner: &Identifier,
        unresolved: bool,
    ) -> SnapshotPersistence {
        let iw = wallet.identity();
        let wm = iw.wallet_manager.read().await;
        let wallet_id = wallet.wallet_id();
        let info = wm.get_wallet_info(&wallet_id).expect("wallet info");
        let mut managed = info
            .identity_manager
            .managed_identity(owner)
            .expect("the owner")
            .clone();
        managed.apply_private_contact_requests_position(PrivateContactRequestsPosition {
            private_payments_unresolved: unresolved,
            ..managed.private_contact_requests_position_record()
        });
        SnapshotPersistence {
            wallet: wm.get_wallet(&wallet_id).expect("the wallet").clone(),
            core: info.core_wallet.clone(),
            identities: BTreeMap::from([(0, managed)]),
            capabilities: PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE,
            #[cfg(feature = "shielded")]
            viewing_key: None,
            stored: Mutex::default(),
        }
    }

    /// A process that paid for a private contact request and ended before the
    /// payment resolved stored the identity's position record marking it, in a
    /// build with the `shielded` feature: reloaded, in any build, the identity
    /// sends and accepts no public contact request, with nothing read, signed
    /// or sent, as no reservation holds the pair the payment is for. A build
    /// with the `shielded` feature releases it once the shielded store is
    /// configured, which restores the reservations and stores the record
    /// unmarked; no private sync does, and a build without it keeps it.
    #[tokio::test]
    async fn should_hold_an_identity_reloaded_marking_an_unresolved_private_payment() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let incoming = public_request(&bob, &alice, 3);
        let (_before_manager, before) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        add_incoming(&before, ALICE, &incoming).await;
        let kept = Arc::new(scan_state_snapshot(&before, &alice.id(), true).await);

        let manager = PlatformWalletManager::new(
            Arc::new(dashpay_sdk(&[&alice, &bob], vec![], PlatformVersion::latest(), None).await),
            Arc::clone(&kept),
            Arc::new(NoopEventHandler),
        );
        manager
            .load_from_persistor()
            .await
            .expect("the relaunch loads");
        let restarted = manager
            .get_wallet(&before.wallet_id())
            .await
            .expect("the relaunched wallet");
        assert!(restarted.private_payments_held().await);

        let public = restarted
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
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactPaymentsNotRestored)
            ),
            "{public:?}"
        );
        let accepted = restarted
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &incoming,
                None,
                &alice_signer,
                &alice_crypto,
                no_payment,
            )
            .await;
        assert!(
            matches!(
                accepted,
                Err(PlatformWalletError::PrivateContactPaymentsNotRestored)
            ),
            "{accepted:?}"
        );

        // A private sync reads the store, and restores nothing.
        let view = restarted.identity().dashpay();
        view.sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("the private sync");
        assert_eq!(
            view.private_contact_readiness(&alice.id())
                .await
                .expect("the readiness"),
            PrivateContactReadiness {
                private_payments_restored: false,
                private_store_read: true,
                unresolved_private_payment: true,
            }
        );
        assert!(restarted.private_payments_held().await, "still held");

        #[cfg(feature = "shielded")]
        {
            let dir = tempfile::tempdir().expect("temporary shielded store");
            manager
                .configure_shielded(dir.path().join("tree.sqlite"))
                .await
                .expect("the shielded store is configured");
            assert!(!restarted.private_payments_held().await, "restored");
            assert_eq!(kept.last_marker(&alice.id()), Some(false));
            assert_eq!(
                view.private_contact_readiness(&alice.id())
                    .await
                    .expect("the readiness"),
                PrivateContactReadiness {
                    private_payments_restored: true,
                    private_store_read: true,
                    unresolved_private_payment: false,
                }
            );
        }
        #[cfg(not(feature = "shielded"))]
        assert!(
            restarted.private_payments_held().await,
            "no shielded store restores it here"
        );
    }

    /// A private payment is marked unresolved in its identity's position
    /// record before it is paid, for a backend keeping the scan state, and
    /// unmarked once its entry is read back; one released before anything was
    /// paid is unmarked too.
    #[tokio::test]
    async fn should_mark_a_private_payment_unresolved_until_it_resolves() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let attesting = || {
            RecordingPersistence::attesting(
                PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE,
            )
        };
        let marker = |persistence: &RecordingPersistence| {
            last_marker(&persistence.stored.lock().expect("stored"), &alice.id())
        };

        let persistence = attesting();
        let (_manager, alice_wallet) = wallet_on(
            Arc::clone(&persistence),
            ALICE_PHRASE,
            &alice,
            &[&alice, &bob],
            vec![],
        )
        .await;
        let marked_when_paid = Arc::new(AtomicBool::new(false));
        let kept = Arc::new(Mutex::new(Vec::new()));
        let pay = {
            let paying = append_entry(&alice_wallet, 0, vec![], &kept);
            let (persistence, marked, alice_id) = (
                Arc::clone(&persistence),
                Arc::clone(&marked_when_paid),
                alice.id(),
            );
            move |entry: BinaryData, started: PaymentStarted| {
                let unresolved =
                    last_marker(&persistence.stored.lock().expect("stored"), &alice_id);
                marked.store(unresolved == Some(true), Ordering::SeqCst);
                paying(entry, started)
            }
        };
        alice_wallet
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
                pay,
            )
            .await
            .expect("paid and read back");
        assert!(
            marked_when_paid.load(Ordering::SeqCst),
            "marked before paying"
        );
        assert_eq!(marker(&persistence), Some(false), "unmarked once read back");

        let persistence = attesting();
        let (_manager, alice_wallet) = wallet_on(
            Arc::clone(&persistence),
            ALICE_PHRASE,
            &alice,
            &[&alice, &bob],
            vec![],
        )
        .await;
        let marked_when_paid = Arc::new(AtomicBool::new(false));
        let failing = {
            let (persistence, marked, alice_id) = (
                Arc::clone(&persistence),
                Arc::clone(&marked_when_paid),
                alice.id(),
            );
            move |_: BinaryData, _: PaymentStarted| {
                let unresolved =
                    last_marker(&persistence.stored.lock().expect("stored"), &alice_id);
                marked.store(unresolved == Some(true), Ordering::SeqCst);
                std::future::ready(Err::<ResponseMetadata, _>(
                    PlatformWalletError::InvalidParameter("no notes to pay with".to_string()),
                ))
            }
        };
        let refused = alice_wallet
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
                failing,
            )
            .await;
        assert!(refused.is_err(), "{refused:?}");
        assert!(
            marked_when_paid.load(Ordering::SeqCst),
            "marked before paying"
        );
        assert_eq!(marker(&persistence), Some(false), "unmarked once released");
    }

    /// The page of DashPay's private store from `start`, holding `entries`.
    fn store_page(start: u64, entries: Vec<Vec<u8>>) -> PrivateDocumentEntries {
        let total_count = start + entries.len() as u64;
        PrivateDocumentEntries {
            chunk_power: None,
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
        PaymentStarted,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<ResponseMetadata, PlatformWalletError>> + Send>,
    > + Send {
        let wallet = Arc::clone(wallet);
        let kept = Arc::clone(kept);
        move |entry, started| {
            started.mark();
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
    ) -> impl FnOnce(
        BinaryData,
        PaymentStarted,
    ) -> std::future::Ready<Result<ResponseMetadata, PlatformWalletError>>
           + Send {
        let kept = Arc::clone(kept);
        move |entry, started| {
            started.mark();
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

    /// A sync reports the requests it recorded: an ignored sender's request is
    /// opened but not recorded, so not reported; once the sender is no longer
    /// ignored, the store is read again and the request is.
    #[tokio::test]
    async fn should_report_only_the_requests_a_sync_recorded() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        alice_wallet
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

        let (_bob_manager, bob_wallet) =
            wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry.to_vec()]).await;
        let set_ignored = |ignored: bool| {
            let bob_wallet = Arc::clone(&bob_wallet);
            let alice_id = alice.id();
            let bob_id = bob.id();
            async move {
                let iw = bob_wallet.identity();
                let mut wm = iw.wallet_manager.write().await;
                let managed = wm
                    .get_wallet_info_mut(&bob_wallet.wallet_id())
                    .expect("wallet info")
                    .identity_manager
                    .managed_identity_mut(&bob_id)
                    .expect("Bob");
                if ignored {
                    managed.ignore_sender(&alice_id);
                } else {
                    managed.unignore_sender(&alice_id);
                }
            }
        };

        set_ignored(true).await;
        let report = bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");
        assert_eq!(report.requests, vec![], "an ignored sender's request");
        assert_eq!(report.rejected, vec![]);

        set_ignored(false).await;
        let report = bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store again");
        assert_eq!(report.requests.len(), 1, "{:?}", report.requests);
        assert_eq!(report.requests[0].sender_id, alice.id());
        managed(&bob_wallet, BOB, |managed| {
            assert_eq!(
                managed.dashpay().incoming_request(&alice.id()),
                Some(&report.requests[0])
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
        let failed = send!(|_, _| std::future::ready(Err(
            PlatformWalletError::InvalidIdentityData("no notes".to_string())
        )));
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
                |_, started: PaymentStarted| {
                    started.mark();
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
        // A public request now would publish the pair, whose private one may
        // already be on chain.
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
            Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob.id()
        ));
    }

    /// A signer whose signatures never come, as a host's prompt the user never
    /// answers: it notes that it was asked.
    #[derive(Debug)]
    struct StalledSigner {
        inner: simple_signer::signer::SimpleSigner,
        asked: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for StalledSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            self.asked.store(true, Ordering::SeqCst);
            std::future::pending().await
        }

        async fn sign_create_witness(
            &self,
            key: &IdentityPublicKey,
            data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            self.inner.sign_create_witness(key, data).await
        }

        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.inner.can_sign_with(key)
        }
    }

    /// From now on, `wallet`'s mock SDK answers a fetch of `identity` with it
    /// when `known`, and fails it otherwise.
    async fn set_identity_known(wallet: &PlatformWallet, identity: &Identity, known: bool) {
        let mut sdk = (*wallet.identity().sdk).clone();
        let mut mock = sdk.mock();
        mock.remove_fetch_expectation::<Identity, _>(identity.id())
            .await;
        if known {
            mock.expect_fetch(identity.id(), Some(identity.clone()))
                .await
                .expect("identity expectation");
        }
    }

    /// Whether `wallet`'s identity `identity_id` holds a private request to
    /// `recipient` pending.
    async fn pending_to(
        wallet: &PlatformWallet,
        identity_id: [u8; 32],
        recipient: &Identity,
    ) -> bool {
        managed(wallet, identity_id, |managed| {
            managed.dashpay().private_reply_pending(&recipient.id())
        })
        .await
    }

    /// A send dropped before it pays, here while its signer never answers,
    /// leaves no reservation behind: nothing was paid, and no sync could
    /// settle one. The recipient is free for the next send.
    #[tokio::test]
    async fn should_release_the_reservation_of_a_send_dropped_before_it_pays() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        let asked = Arc::new(AtomicBool::new(false));
        let stalled = StalledSigner {
            inner: signer(ALICE_PHRASE, &alice),
            asked: Arc::clone(&asked),
        };
        let paid = Arc::new(Mutex::new(Vec::new()));

        let cancelled = tokio::time::timeout(
            Duration::from_millis(200),
            view.send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &stalled,
                &alice_crypto,
                keep_entry(&paid),
            ),
        )
        .await;
        assert!(cancelled.is_err(), "the send never completes");
        assert!(
            asked.load(Ordering::SeqCst),
            "it was dropped while signing, so after reserving"
        );
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");
        assert!(!pending_to(&alice_wallet, ALICE, &bob).await);

        let kept = Arc::new(Mutex::new(Vec::new()));
        let next = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &signer(ALICE_PHRASE, &alice),
                &alice_crypto,
                keep_entry(&kept),
            )
            .await;
        assert!(
            matches!(
                next,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "{next:?}"
        );
        assert_eq!(kept.lock().expect("kept").len(), 1, "the next send pays");
    }

    /// A sync settles a pending request only once it records the entry: one
    /// whose recipient cannot be read leaves no record, so the request stays
    /// pending, and it settles on a later sync that reads the recipient.
    #[tokio::test]
    async fn should_settle_a_pending_request_only_once_its_entry_is_recorded() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        let kept = Arc::new(Mutex::new(Vec::new()));
        let sent = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&kept),
            )
            .await;
        assert!(
            matches!(
                sent,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "{sent:?}"
        );
        let entry = kept.lock().expect("kept").remove(0);
        publish(&alice_wallet, 0, vec![entry.to_vec()]).await;

        set_identity_known(&alice_wallet, &bob, false).await;
        let report = view
            .sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("Alice reads the store");
        assert_eq!(report.requests, vec![]);
        assert_eq!(report.unpersisted_identities, vec![alice.id()]);
        managed(&alice_wallet, ALICE, |managed| {
            assert!(managed.dashpay().private_reply_pending(&bob.id()));
            assert!(managed.dashpay().outgoing_request(&bob.id()).is_none());
        })
        .await;

        set_identity_known(&alice_wallet, &bob, true).await;
        let report = view
            .sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("Alice reads the store again");
        assert_eq!(report.requests.len(), 1, "{:?}", report.requests);
        managed(&alice_wallet, ALICE, |managed| {
            assert!(!managed.dashpay().private_reply_pending(&bob.id()));
            assert_eq!(
                managed.dashpay().outgoing_request(&bob.id()),
                Some(&report.requests[0])
            );
        })
        .await;
    }

    /// A pending request whose entry is read back after a newer request to
    /// the recipient is tracked, here as another device's rotation would be,
    /// is settled: its entry executed, and the newer request stays tracked.
    #[tokio::test]
    async fn should_settle_a_pending_request_read_back_after_a_newer_one() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        let kept = Arc::new(Mutex::new(Vec::new()));
        let sent = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&kept),
            )
            .await;
        assert!(
            matches!(
                sent,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "{sent:?}"
        );
        let entry = kept.lock().expect("kept").remove(0);

        let mut newer = public_request(&alice, &bob, (1 << 28) | 5);
        newer.core_height_created_at = 0;
        {
            let iw = alice_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&alice_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&alice.id())
                .expect("Alice")
                .add_sent_contact_request(newer.clone(), &iw.persister)
                .expect("the newer request");
        }
        publish(&alice_wallet, 0, vec![entry.to_vec()]).await;
        view.sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("Alice reads the store");
        managed(&alice_wallet, ALICE, |managed| {
            assert!(!managed.dashpay().private_reply_pending(&bob.id()));
            assert_eq!(managed.dashpay().outgoing_request(&bob.id()), Some(&newer));
        })
        .await;
    }

    /// A request paid for and unconfirmed before a restart is restored from
    /// its entry: until a sync reads the entry back, neither a second private
    /// request nor a public one goes to the recipient. Once recorded, the
    /// entry restores nothing.
    #[tokio::test]
    async fn should_restore_a_pending_request_paid_for_before_a_restart() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, before) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        before
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
                keep_entry(&kept),
            )
            .await
            .expect_err("not read back");
        let entry = kept.lock().expect("kept").remove(0);

        let (_restarted_manager, restarted) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = restarted.identity().dashpay();
        assert!(!pending_to(&restarted, ALICE, &bob).await);
        let restored = view
            .restore_pending_private_contact_requests(std::slice::from_ref(&entry), &alice_crypto)
            .await
            .expect("restored");
        assert_eq!(restored, vec![(alice.id(), bob.id())]);
        assert!(pending_to(&restarted, ALICE, &bob).await);

        let paid = Arc::new(Mutex::new(Vec::new()));
        let second = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&paid),
            )
            .await;
        assert!(
            matches!(second, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob.id()),
            "{second:?}"
        );
        assert!(
            paid.lock().expect("paid").is_empty(),
            "nothing is paid twice"
        );
        let public = view
            .send_contact_request_with_external_signer(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                &alice_signer,
                &alice_crypto,
            )
            .await;
        assert!(
            matches!(public, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob.id()),
            "{public:?}"
        );

        publish(&restarted, 0, vec![entry.to_vec()]).await;
        view.sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("the restarted wallet reads the store");
        managed(&restarted, ALICE, |managed| {
            assert!(!managed.dashpay().private_reply_pending(&bob.id()));
            assert!(managed.dashpay().outgoing_request(&bob.id()).is_some());
        })
        .await;
        assert_eq!(
            view.restore_pending_private_contact_requests(&[entry], &alice_crypto)
                .await
                .expect("restored"),
            vec![],
            "a recorded entry is not restored"
        );
    }

    /// A send whose payment finds a request to the same recipient paid for
    /// and unconfirmed before a restart pays nothing: the recipient stays
    /// reserved for that request, which a sync settles once it reads it back.
    #[tokio::test]
    async fn should_hold_a_send_for_a_request_paid_for_before_a_restart() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let (alice_id, bob_id) = (alice.id(), bob.id());
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, before) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        before
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice_id,
                &bob_id,
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&kept),
            )
            .await
            .expect_err("not read back");
        let unconfirmed = vec![kept.lock().expect("kept").remove(0)];

        // The host restores nothing at startup; the payment checks.
        let (_restarted_manager, restarted) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = restarted.identity().dashpay();
        let paid = Arc::new(Mutex::new(Vec::new()));
        let held = view
            .send_private_contact_request(
                &alice_id,
                &bob_id,
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                |entry: BinaryData, started: PaymentStarted| {
                    let paid = Arc::clone(&paid);
                    let unconfirmed = &unconfirmed;
                    let crypto = &alice_crypto;
                    async move {
                        if view
                            .hold_for_restored_private_contact_request(
                                unconfirmed,
                                crypto,
                                &alice_id,
                                &bob_id,
                            )
                            .await?
                        {
                            return Err(PlatformWalletError::PrivateContactRequestPending(bob_id));
                        }
                        started.mark();
                        paid.lock().expect("paid").push(entry);
                        Ok(ResponseMetadata::default())
                    }
                },
            )
            .await;
        assert!(
            matches!(held, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob_id),
            "{held:?}"
        );
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");
        assert!(pending_to(&restarted, ALICE, &bob).await);

        // The restored entry, not the unpaid one, settles it.
        publish(&restarted, 0, vec![unconfirmed[0].to_vec()]).await;
        view.sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("the restarted wallet reads the store");
        assert!(!pending_to(&restarted, ALICE, &bob).await);
    }

    /// A rotation whose payment is unconfirmed at a restart, over a request
    /// that was recorded, is restored: the recorded request is older, so it
    /// does not discharge the rotation. Until the rotation is read back, a
    /// retried rotation is refused unpaid, whether the host restored at
    /// startup or the payment finds it; once it is recorded, its entry, and
    /// the older one, restore nothing.
    #[tokio::test]
    async fn should_restore_a_rotation_paid_for_before_a_restart() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let (alice_id, bob_id) = (alice.id(), bob.id());
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, before) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        let v0 = before
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice_id,
                &bob_id,
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                append_entry(&before, 0, vec![], &kept),
            )
            .await
            .expect("v0 is recorded");
        assert_eq!(account_reference_version(v0.account_reference), 0);
        let v0_entry = kept.lock().expect("kept").remove(0);
        let unconfirmed = {
            let kept = Arc::clone(&kept);
            move |entry: BinaryData, started: PaymentStarted| {
                started.mark();
                kept.lock().expect("kept").push(entry);
                std::future::ready(Err::<ResponseMetadata, _>(
                    PlatformWalletError::ShieldedSpendUnconfirmed {
                        operation: "private document create",
                        reason: "the proof fetch timed out".to_string(),
                    },
                ))
            }
        };
        let rotation = before
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice_id,
                &bob_id,
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                unconfirmed,
            )
            .await;
        assert!(
            matches!(
                rotation,
                Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
            ),
            "{rotation:?}"
        );
        let v1_entry = kept.lock().expect("kept").remove(0);

        // A restart: the store holds v0 only, which the restarted wallet
        // records. The host restores the unconfirmed rotation.
        let (_restarted_manager, restarted) = wallet(
            ALICE_PHRASE,
            &alice,
            &[&alice, &bob],
            vec![v0_entry.to_vec()],
        )
        .await;
        let view = restarted.identity().dashpay();
        view.sync_private_contact_requests(&alice_crypto, 4)
            .await
            .expect("the restarted wallet reads the store");
        managed(&restarted, ALICE, |managed| {
            assert_eq!(managed.dashpay().outgoing_request(&bob_id), Some(&v0));
        })
        .await;
        assert_eq!(
            view.restore_pending_private_contact_requests(
                std::slice::from_ref(&v1_entry),
                &alice_crypto
            )
            .await
            .expect("restored"),
            vec![(alice_id, bob_id)],
            "a recorded v0 does not discharge the v1 rotation"
        );
        let retried = view
            .send_private_contact_request(
                &alice_id,
                &bob_id,
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                no_payment,
            )
            .await;
        assert!(
            matches!(retried, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob_id),
            "{retried:?}"
        );

        // Another restart, the host restoring nothing: the payment of the
        // retried rotation finds the unconfirmed one and pays nothing.
        let (_held_manager, held_wallet) = wallet(
            ALICE_PHRASE,
            &alice,
            &[&alice, &bob],
            vec![v0_entry.to_vec()],
        )
        .await;
        let held_view = held_wallet.identity().dashpay();
        let paid = Arc::new(Mutex::new(Vec::new()));
        let unconfirmed_entries = vec![v1_entry.clone()];
        let held = held_view
            .send_private_contact_request(
                &alice_id,
                &bob_id,
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                |entry: BinaryData, started: PaymentStarted| {
                    let paid = Arc::clone(&paid);
                    let unconfirmed_entries = &unconfirmed_entries;
                    let crypto = &alice_crypto;
                    async move {
                        if held_view
                            .hold_for_restored_private_contact_request(
                                unconfirmed_entries,
                                crypto,
                                &alice_id,
                                &bob_id,
                            )
                            .await?
                        {
                            return Err(PlatformWalletError::PrivateContactRequestPending(bob_id));
                        }
                        started.mark();
                        paid.lock().expect("paid").push(entry);
                        Ok(ResponseMetadata::default())
                    }
                },
            )
            .await;
        assert!(
            matches!(held, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob_id),
            "{held:?}"
        );
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");

        // v1 lands and is read back, after v0 where each wallet's cursor is:
        // settled, and discharged.
        for wallet in [&restarted, &held_wallet] {
            publish(wallet, 1, vec![v1_entry.to_vec()]).await;
            wallet
                .identity()
                .dashpay()
                .sync_private_contact_requests(&alice_crypto, 4)
                .await
                .expect("the store is read");
            managed(wallet, ALICE, |managed| {
                assert!(!managed.dashpay().private_reply_pending(&bob_id));
                let tracked = managed
                    .dashpay()
                    .outgoing_request(&bob_id)
                    .expect("tracked");
                assert_eq!(account_reference_version(tracked.account_reference), 1);
            })
            .await;
        }
        assert_eq!(
            view.restore_pending_private_contact_requests(&[v1_entry, v0_entry], &alice_crypto)
                .await
                .expect("restored"),
            vec![],
            "a recorded rotation, and the request it rotated, restore nothing"
        );
    }

    /// From now on, `wallet`'s mock SDK serves a store holding `entries` in
    /// chunks of `chunk_size`, a page from each chunk's start to its end or
    /// the store's.
    async fn publish_chunked(wallet: &PlatformWallet, entries: &[Vec<u8>], chunk_size: usize) {
        let total = entries.len() as u64;
        for (index, chunk) in entries.chunks(chunk_size).enumerate() {
            let start = (index * chunk_size) as u64;
            let mut page = store_page(start, chunk.to_vec());
            page.total_count = total;
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
            mock.expect_fetch(query, Some(page))
                .await
                .expect("page expectation");
        }
    }

    /// What `wallet` tracks with `contacts` after a sync: each one's requests
    /// both ways.
    async fn tracked(
        wallet: &PlatformWallet,
        identity_id: [u8; 32],
        contacts: &[Identifier],
    ) -> Vec<(Option<ContactRequest>, Option<ContactRequest>)> {
        managed(wallet, identity_id, |managed| {
            contacts
                .iter()
                .map(|contact| {
                    (
                        managed.dashpay().outgoing_request(contact).cloned(),
                        managed.dashpay().incoming_request(contact).cloned(),
                    )
                })
                .collect()
        })
        .await
    }

    /// A store of many pages is read in one pass, its pages fetched together
    /// and scanned in batches: the wallet records what it records from the
    /// same entries in one page, and derives the scan keys once a batch, not
    /// once a page.
    #[tokio::test]
    async fn should_sync_a_store_of_many_pages_in_one_pass() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let carol = identity(CAROL_PHRASE, CAROL);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let everyone = [&alice, &bob, &carol];

        // Alice's requests to Bob and Carol, and Bob's to Alice.
        let (_alice_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &everyone, vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        for (start, recipient) in [(0, bob.id()), (1, carol.id())] {
            alice_wallet
                .identity()
                .dashpay()
                .send_private_contact_request(
                    &alice.id(),
                    &recipient,
                    None,
                    AutoAcceptProofSource::None,
                    None,
                    &alice_signer,
                    &alice_crypto,
                    append_entry(&alice_wallet, start, vec![], &kept),
                )
                .await
                .expect("Alice sends");
        }
        let to_bob = kept.lock().expect("kept")[0].to_vec();
        let (_bob_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &everyone, vec![to_bob]).await;
        bob_wallet
            .identity()
            .dashpay()
            .send_private_contact_request(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &bob_signer,
                &bob_crypto,
                append_entry(&bob_wallet, 1, vec![], &kept),
            )
            .await
            .expect("Bob sends");
        let entries: Vec<Vec<u8>> = kept
            .lock()
            .expect("kept")
            .iter()
            .map(|entry| entry.to_vec())
            .collect();
        assert_eq!(entries.len(), 3);
        let contacts = [bob.id(), carol.id()];

        // Restored from seed, once with the store in one page and once in
        // pages of one entry.
        let (_one_manager, one_page) =
            wallet(ALICE_PHRASE, &alice, &everyone, entries.clone()).await;
        let one_page_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let one = one_page
            .identity()
            .dashpay()
            .sync_private_contact_requests(&one_page_crypto, 64)
            .await
            .expect("the one-page store reads");
        assert_eq!(one.pages_read, 1);

        let (_many_manager, many_pages) = wallet(ALICE_PHRASE, &alice, &everyone, vec![]).await;
        publish_chunked(&many_pages, &entries, 1).await;
        let many_pages_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let many = many_pages
            .identity()
            .dashpay()
            .sync_private_contact_requests(&many_pages_crypto, 64)
            .await
            .expect("the store of many pages reads");
        assert_eq!(many.pages_read, 3);
        assert!(many.complete);
        assert_eq!(many.next_position, 3);
        assert_eq!(many.rejected, vec![]);
        assert_eq!(many.requests.len(), 3, "{:?}", many.requests);
        // A page's received requests are recorded before its sent ones, so
        // the order differs; the requests do not.
        let sorted = |mut requests: Vec<ContactRequest>| {
            requests.sort_by_key(|request| (request.sender_id, request.recipient_id));
            requests
        };
        assert_eq!(sorted(many.requests.clone()), sorted(one.requests.clone()));
        assert_eq!(
            tracked(&many_pages, ALICE, &contacts).await,
            tracked(&one_page, ALICE, &contacts).await
        );
        let derivations = many_pages_crypto
            .scan_key_derivations
            .load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            (1..many.pages_read).contains(&derivations),
            "{derivations} derivations for {} pages",
            many.pages_read
        );
    }

    /// Alice's private request to Bob, carrying `shielded_address`, read back
    /// from the store: its entry and Bob's record of it.
    async fn alice_to_bob(shielded_address: [u8; 43]) -> (Vec<u8>, ContactRequest) {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let (_alice_manager, alice_wallet) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        alice_wallet
            .identity()
            .dashpay()
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                Some(shielded_address),
                &signer(ALICE_PHRASE, &alice),
                &alice_crypto,
                append_entry(&alice_wallet, 0, vec![], &kept),
            )
            .await
            .expect("Alice sends");
        let entry = kept.lock().expect("kept").remove(0).to_vec();
        let (_bob_manager, bob_wallet) =
            wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry.clone()]).await;
        bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");
        let received = managed(&bob_wallet, BOB, |managed| {
            managed.dashpay().incoming_request(&alice.id()).cloned()
        })
        .await
        .expect("Bob holds Alice's request");
        (entry, received)
    }

    /// For a backend attesting the scan state, a sync stores the private
    /// contact's shielded address and then the read position, after the
    /// request the entry held.
    #[tokio::test]
    async fn should_store_the_read_position_after_what_the_entries_held() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let address = [0x55; 43];
        let (entry, _) = alice_to_bob(address).await;
        let persistence = RecordingPersistence::attesting(
            PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE,
        );
        let (_manager, bob_wallet) = wallet_on(
            Arc::clone(&persistence),
            BOB_PHRASE,
            &bob,
            &[&alice, &bob],
            vec![entry],
        )
        .await;
        bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");

        let contacts = persistence.contacts();
        let index_of = |found: &dyn Fn(&ContactChangeSet) -> bool| {
            contacts
                .iter()
                .position(found)
                .expect("the changeset is stored")
        };
        let request = index_of(&|changeset| !changeset.incoming_requests.is_empty());
        let addresses = index_of(&|changeset| {
            changeset
                .private_shielded_addresses
                .get(&(bob.id(), alice.id()))
                == Some(&address)
        });
        let position = index_of(&|changeset| {
            changeset
                .private_contact_requests_positions
                .get(&bob.id())
                .map(|position| position.position)
                == Some(1)
        });
        assert!(
            request < addresses && addresses < position,
            "request {request}, addresses {addresses}, position {position}"
        );
    }

    /// A wallet loaded with a stored read position and shielded address, as a
    /// backend attesting them restores them, reads the store on from there:
    /// no page before it is fetched, which the mock, answering none, would
    /// fail. The contact's shielded address is kept.
    #[tokio::test]
    async fn should_resume_a_private_sync_from_a_restored_position() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let address = [0x55; 43];
        let (entry, received) = alice_to_bob(address).await;
        let persistence = RecordingPersistence::attesting(
            PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE,
        );
        let (_manager, restored) =
            wallet_on(persistence, BOB_PHRASE, &bob, &[&alice, &bob], vec![entry]).await;
        {
            let iw = restored.identity();
            let mut wm = iw.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&restored.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&bob.id())
                .expect("Bob");
            managed.apply_incoming_contact_request(received.clone());
            let key_fingerprint = managed.private_scan_key_fingerprint();
            managed.apply_private_contact_requests_position(PrivateContactRequestsPosition {
                position: 1,
                key_fingerprint,
                private_payments_unresolved: false,
            });
            managed.apply_private_shielded_address(alice.id(), address);
        }
        let contract = dashpay_contract().expect("bundled DashPay");
        {
            let mut sdk = (*restored.identity().sdk).clone();
            sdk.mock()
                .remove_fetch_expectation::<PrivateDocumentEntries, _>(
                    PrivateDocumentEntriesQuery::new(
                        contract.id(),
                        PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                        0,
                    ),
                )
                .await;
        }
        publish(&restored, 1, vec![]).await;

        let report = restored
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("the store reads on from position 1");
        assert_eq!(report.pages_read, 1);
        assert!(report.complete);
        assert_eq!(report.next_position, 1);
        managed(&restored, BOB, |managed| {
            assert_eq!(
                managed
                    .dashpay()
                    .private_contact(&alice.id())
                    .expect("a private contact")
                    .shielded_address,
                Some(address)
            );
            assert_eq!(
                managed.dashpay().incoming_request(&alice.id()),
                Some(&received)
            );
        })
        .await;
    }

    /// Un-ignoring a sender rewinds the private read position, and a backend
    /// attesting the scan state stores the rewind.
    #[tokio::test]
    async fn should_store_a_rewound_position_when_a_sender_is_unignored() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let persistence = RecordingPersistence::attesting(
            PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE,
        );
        let (_manager, bob_wallet) = wallet_on(
            Arc::clone(&persistence),
            BOB_PHRASE,
            &bob,
            &[&alice, &bob],
            vec![],
        )
        .await;
        let view = bob_wallet.identity().dashpay();
        {
            let iw = bob_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&bob_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&bob.id())
                .expect("Bob")
                .apply_private_contact_requests_position(PrivateContactRequestsPosition {
                    position: 7,
                    key_fingerprint: [0; 32],
                    private_payments_unresolved: false,
                });
        }
        view.ignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("ignored");
        view.unignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("un-ignored");
        let unignore = persistence
            .contacts()
            .into_iter()
            .find(|changeset| changeset.unignored.contains(&(bob.id(), alice.id())))
            .expect("the un-ignore is stored");
        assert_eq!(
            unignore
                .private_contact_requests_positions
                .iter()
                .map(|(owner, position)| (*owner, position.position))
                .collect::<Vec<_>>(),
            vec![(bob.id(), 0)]
        );
    }

    /// A backend that does not attest the scan state gets none of it, and a
    /// load drops whatever it restored: the store is read from its start.
    #[tokio::test]
    async fn should_read_the_store_from_its_start_for_a_backend_without_the_capability() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let (entry, _) = alice_to_bob([0x55; 43]).await;
        let persistence = RecordingPersistence::attesting(PersistenceCapabilities::NONE);
        let (_manager, bob_wallet) = wallet_on(
            Arc::clone(&persistence),
            BOB_PHRASE,
            &bob,
            &[&alice, &bob],
            vec![entry],
        )
        .await;
        let view = bob_wallet.identity().dashpay();
        view.sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");
        view.ignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("ignored");
        view.unignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("un-ignored");
        assert!(persistence.contacts().iter().all(|changeset| {
            changeset.private_contact_requests_positions.is_empty()
                && changeset.private_shielded_addresses.is_empty()
                && changeset.removed_private_shielded_addresses.is_empty()
        }));

        let mut loaded = ManagedIdentity::new(bob.clone(), 0);
        let key_fingerprint = loaded.private_scan_key_fingerprint();
        loaded.apply_private_contact_requests_position(PrivateContactRequestsPosition {
            position: 5,
            key_fingerprint,
            private_payments_unresolved: false,
        });
        loaded.apply_private_shielded_address(alice.id(), [0x55; 43]);
        let mut start_state = IdentityManagerStartState::default();
        start_state
            .wallet_identities
            .entry(bob_wallet.wallet_id())
            .or_default()
            .insert(0, loaded);
        start_state.forget_private_contact_scan_state();
        let forgotten = &start_state.wallet_identities[&bob_wallet.wallet_id()][&0];
        assert_eq!(forgotten.dashpay().private_contact_requests_position(), 0);
        assert_eq!(
            forgotten.dashpay().private_shielded_address(&alice.id()),
            None
        );
    }

    /// A hook a [`HookedCrypto`] runs before one of its calls.
    type Hook = Arc<dyn Fn() -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

    /// A provider deriving from a test seed that runs hooks first: before
    /// deriving a receiving xpub, as a slow or interactive signer pauses a
    /// send's preparation, before scanning pages, and before scanning a
    /// single page, as a restoration does.
    struct HookedCrypto {
        inner: SeedCryptoProvider,
        before_receiving_xpub: Option<Hook>,
        before_scan: Option<Hook>,
        before_page_scan: Option<Hook>,
    }

    impl HookedCrypto {
        fn new(phrase: &str) -> Self {
            Self {
                inner: SeedCryptoProvider::from_seed(seed(phrase), Network::Testnet),
                before_receiving_xpub: None,
                before_scan: None,
                before_page_scan: None,
            }
        }
    }

    #[async_trait::async_trait]
    impl ContactCryptoProvider for HookedCrypto {
        async fn receiving_xpub(
            &self,
            path: &DerivationPath,
        ) -> Result<key_wallet::bip32::ExtendedPubKey, PlatformWalletError> {
            if let Some(hook) = &self.before_receiving_xpub {
                hook().await;
            }
            self.inner.receiving_xpub(path).await
        }

        async fn ecdh_shared_secret(
            &self,
            path: &DerivationPath,
            peer: &PublicKey,
        ) -> Result<zeroize::Zeroizing<[u8; 32]>, PlatformWalletError> {
            self.inner.ecdh_shared_secret(path, peer).await
        }

        async fn export_auto_accept_private_key(
            &self,
            path: &DerivationPath,
        ) -> Result<SecretKey, PlatformWalletError> {
            self.inner.export_auto_accept_private_key(path).await
        }

        async fn account_reference(
            &self,
            path: &DerivationPath,
            compact_xpub: &[u8],
            account_index: u32,
            version: u32,
        ) -> Result<u32, PlatformWalletError> {
            self.inner
                .account_reference(path, compact_xpub, account_index, version)
                .await
        }

        async fn unmask_account_reference(
            &self,
            path: &DerivationPath,
            compact_xpub: &[u8],
            account_reference: u32,
        ) -> Result<(u32, u32), PlatformWalletError> {
            self.inner
                .unmask_account_reference(path, compact_xpub, account_reference)
                .await
        }

        async fn contact_info_seal(
            &self,
            root_path: &DerivationPath,
            derivation_index: u32,
            contact_id: &[u8; 32],
            private_data_plaintext: &[u8],
            private_data_iv: &[u8; 16],
        ) -> Result<super::super::contact_requests::ContactInfoSealed, PlatformWalletError>
        {
            self.inner
                .contact_info_seal(
                    root_path,
                    derivation_index,
                    contact_id,
                    private_data_plaintext,
                    private_data_iv,
                )
                .await
        }

        async fn contact_info_open(
            &self,
            root_path: &DerivationPath,
            derivation_index: u32,
            enc_to_user_id: &[u8; 32],
            private_data_blob: &[u8],
        ) -> Result<super::super::contact_requests::ContactInfoOpened, PlatformWalletError>
        {
            self.inner
                .contact_info_open(
                    root_path,
                    derivation_index,
                    enc_to_user_id,
                    private_data_blob,
                )
                .await
        }

        async fn scan_private_document_page(
            &self,
            keys: &PrivateDocumentKeyPaths,
            data_contract_id: Identifier,
            document_type_name: &str,
            start_position: u64,
            page: &PrivateDocumentEntries,
        ) -> Result<PrivateDocumentScan, PlatformWalletError> {
            if let Some(hook) = &self.before_page_scan {
                hook().await;
            }
            self.inner
                .scan_private_document_page(
                    keys,
                    data_contract_id,
                    document_type_name,
                    start_position,
                    page,
                )
                .await
        }

        async fn scan_private_document_pages(
            &self,
            keys: &PrivateDocumentKeyPaths,
            data_contract_id: Identifier,
            document_type_name: &str,
            pages: &[(u64, &PrivateDocumentEntries)],
        ) -> Result<Vec<PrivateDocumentScan>, PlatformWalletError> {
            if let Some(hook) = &self.before_scan {
                hook().await;
            }
            self.inner
                .scan_private_document_pages(keys, data_contract_id, document_type_name, pages)
                .await
        }

        #[allow(clippy::too_many_arguments)]
        async fn seal_private_contact_request(
            &self,
            sender_encryption_key_path: &DerivationPath,
            dashpay_contract: &DataContract,
            sender: &Identity,
            sender_encryption_key_id: KeyID,
            signing_key_id: KeyID,
            signature: &[u8; 65],
            recipient: &Identity,
            recipient_key_id: KeyID,
            request: &PrivateContactRequest,
            esk_seed: &[u8; 32],
            platform_version: &PlatformVersion,
        ) -> Result<BinaryData, PlatformWalletError> {
            self.inner
                .seal_private_contact_request(
                    sender_encryption_key_path,
                    dashpay_contract,
                    sender,
                    sender_encryption_key_id,
                    signing_key_id,
                    signature,
                    recipient,
                    recipient_key_id,
                    request,
                    esk_seed,
                    platform_version,
                )
                .await
        }
    }

    /// A hook that, the `call`th time it runs (from 1), says so on `reached`
    /// and waits for `release`.
    fn pause_on_call(
        call: usize,
        reached: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) -> Hook {
        let runs = Arc::new(AtomicUsize::new(0));
        Arc::new(move || {
            let (reached, release, runs) = (
                Arc::clone(&reached),
                Arc::clone(&release),
                Arc::clone(&runs),
            );
            Box::pin(async move {
                if runs.fetch_add(1, Ordering::SeqCst) + 1 == call {
                    reached.notify_one();
                    release.notified().await;
                }
            })
        })
    }

    /// A public request that passed its channel check and is still being
    /// prepared holds the pair: a private request started meanwhile is
    /// refused before anything is paid, and only the public one goes on.
    /// Once it ends, the pair is free again.
    #[tokio::test]
    async fn should_let_only_one_channel_send_to_a_pair_at_a_time() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        let reached = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        // The second derivation: the first checks the provider's seed before
        // the store is read.
        let mut pausing = HookedCrypto::new(ALICE_PHRASE);
        pausing.before_receiving_xpub =
            Some(pause_on_call(2, Arc::clone(&reached), Arc::clone(&release)));
        let paid = Arc::new(Mutex::new(Vec::new()));

        let (alice_id, bob_id) = (alice.id(), bob.id());
        let public = view.send_contact_request_with_external_signer(
            &alice_id,
            &bob_id,
            None,
            AutoAcceptProofSource::None,
            &alice_signer,
            &pausing,
        );
        let private = async {
            reached.notified().await;
            let claimed = managed(&alice_wallet, ALICE, |managed| {
                managed.dashpay().public_contact_request_claimed(&bob.id())
            })
            .await;
            let sent = view
                .send_private_contact_request(
                    &alice.id(),
                    &bob.id(),
                    None,
                    AutoAcceptProofSource::None,
                    None,
                    &alice_signer,
                    &alice_crypto,
                    keep_entry(&paid),
                )
                .await;
            release.notify_one();
            (claimed, sent)
        };
        let (public, (claimed, private)) = tokio::join!(public, private);
        assert!(claimed, "the paused public send holds the pair");
        assert!(
            matches!(&private, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if *recipient == bob.id()),
            "{private:?}"
        );
        assert!(paid.lock().expect("paid").is_empty(), "nothing is paid");
        assert!(
            !matches!(
                &public,
                Err(PlatformWalletError::PrivateContactRequestPending(_)
                    | PlatformWalletError::PrivateContactRequestNeedsPrivateReply(_))
            ),
            "the public send goes on to its submission, which this mock refuses: {public:?}"
        );

        assert!(!pending_to(&alice_wallet, ALICE, &bob).await);
        managed(&alice_wallet, ALICE, |managed| {
            assert!(!managed.dashpay().public_contact_request_claimed(&bob.id()));
        })
        .await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        let after = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&kept),
            )
            .await;
        assert!(
            matches!(
                after,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "{after:?}"
        );
        assert_eq!(
            kept.lock().expect("kept").len(),
            1,
            "the pair is free again"
        );
    }

    /// `identity` without its key `key_id`, as a device that has not yet
    /// fetched a key another device added holds it.
    fn without_key(identity: &Identity, key_id: KeyID) -> Identity {
        Identity::V0(IdentityV0 {
            id: identity.id(),
            public_keys: identity
                .public_keys()
                .iter()
                .filter(|(id, _)| **id != key_id)
                .map(|(id, key)| (*id, key.clone()))
                .collect(),
            balance: identity.balance(),
            revision: identity.revision(),
        })
    }

    /// A store read with keys missing one an entry is sealed to is read again
    /// once a refresh brings the key: the refresh folds it in, the read
    /// position and the read-to-the-end flag are rewound, and the next sync
    /// records the request.
    #[tokio::test]
    async fn should_read_the_store_again_once_an_identity_gains_a_scanning_key() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let (entry, _) = alice_to_bob([0x55; 43]).await;
        let stale = without_key(&bob, DECRYPTION_KEY_ID);
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &stale, &[&alice, &bob], vec![entry]).await;
        let view = bob_wallet.identity().dashpay();

        let report = view
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store with his older keys");
        assert_eq!(
            report.requests,
            vec![],
            "the entry opens with no key he holds"
        );
        assert!(store_read(&bob_wallet, BOB).await);
        managed(&bob_wallet, BOB, |managed| {
            assert_eq!(managed.dashpay().private_contact_requests_position(), 1);
        })
        .await;

        bob_wallet
            .identity()
            .refresh_identity(&bob.id())
            .await
            .expect("the refresh folds in the decryption key");
        view.reconcile_private_scan_keys()
            .await
            .expect("reconciled");
        assert!(
            !store_read(&bob_wallet, BOB).await,
            "a send reads the store again"
        );
        managed(&bob_wallet, BOB, |managed| {
            assert_eq!(managed.dashpay().private_contact_requests_position(), 0);
        })
        .await;

        let report = view
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store again");
        assert_eq!(report.requests.len(), 1, "{:?}", report.requests);
        managed(&bob_wallet, BOB, |managed| {
            assert!(managed.dashpay().incoming_request(&alice.id()).is_some());
        })
        .await;
    }

    /// A pass whose provider resolves another wallet's seed, valid but
    /// deriving unrelated keys that open nothing, is refused with nothing
    /// read or recorded; the pass with the wallet's own seed then reads the
    /// request the refused one would have passed over.
    #[tokio::test]
    async fn should_refuse_a_pass_with_another_seed_and_leave_its_position_unmoved() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let (entry, _) = alice_to_bob([0x55; 43]).await;
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry]).await;
        let view = bob_wallet.identity().dashpay();

        let another_seed = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let refused = view.sync_private_contact_requests(&another_seed, 4).await;
        assert!(
            matches!(refused, Err(PlatformWalletError::SeedMismatch { .. })),
            "{refused:?}"
        );
        managed(&bob_wallet, BOB, |managed| {
            assert_eq!(
                managed.dashpay().private_contact_requests_position(),
                0,
                "nothing recorded as read"
            );
            assert!(!managed.dashpay().private_contact_requests_read());
            assert!(managed.dashpay().incoming_request(&alice.id()).is_none());
        })
        .await;

        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        view.sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("the pass with Bob's seed");
        managed(&bob_wallet, BOB, |managed| {
            assert!(managed.dashpay().incoming_request(&alice.id()).is_some());
            assert_eq!(managed.dashpay().private_contact_requests_position(), 1);
        })
        .await;
    }

    /// A pass that started with older keys commits no progress once a key
    /// update lands under it, even when the rewind that update brings is
    /// taken while the pass runs: the next pass reads with the new keys from
    /// where the update left the position.
    #[tokio::test]
    async fn should_not_advance_over_a_key_update_during_a_pass() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let (entry, _) = alice_to_bob([0x55; 43]).await;
        let stale = without_key(&bob, DECRYPTION_KEY_ID);
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &stale, &[&alice, &bob], vec![entry]).await;

        let mut updating = HookedCrypto::new(BOB_PHRASE);
        updating.before_scan = Some({
            let bob_wallet = Arc::clone(&bob_wallet);
            let bob = bob.clone();
            Arc::new(move || {
                let (bob_wallet, bob) = (Arc::clone(&bob_wallet), bob.clone());
                Box::pin(async move {
                    let iw = bob_wallet.identity();
                    let mut wm = iw.wallet_manager.write().await;
                    let managed = wm
                        .get_wallet_info_mut(&bob_wallet.wallet_id())
                        .expect("wallet info")
                        .identity_manager
                        .managed_identity_mut(&bob.id())
                        .expect("Bob");
                    // A refresh's key, and a read before a send rewinding for it.
                    managed.identity = bob;
                    managed.reconcile_private_scan_keys();
                })
            })
        });
        bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&updating, 4)
            .await
            .expect("the pass with older keys");
        managed(&bob_wallet, BOB, |managed| {
            assert_eq!(
                managed.dashpay().private_contact_requests_position(),
                0,
                "no progress over the update"
            );
            assert!(!managed.dashpay().private_contact_requests_read());
            assert!(managed.dashpay().incoming_request(&alice.id()).is_none());
        })
        .await;

        bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("the pass with the new keys");
        managed(&bob_wallet, BOB, |managed| {
            assert!(managed.dashpay().incoming_request(&alice.id()).is_some());
            assert_eq!(managed.dashpay().private_contact_requests_position(), 1);
        })
        .await;
    }

    /// A reply dropped while its payment source is still being resolved, as
    /// a host's prompt left open and abandoned, keeps no reservation:
    /// nothing was paid, and a retry goes on to pay.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_release_a_reply_dropped_while_its_payment_is_resolved() {
        use crate::wallet::platform_wallet::PrivateReplyPayment;

        struct NoProver;
        impl dpp::shielded::builder::OrchardProver for NoProver {
            fn proving_key(&self) -> &grovedb_commitment_tree::ProvingKey {
                panic!("nothing is proved")
            }
        }

        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let (entry, received) = alice_to_bob([0x55; 43]).await;
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry]).await;
        bob_wallet
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");

        let dropped = tokio::time::timeout(
            Duration::from_millis(200),
            bob_wallet.accept_contact_request_replying_in_kind(
                &received,
                None,
                &bob_signer,
                &bob_crypto,
                || {
                    std::future::pending::<
                        Result<PrivateReplyPayment<NoProver, Vec<u8>>, PlatformWalletError>,
                    >()
                },
            ),
        )
        .await;
        assert!(dropped.is_err(), "the payment source never resolves");
        assert!(!pending_to(&bob_wallet, BOB, &alice).await);

        let reply = Arc::new(Mutex::new(Vec::new()));
        let retried = bob_wallet
            .identity()
            .dashpay()
            .accept_private_contact_request(
                &received,
                None,
                &bob_signer,
                &bob_crypto,
                keep_entry(&reply),
            )
            .await;
        assert!(
            matches!(
                retried,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "paid, not read back: {retried:?}"
        );
        assert_eq!(reply.lock().expect("reply").len(), 1, "the retry pays");
    }

    /// A private request whose payment was unconfirmed, and which the
    /// shielded coordinator then settled as never executing (a definitive
    /// consensus rejection of its saved transition), frees its pair without
    /// a restart: the retry pays, even after a release was dropped while it
    /// waited for the wallet lock.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_free_a_pair_once_its_unconfirmed_payment_is_rejected_for_good() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        let kept = Arc::new(Mutex::new(Vec::new()));
        let unconfirmed = {
            let kept = Arc::clone(&kept);
            move |entry: BinaryData, started: PaymentStarted| {
                started.mark();
                kept.lock().expect("kept").push(entry);
                std::future::ready(Err::<ResponseMetadata, _>(
                    PlatformWalletError::ShieldedSpendUnconfirmed {
                        operation: "private document create",
                        reason: "the proof fetch timed out".to_string(),
                    },
                ))
            }
        };
        let sent = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                unconfirmed,
            )
            .await;
        assert!(
            matches!(
                sent,
                Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
            ),
            "{sent:?}"
        );
        assert!(pending_to(&alice_wallet, ALICE, &bob).await);
        let entry = kept.lock().expect("kept").remove(0);

        // The redrive pass's definitive rejection of the saved transition.
        note_rejected_private_documents(
            &private_contact_request_payment(entry),
            Some(&alice_wallet.identity().persister),
        );

        // A release dropped while it waits for the wallet lock keeps the
        // notice for the next call.
        {
            let wm = alice_wallet.identity().wallet_manager.write().await;
            let release = view.release_rejected_private_requests();
            assert!(
                tokio::time::timeout(Duration::from_millis(50), release)
                    .await
                    .is_err(),
                "the release waits for the wallet lock"
            );
            drop(wm);
        }
        assert!(pending_to(&alice_wallet, ALICE, &bob).await);

        let retry = Arc::new(Mutex::new(Vec::new()));
        let retried = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&retry),
            )
            .await;
        assert!(
            matches!(
                retried,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "paid, not read back: {retried:?}"
        );
        assert_eq!(retry.lock().expect("retry").len(), 1, "the retry pays");
    }

    /// A restoration that opened an unconfirmed request's entry before the
    /// shielded coordinator settled its payment as never executing, and that
    /// resumes only after a send's reconciliation took the rejection, reserves
    /// nothing: the rejection outlives that take for the session. Restoring
    /// from the payment's record afterwards reserves nothing either, and a
    /// retry pays.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_not_restore_a_request_rejected_for_good_while_its_restore_scanned() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        let unconfirmed = {
            let kept = Arc::clone(&kept);
            move |entry: BinaryData, started: PaymentStarted| {
                started.mark();
                kept.lock().expect("kept").push(entry);
                std::future::ready(Err::<ResponseMetadata, _>(
                    PlatformWalletError::ShieldedSpendUnconfirmed {
                        operation: "private document create",
                        reason: "the proof fetch timed out".to_string(),
                    },
                ))
            }
        };
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
                unconfirmed,
            )
            .await;
        assert!(
            matches!(
                sent,
                Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
            ),
            "{sent:?}"
        );
        let entry = kept.lock().expect("kept").remove(0);
        let commitment: [u8; 32] = entry.as_slice()[..32]
            .try_into()
            .expect("an entry starts with its commitment");

        // Restarted: nothing reserved until a restoration runs.
        let (_restarted_manager, restarted) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        assert!(!pending_to(&restarted, ALICE, &bob).await);
        let view = restarted.identity().dashpay();
        let persister = &restarted.identity().persister;

        let scanning = Arc::new(tokio::sync::Notify::new());
        let resume = Arc::new(tokio::sync::Notify::new());
        let mut pausing = HookedCrypto::new(ALICE_PHRASE);
        pausing.before_page_scan = Some({
            let (scanning, resume) = (Arc::clone(&scanning), Arc::clone(&resume));
            Arc::new(move || {
                let (scanning, resume) = (Arc::clone(&scanning), Arc::clone(&resume));
                Box::pin(async move {
                    scanning.notify_one();
                    resume.notified().await;
                })
            })
        });
        let restore =
            view.restore_pending_private_contact_requests(std::slice::from_ref(&entry), &pausing);
        let reject = async {
            scanning.notified().await;
            // The redrive pass's definitive rejection of the saved transition,
            // then a send's reconciliation, with nothing restored to release.
            note_rejected_private_documents(
                &private_contact_request_payment(entry.clone()),
                Some(persister),
            );
            view.release_rejected_private_requests()
                .await
                .expect("the release");
            assert!(
                !persister.has_rejected_private_entries(),
                "the rejection was taken"
            );
            resume.notify_one();
        };
        let (restored, ()) = tokio::join!(restore, reject);
        assert_eq!(restored.expect("the restoration"), vec![]);
        assert!(!pending_to(&restarted, ALICE, &bob).await);

        let from_record = view
            .restore_private_payments(&[(alice.id(), bob.id(), 0, commitment)])
            .await
            .expect("the restoration from the record");
        assert_eq!(from_record, vec![]);
        assert!(!pending_to(&restarted, ALICE, &bob).await);

        let retry = Arc::new(Mutex::new(Vec::new()));
        let retried = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                keep_entry(&retry),
            )
            .await;
        assert!(
            matches!(
                retried,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "paid, not read back: {retried:?}"
        );
        assert_eq!(retry.lock().expect("retry").len(), 1, "the retry pays");
    }

    /// The type 29 transition paying for `entry`, with placeholder actions:
    /// as the shielded store keeps a payment's signed transition.
    #[cfg(feature = "shielded")]
    fn private_contact_request_payment(
        entry: BinaryData,
    ) -> dpp::state_transition::StateTransition {
        use dpp::shielded::SerializedAction;
        use dpp::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
        use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;

        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
            dashpay_contract().expect("bundled DashPay").id(),
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME.to_string(),
            vec![entry],
            vec![action.clone(), action],
            0,
            [7; 32],
            vec![8; 64],
            [9; 64],
            PlatformVersion::latest(),
        )
        .expect("a type 29 transition")
    }

    /// A process that exits after its private payment was admitted, during
    /// the wait for its result, still leaves the payment's record, saved
    /// before the broadcast: a restarted wallet restores the request's
    /// reservation from it, so neither a second private request nor a
    /// public one goes to the recipient.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_restore_a_request_whose_process_exited_during_its_result_wait() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let db_path = dir.path().join("tree.sqlite");
        let coordinator_at = |db_path: std::path::PathBuf| {
            let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open the store");
            Arc::new(NetworkShieldedCoordinator::new(
                Arc::new(dash_sdk::Sdk::new_mock()),
                Network::Testnet,
                db_path,
                store,
            ))
        };

        let before = coordinator_at(db_path.clone());
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let subwallet = SubwalletId::new(alice_wallet.wallet_id(), 0);
        let pay = {
            let store = Arc::clone(before.store());
            move |entry: BinaryData, started: PaymentStarted| async move {
                // As the shielded payment does: the record saved, marked,
                // broadcast and admitted; then the result never comes.
                let transition = private_contact_request_payment(entry);
                arm_redrive_before_broadcast(
                    &store,
                    subwallet,
                    Some([0x77; 32]),
                    [7; 32],
                    vec![[1; 32]],
                    &transition,
                    "private document create from shielded pool",
                    None,
                    Some(&started),
                )
                .await?;
                std::future::pending::<()>().await;
                Ok(ResponseMetadata::default())
            }
        };
        let exited = tokio::time::timeout(
            Duration::from_millis(200),
            alice_wallet
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
                    pay,
                ),
        )
        .await;
        assert!(exited.is_err(), "the process exits during the wait");
        drop(before);

        let after = coordinator_at(db_path);
        let (_restarted_manager, restarted) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        assert_eq!(restarted.wallet_id(), alice_wallet.wallet_id());
        assert!(!pending_to(&restarted, ALICE, &bob).await);
        let restored = restarted
            .restore_pending_private_contact_requests(&after, &alice_crypto)
            .await
            .expect("restored from the saved record");
        assert_eq!(restored, vec![(alice.id(), bob.id())]);
        assert!(pending_to(&restarted, ALICE, &bob).await);
        let public = restarted
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
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "{public:?}"
        );
    }

    /// Two accepts of one private request straddling the first reply's
    /// settlement: the second read that no reply was sent before the first
    /// ran, and reaches its reply after the first's was recorded and its
    /// reservation settled. It adopts that reply, and its payment never runs.
    #[tokio::test]
    async fn should_adopt_a_reply_another_accept_recorded_in_between() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let (entry, received) = alice_to_bob([0x55; 43]).await;
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry]).await;
        let view = bob_wallet.identity().dashpay();
        view.sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");
        let first_done = Arc::new(tokio::sync::Notify::new());
        let reply = Arc::new(Mutex::new(Vec::new()));

        let second = view.accept_contact_request_replying_with(&received, &bob_crypto, || {
            let first_done = Arc::clone(&first_done);
            let (received, bob_signer, bob_crypto) = (&received, &bob_signer, &bob_crypto);
            async move {
                first_done.notified().await;
                view.send_private_reply(received, None, bob_signer, bob_crypto, no_payment)
                    .await
            }
        });
        let first = async {
            let accepted = view
                .accept_private_contact_request(
                    &received,
                    None,
                    &bob_signer,
                    &bob_crypto,
                    append_entry(&bob_wallet, 1, vec![], &reply),
                )
                .await;
            first_done.notify_one();
            accepted
        };
        let (second, first) = tokio::join!(second, first);
        let first = first.expect("the first reply is paid and recorded");
        let second = second.expect("the second accept adopts it");
        assert_eq!(second.outgoing_request, first.outgoing_request);
        assert_eq!(reply.lock().expect("reply").len(), 1, "one reply is paid");
        assert!(!pending_to(&bob_wallet, BOB, &alice).await);
    }

    /// A `pay` that must not run.
    fn no_payment(
        _: BinaryData,
        _: PaymentStarted,
    ) -> std::future::Ready<Result<ResponseMetadata, PlatformWalletError>> {
        panic!("nothing is paid")
    }

    /// Whether `wallet`'s identity `identity_id` read the private store to its
    /// end since load.
    async fn store_read(wallet: &PlatformWallet, identity_id: [u8; 32]) -> bool {
        managed(wallet, identity_id, |managed| {
            managed.dashpay().private_contact_requests_read()
        })
        .await
    }

    /// A private reply paid for and confirmed but not read back leaves
    /// nothing that survives a restart. After one, with Alice's request
    /// already known (as a sync that stopped between the two leaves it), the
    /// accept a host calls reads the store first, finds the reply, and adopts
    /// it: nothing is paid twice.
    #[tokio::test]
    async fn should_adopt_a_private_reply_paid_for_before_a_restart() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let (_alice_manager, alice_wallet) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        alice_wallet
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
        let request_entry = kept.lock().expect("kept").remove(0);

        let (_bob_manager, before) = wallet(
            BOB_PHRASE,
            &bob,
            &[&alice, &bob],
            vec![request_entry.to_vec()],
        )
        .await;
        before
            .identity()
            .dashpay()
            .sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("Bob reads the store");
        let received = managed(&before, BOB, |managed| {
            managed.dashpay().incoming_request(&alice.id()).cloned()
        })
        .await
        .expect("Bob holds Alice's request");
        let reply = Arc::new(Mutex::new(Vec::new()));
        let paid = before
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &received,
                None,
                &bob_signer,
                &bob_crypto,
                keep_entry(&reply),
            )
            .await;
        assert!(
            matches!(
                paid,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "paid and confirmed, not read back: {paid:?}"
        );
        let reply_entry = reply.lock().expect("reply").remove(0);

        // The restart: the reply is in the store, the reservation is gone.
        let (_restarted_manager, restarted) = wallet(
            BOB_PHRASE,
            &bob,
            &[&alice, &bob],
            vec![request_entry.to_vec(), reply_entry.to_vec()],
        )
        .await;
        {
            let iw = restarted.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&restarted.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&bob.id())
                .expect("Bob")
                .add_incoming_contact_request(received.clone(), &iw.persister)
                .expect("Alice's request");
        }
        assert!(!store_read(&restarted, BOB).await);
        let established = restarted
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &received,
                None,
                &bob_signer,
                &bob_crypto,
                no_payment,
            )
            .await
            .expect("the reply read back is adopted");
        assert_eq!(established.contact_identity_id, alice.id());
        assert_eq!(established.incoming_request, received);
        assert!(established.outgoing_request.came_privately());
        assert!(store_read(&restarted, BOB).await);
        assert!(!pending_to(&restarted, BOB, &alice).await);
    }

    /// Track `request` as an incoming request of `wallet`'s identity
    /// `identity_id`, as a restarted wallet restores the public ones.
    async fn add_incoming(
        wallet: &PlatformWallet,
        identity_id: [u8; 32],
        request: &ContactRequest,
    ) {
        let iw = wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .expect("wallet info")
            .identity_manager
            .managed_identity_mut(&Identifier::from(identity_id))
            .expect("managed identity")
            .add_incoming_contact_request(request.clone(), &iw.persister)
            .expect("incoming request");
    }

    /// A private reply to a public request, paid for and not read back,
    /// leaves nothing that survives a restart. After one, before any sync,
    /// a public send to the contact and the accept a host calls each read
    /// the store first and find the reply: the send is refused, as the
    /// pair's requests went privately, and the accept adopts the reply.
    /// Nothing is broadcast or paid again.
    #[tokio::test]
    async fn should_not_reply_publicly_after_a_restart_with_a_private_reply_unread() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let incoming = public_request(&alice, &bob, 3);
        let (_before_manager, before) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![]).await;
        add_incoming(&before, BOB, &incoming).await;
        let reply = Arc::new(Mutex::new(Vec::new()));
        let paid = before
            .identity()
            .dashpay()
            .accept_private_contact_request(
                &incoming,
                None,
                &bob_signer,
                &bob_crypto,
                keep_entry(&reply),
            )
            .await;
        assert!(
            matches!(
                paid,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "paid and confirmed, not read back: {paid:?}"
        );
        let reply_entry = reply.lock().expect("reply").remove(0).to_vec();

        // Restarted with the reply in the store: a public send first.
        let (_sending_manager, sending) =
            wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![reply_entry.clone()]).await;
        add_incoming(&sending, BOB, &incoming).await;
        let public = sending
            .identity()
            .dashpay()
            .send_contact_request_with_external_signer(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                &bob_signer,
                &bob_crypto,
            )
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactRequestNeedsPrivateReply(contact))
                    if contact == alice.id()
            ),
            "{public:?}"
        );

        // Restarted again: the accept a host calls first.
        let (_accepting_manager, accepting) =
            wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![reply_entry]).await;
        add_incoming(&accepting, BOB, &incoming).await;
        let established = accepting
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &incoming,
                None,
                &bob_signer,
                &bob_crypto,
                no_payment,
            )
            .await
            .expect("the private reply read back is adopted");
        assert_eq!(established.contact_identity_id, alice.id());
        assert_eq!(established.incoming_request, incoming);
        assert!(established.outgoing_request.came_privately());
    }

    /// After a restart, before any sync, the accept a host calls reads the
    /// store before it chooses the reply: a public request since followed by
    /// a private one from the same sender, unread, takes a private reply,
    /// where choosing from the public request alone would pick a public one
    /// and refuse it once the store is read.
    #[tokio::test]
    async fn should_reply_privately_in_kind_once_a_restart_reads_a_private_request() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let public = public_request(&alice, &bob, 3);

        // Alice's public request, then a private one to Bob.
        let (_alice_manager, alice_wallet) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        {
            let iw = alice_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&alice_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&alice.id())
                .expect("Alice")
                .add_sent_contact_request(public.clone(), &iw.persister)
                .expect("the public request");
        }
        let kept = Arc::new(Mutex::new(Vec::new()));
        alice_wallet
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
            .expect("Alice sends the private request");
        let private_entry = kept.lock().expect("kept").remove(0).to_vec();

        // Bob, restarted, holds the public request; the private one is in the
        // store, unread.
        let (_bob_manager, bob_wallet) =
            wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![private_entry]).await;
        add_incoming(&bob_wallet, BOB, &public).await;
        let reply = Arc::new(Mutex::new(Vec::new()));
        let accepted = bob_wallet
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &public,
                None,
                &bob_signer,
                &bob_crypto,
                keep_entry(&reply),
            )
            .await;
        assert!(
            matches!(
                accepted,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "a private reply, paid and not read back: {accepted:?}"
        );
        assert_eq!(reply.lock().expect("reply").len(), 1, "one reply is paid");
    }

    /// A private reply to a public request whose payment is unconfirmed when
    /// the process ends is saved with the request it pays for: binding the
    /// restarted wallet to its shielded coordinator reserves the pair again,
    /// with no host call and no keys. Before any sync, neither the accept a
    /// host calls nor a public send then goes to the contact. Before the
    /// bind, the relaunched wallet, which bound its shielded account before,
    /// refuses both too, with nothing sent: the reservation is not restored
    /// yet.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_not_reply_publicly_after_a_restart_with_a_private_reply_unconfirmed() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let incoming = public_request(&alice, &bob, 3);
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let db_path = dir.path().join("tree.sqlite");
        let coordinator_at = |db_path: std::path::PathBuf| {
            let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open the store");
            Arc::new(NetworkShieldedCoordinator::new(
                Arc::new(dash_sdk::Sdk::new_mock()),
                Network::Testnet,
                db_path,
                store,
            ))
        };

        let before_coordinator = coordinator_at(db_path.clone());
        let (_before_manager, before) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![]).await;
        add_incoming(&before, BOB, &incoming).await;
        // What the backend keeps of the wallet, before the reply is paid.
        let kept = Arc::new(snapshot(&before, &bob.id(), BOB_PHRASE).await);
        let subwallet = SubwalletId::new(before.wallet_id(), 0);
        let pay = {
            let store = Arc::clone(before_coordinator.store());
            let (sender, recipient) = (bob.id(), alice.id());
            move |entry: BinaryData, started: PaymentStarted| async move {
                // As the shielded payment does: the record saved with the
                // request it pays for, marked, broadcast; its result unknown.
                let transition = private_contact_request_payment(entry);
                let payment = PrivateContactPayment {
                    sender: sender.to_buffer(),
                    recipient: recipient.to_buffer(),
                    account_reference: started
                        .account_reference()
                        .expect("the send names its request"),
                };
                arm_redrive_before_broadcast(
                    &store,
                    subwallet,
                    Some([0x77; 32]),
                    [7; 32],
                    vec![[1; 32]],
                    &transition,
                    "private document create from shielded pool",
                    Some(payment),
                    Some(&started),
                )
                .await?;
                Err::<ResponseMetadata, _>(PlatformWalletError::ShieldedSpendUnconfirmed {
                    operation: "private document create",
                    reason: "the result wait timed out".to_string(),
                })
            }
        };
        let paid = before
            .identity()
            .dashpay()
            .accept_private_contact_request(&incoming, None, &bob_signer, &bob_crypto, pay)
            .await;
        assert!(
            matches!(
                paid,
                Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
            ),
            "{paid:?}"
        );
        drop(before_coordinator);

        // The restart: the reply is not in the store and the host restores
        // nothing; only the bind can.
        let after = coordinator_at(db_path);
        let restarted_manager = PlatformWalletManager::new(
            Arc::new(dashpay_sdk(&[&alice, &bob], vec![], PlatformVersion::latest(), None).await),
            kept,
            Arc::new(NoopEventHandler),
        );
        restarted_manager
            .load_from_persistor()
            .await
            .expect("the relaunch loads");
        let restarted = restarted_manager
            .get_wallet(&before.wallet_id())
            .await
            .expect("the relaunched wallet");
        assert!(!pending_to(&restarted, BOB, &alice).await);

        // Before the bind: refused, with nothing read or sent.
        let accepted = restarted
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &incoming,
                None,
                &bob_signer,
                &bob_crypto,
                no_payment,
            )
            .await;
        assert!(
            matches!(
                accepted,
                Err(PlatformWalletError::PrivateContactPaymentsNotRestored)
            ),
            "{accepted:?}"
        );
        let public = restarted
            .identity()
            .dashpay()
            .send_contact_request_with_external_signer(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                &bob_signer,
                &bob_crypto,
            )
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactPaymentsNotRestored)
            ),
            "{public:?}"
        );

        restarted
            .bind_shielded(&seed(BOB_PHRASE), &[0], &after)
            .await
            .expect("bound");
        assert!(
            pending_to(&restarted, BOB, &alice).await,
            "the bind restores the reservation"
        );

        let accepted = restarted
            .identity()
            .dashpay()
            .accept_contact_request_replying_in_kind(
                &incoming,
                None,
                &bob_signer,
                &bob_crypto,
                no_payment,
            )
            .await;
        assert!(
            matches!(
                accepted,
                Err(PlatformWalletError::PrivateContactRequestPending(contact))
                    if contact == alice.id()
            ),
            "{accepted:?}"
        );
        let public = restarted
            .identity()
            .dashpay()
            .send_contact_request_with_external_signer(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                &bob_signer,
                &bob_crypto,
            )
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactRequestPending(contact))
                    if contact == alice.id()
            ),
            "{public:?}"
        );
    }

    /// A payment saved with its request is restored only while neither that
    /// request nor a later one is recorded: a request read back before the
    /// restart discharges it, a rotation past the tracked request does not,
    /// and a sender that is not the wallet's identity is skipped.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_restore_a_saved_payment_only_while_its_request_is_unrecorded() {
        use platform_encryption::ACCOUNT_REFERENCE_VERSION_SHIFT;

        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![]).await;
        let reference = 5;
        let rotated = reference | (1 << ACCOUNT_REFERENCE_VERSION_SHIFT);
        {
            let iw = bob_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&bob_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&bob.id())
                .expect("Bob")
                .add_sent_contact_request(public_request(&bob, &alice, reference), &iw.persister)
                .expect("sent request");
        }
        let view = bob_wallet.identity().dashpay();

        let recorded = view
            .restore_private_payments(&[
                (bob.id(), alice.id(), reference, [9; 32]),
                (alice.id(), bob.id(), reference, [8; 32]),
            ])
            .await
            .expect("restore");
        assert!(recorded.is_empty(), "{recorded:?}");
        assert!(!pending_to(&bob_wallet, BOB, &alice).await);

        let rotation = view
            .restore_private_payments(&[(bob.id(), alice.id(), rotated, [9; 32])])
            .await
            .expect("restore");
        assert_eq!(rotation, vec![(bob.id(), alice.id())]);
        assert!(pending_to(&bob_wallet, BOB, &alice).await);
    }

    /// On a network a verified answer shows below protocol version 15 there
    /// is no private store to read: a public send goes on to its broadcast,
    /// as it did before private contact requests, rather than being refused
    /// for a read that cannot run.
    #[tokio::test]
    async fn should_send_publicly_on_a_network_below_protocol_version_15() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let (_manager, bob_wallet) = wallet_at(
            Arc::new(NoPersistence),
            BOB_PHRASE,
            &bob,
            &[&alice, &bob],
            vec![],
            PlatformVersion::get(14).expect("protocol version 14"),
            Some(14),
        )
        .await;
        let sent = bob_wallet
            .identity()
            .dashpay()
            .send_contact_request_with_external_signer(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                &bob_signer,
                &bob_crypto,
            )
            .await;
        // Past the read and the claim: the transition is being built, and
        // this mock does not answer its nonce fetch.
        assert!(
            matches!(&sent, Err(PlatformWalletError::Sdk(error))
                if error.to_string().contains("GetIdentityContractNonceRequest")),
            "the send goes on to its transition: {sent:?}"
        );
        assert!(!store_read(&bob_wallet, BOB).await, "nothing was read");
    }

    /// An SDK pinned below protocol version 15 refuses to read the private
    /// store whatever the network runs. On a network a verified answer shows
    /// at 15, or one whose version is not verified, the store may hold a
    /// private request, so a public send is refused with that refusal and
    /// sends nothing.
    #[tokio::test]
    async fn should_refuse_a_public_send_from_an_sdk_pinned_below_a_network_at_15() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        for verified_network in [Some(15), None] {
            let (_manager, bob_wallet) = wallet_at(
                Arc::new(NoPersistence),
                BOB_PHRASE,
                &bob,
                &[&alice, &bob],
                vec![],
                PlatformVersion::get(14).expect("protocol version 14"),
                verified_network,
            )
            .await;
            let sent = bob_wallet
                .identity()
                .dashpay()
                .send_contact_request_with_external_signer(
                    &bob.id(),
                    &alice.id(),
                    None,
                    AutoAcceptProofSource::None,
                    &bob_signer,
                    &bob_crypto,
                )
                .await;
            assert!(
                matches!(
                    &sent,
                    Err(PlatformWalletError::Sdk(
                        dash_sdk::Error::ProtocolVersionNotActive { .. }
                    ))
                ),
                "verified {verified_network:?}: {sent:?}"
            );
        }
    }

    /// A public request's claim is released only by its own attempt: a send
    /// that outlived its wallet's removal and re-creation under the same id,
    /// dropped once the new wallet's send claimed the same pair, leaves that
    /// claim in place, and the new send's own release frees the pair.
    #[tokio::test]
    async fn should_release_only_its_own_claim_across_a_wallet_re_creation() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let (manager, old_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let old_claim = old_wallet
            .identity()
            .dashpay()
            .claim_public_pair(&alice.id(), &bob.id(), false)
            .await
            .expect("claimed")
            .expect("a claim");

        manager
            .remove_wallet(&old_wallet.wallet_id())
            .await
            .expect("removed");
        let new_wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed(ALICE_PHRASE),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("re-created");
        assert_eq!(new_wallet.wallet_id(), old_wallet.wallet_id());
        {
            let iw = new_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&new_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .add_identity(alice.clone(), 0, new_wallet.wallet_id(), &iw.persister)
                .expect("own identity");
        }
        let new_claim = new_wallet
            .identity()
            .dashpay()
            .claim_public_pair(&alice.id(), &bob.id(), false)
            .await
            .expect("claimed")
            .expect("a claim");
        let claimed = || async {
            managed(&new_wallet, ALICE, |managed| {
                managed.dashpay().public_contact_request_claimed(&bob.id())
            })
            .await
        };

        drop(old_claim);
        assert!(
            claimed().await,
            "the old send's release leaves the new claim"
        );
        drop(new_claim);
        assert!(!claimed().await, "the new send's release frees the pair");
    }

    /// Unignoring a sender rewinds the private read and clears its
    /// readiness: a pass that read the store with the sender ignored passed
    /// over its private request, so a public send to it reads the store
    /// again first, finds the request, and is refused, publishing nothing.
    #[tokio::test]
    async fn should_read_the_store_again_before_a_public_send_once_a_sender_is_unignored() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let (entry, _) = alice_to_bob([0x55; 43]).await;
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![entry]).await;
        let view = bob_wallet.identity().dashpay();
        view.ignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("ignored");
        view.sync_private_contact_requests(&bob_crypto, 4)
            .await
            .expect("the pass");
        assert!(store_read(&bob_wallet, BOB).await);
        assert!(
            managed(&bob_wallet, BOB, |managed| {
                managed.dashpay().incoming_request(&alice.id()).is_none()
            })
            .await,
            "passed over while ignored"
        );

        view.unignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("unignored");
        assert!(!store_read(&bob_wallet, BOB).await, "read again first");
        let public = view
            .send_contact_request_with_external_signer(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                &bob_signer,
                &bob_crypto,
            )
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactRequestNeedsPrivateReply(contact))
                    if contact == alice.id()
            ),
            "{public:?}"
        );
    }

    /// A sender unignored while a pass runs, after the pass passed over its
    /// private request: the pass reaches the store's end without taking the
    /// store as read, so a public send to the sender reads it again first,
    /// finds the request, and is refused, publishing nothing.
    #[tokio::test]
    async fn should_not_take_the_store_as_read_by_a_pass_a_sender_was_unignored_under() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let bob_crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);
        let bob_signer = signer(BOB_PHRASE, &bob);
        let (entry, _) = alice_to_bob([0x55; 43]).await;
        // Alice's request first, then pages enough for a second scan batch.
        let mut entries = vec![entry];
        entries.extend((1..=PRIVATE_SCAN_BATCH_PAGES as u8).map(|byte| vec![byte; 589]));
        let (_manager, bob_wallet) = wallet(BOB_PHRASE, &bob, &[&alice, &bob], vec![]).await;
        publish_chunked(&bob_wallet, &entries, 1).await;
        let view = bob_wallet.identity().dashpay();
        view.ignore_contact_sender(&bob.id(), &alice.id())
            .await
            .expect("ignored");

        // Unignored before the pass scans its second batch, once the first,
        // holding Alice's request, was ingested with her ignored.
        let mut unignoring = HookedCrypto::new(BOB_PHRASE);
        unignoring.before_scan = Some({
            let bob_wallet = Arc::clone(&bob_wallet);
            let scans = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let (bob_id, alice_id) = (bob.id(), alice.id());
            Arc::new(move || {
                let (bob_wallet, scans) = (Arc::clone(&bob_wallet), Arc::clone(&scans));
                Box::pin(async move {
                    if scans.fetch_add(1, Ordering::SeqCst) == 1 {
                        bob_wallet
                            .identity()
                            .dashpay()
                            .unignore_contact_sender(&bob_id, &alice_id)
                            .await
                            .expect("unignored");
                    }
                })
            })
        });
        view.sync_private_contact_requests(&unignoring, 64)
            .await
            .expect("the pass");
        assert!(
            !store_read(&bob_wallet, BOB).await,
            "not read for a sender unignored under the pass"
        );

        let public = view
            .send_contact_request_with_external_signer(
                &bob.id(),
                &alice.id(),
                None,
                AutoAcceptProofSource::None,
                &bob_signer,
                &bob_crypto,
            )
            .await;
        assert!(
            matches!(
                public,
                Err(PlatformWalletError::PrivateContactRequestNeedsPrivateReply(contact))
                    if contact == alice.id()
            ),
            "{public:?}"
        );
    }

    /// The first private send since load reads the store first. While that
    /// read cannot finish, here because the recipient of a request the store
    /// holds from Alice cannot be read, it is refused with nothing sealed or
    /// paid; a sync error comes back as it is; once the store reads, the send
    /// goes on.
    #[tokio::test]
    async fn should_refuse_a_private_send_until_the_store_reads_to_its_end() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, before) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let kept = Arc::new(Mutex::new(Vec::new()));
        before
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
                keep_entry(&kept),
            )
            .await
            .expect_err("not read back");
        let entry = kept.lock().expect("kept").remove(0);

        let (_restarted_manager, restarted) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![entry.to_vec()]).await;
        let view = restarted.identity().dashpay();
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

        set_identity_known(&restarted, &bob, false).await;
        let held = send!(no_payment);
        assert!(
            matches!(held, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == bob.id()),
            "{held:?}"
        );
        assert!(!store_read(&restarted, ALICE).await);
        assert!(
            !pending_to(&restarted, ALICE, &bob).await,
            "nothing reserved"
        );

        let contract = dashpay_contract().expect("bundled DashPay");
        let page = PrivateDocumentEntriesQuery::new(
            contract.id(),
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            0,
        );
        {
            let mut sdk = (*restarted.identity().sdk).clone();
            sdk.mock()
                .remove_fetch_expectation::<PrivateDocumentEntries, _>(page)
                .await;
        }
        let failed = send!(no_payment);
        assert!(
            matches!(failed, Err(PlatformWalletError::Sdk(_))),
            "{failed:?}"
        );

        publish(&restarted, 0, vec![entry.to_vec()]).await;
        set_identity_known(&restarted, &bob, true).await;
        let paid = Arc::new(Mutex::new(Vec::new()));
        let sent = send!(keep_entry(&paid));
        assert!(
            matches!(
                sent,
                Err(PlatformWalletError::PrivateContactRequestPending(_))
            ),
            "paid, not read back: {sent:?}"
        );
        assert_eq!(paid.lock().expect("paid").len(), 1, "the send goes on");
        assert!(store_read(&restarted, ALICE).await);
        managed(&restarted, ALICE, |managed| {
            assert!(
                managed.dashpay().outgoing_request(&bob.id()).is_some(),
                "the request paid for before the restart is recorded first"
            );
        })
        .await;
    }

    /// Once the store was read to its end, a private send reads nothing more
    /// before it pays: here a page read would fail, and the send still pays.
    #[tokio::test]
    async fn should_read_the_private_store_once_since_load() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let carol = identity(BOB_PHRASE, [0xC1; 32]);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) =
            wallet(ALICE_PHRASE, &alice, &[&alice, &bob, &carol], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        macro_rules! send {
            ($recipient:expr, $kept:expr) => {
                view.send_private_contact_request(
                    &alice.id(),
                    &$recipient,
                    None,
                    AutoAcceptProofSource::None,
                    None,
                    &alice_signer,
                    &alice_crypto,
                    keep_entry($kept),
                )
                .await
            };
        }
        assert!(!store_read(&alice_wallet, ALICE).await);
        let read_when_paid = Arc::new(AtomicBool::new(false));
        view.send_private_contact_request(
            &alice.id(),
            &bob.id(),
            None,
            AutoAcceptProofSource::None,
            None,
            &alice_signer,
            &alice_crypto,
            |_: BinaryData, started: PaymentStarted| {
                started.mark();
                let read_when_paid = Arc::clone(&read_when_paid);
                let wallet = &alice_wallet;
                async move {
                    read_when_paid.store(store_read(wallet, ALICE).await, Ordering::SeqCst);
                    Ok(ResponseMetadata {
                        time_ms: BLOCK_TIME_MS,
                        ..Default::default()
                    })
                }
            },
        )
        .await
        .expect_err("not read back");
        assert!(
            read_when_paid.load(Ordering::SeqCst),
            "the store was read before the first payment"
        );

        let contract = dashpay_contract().expect("bundled DashPay");
        {
            let mut sdk = (*alice_wallet.identity().sdk).clone();
            sdk.mock()
                .remove_fetch_expectation::<PrivateDocumentEntries, _>(
                    PrivateDocumentEntriesQuery::new(
                        contract.id(),
                        PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                        0,
                    ),
                )
                .await;
        }
        let second = Arc::new(Mutex::new(Vec::new()));
        let sent = send!(carol.id(), &second);
        assert!(
            matches!(sent, Err(PlatformWalletError::PrivateContactRequestPending(recipient))
                if recipient == carol.id()),
            "paid, its read back failing: {sent:?}"
        );
        assert_eq!(second.lock().expect("second").len(), 1, "no read first");
    }

    /// Once the account reference's rotation version is at its last, a
    /// request with the same xpub and key would repeat the tracked one: it is
    /// refused before anything is paid, and nothing stays pending.
    #[tokio::test]
    async fn should_refuse_a_request_once_every_rotation_version_is_used() {
        let alice = identity(ALICE_PHRASE, ALICE);
        let bob = identity(BOB_PHRASE, BOB);
        let alice_crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let alice_signer = signer(ALICE_PHRASE, &alice);
        let (_manager, alice_wallet) = wallet(ALICE_PHRASE, &alice, &[&alice, &bob], vec![]).await;
        let view = alice_wallet.identity().dashpay();
        let kept = Arc::new(Mutex::new(Vec::new()));
        let sent = view
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
            .expect("Alice sends the request, read back");
        assert_eq!(account_reference_version(sent.account_reference), 0);

        let mut exhausted = sent.clone();
        exhausted.account_reference = (sent.account_reference & 0x0FFF_FFFF) | (15 << 28);
        assert_eq!(account_reference_version(exhausted.account_reference), 15);
        {
            let iw = alice_wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&alice_wallet.wallet_id())
                .expect("wallet info")
                .identity_manager
                .managed_identity_mut(&alice.id())
                .expect("Alice")
                .add_sent_contact_request(exhausted, &iw.persister)
                .expect("the version 15 request");
        }

        let result = view
            .send_private_contact_request(
                &alice.id(),
                &bob.id(),
                None,
                AutoAcceptProofSource::None,
                None,
                &alice_signer,
                &alice_crypto,
                |_: BinaryData, _: PaymentStarted| -> std::future::Ready<Result<ResponseMetadata, PlatformWalletError>> {
                    panic!("a request that repeats the tracked one is not paid for")
                },
            )
            .await;
        assert!(
            matches!(&result, Err(PlatformWalletError::InvalidParameter(message))
                if message.contains("no account reference version is left")),
            "{result:?}"
        );
        assert!(!pending_to(&alice_wallet, ALICE, &bob).await);
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
            key_fingerprint: managed.private_scan_key_fingerprint(),
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
