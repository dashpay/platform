//! Scanning a private document type's store for the entries a wallet received or sent.
//!
//! An entry names nobody in the clear, so a wallet finds its own by trying every entry: with
//! the secret keys its identities receive at, and with the outgoing viewing keys its identities
//! send with, which recover what the wallet sent after a restore from seed.
//! [`PrivateDocumentScanKeys`] prepares both once, and [`Sdk::scan_private_document_page`]
//! fetches one proved page of the store from a cursor and sorts its entries.
//! [`Sdk::private_document_pages`] reads the store from a cursor to its end as a stream of
//! proved pages in store order, many fetched at once.
//!
//! A scan only opens entries. What an entry claims about its sender is checked against the
//! sender identity's keys: a deniable MAC with [`PrivateDocumentScanKeys::verify_sender`], a
//! signature with [`OpenedPrivateDocumentEntry::recover_signer`]. The payload is the type's to
//! decode and validate. `crate::platform::dashpay` does both for DashPay's private contact
//! requests.

use std::future::Future;
use std::pin::Pin;

use crate::platform::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntriesQuery, PrivateDocumentEntry,
};
use crate::platform::{Fetch, Identifier};
use crate::{Error, Sdk};
use dpp::identity::KeyID;
use dpp::secp256k1::{PublicKey, SecretKey};
use futures::stream::{self, Stream, StreamExt};
use platform_encryption::{
    private_document_outgoing_viewing_key, recover_sent_private_document_entry,
    OpenedPrivateDocumentEntry, PrivateDocumentEnvelopeError, PrivateDocumentRecipientKeys,
    PrivateDocumentTarget,
};
use zeroize::{Zeroize, Zeroizing};

/// One identity of the wallet and the keys entries to it may be encrypted to.
struct ScanRecipient {
    identity_id: Identifier,
    key_ids: Vec<KeyID>,
    keys: PrivateDocumentRecipientKeys,
}

/// One identity of the wallet and an outgoing viewing key it seals entries with. The key lives
/// in an allocation of its own, wiped when dropped: as senders are added, their vector moves
/// the pointer to it, never its bytes, so no copy of the key is left in a freed buffer.
struct ScanSender {
    identity_id: Identifier,
    key_id: KeyID,
    outgoing_viewing_key: Box<Zeroizing<[u8; 32]>>,
}

/// The keys a wallet scans a private document type's store with: for each of its identities,
/// the secret keys entries to it may be encrypted to, and the outgoing viewing key it sealed its
/// own entries with.
///
/// No secret moves as identities are added. A recipient's secret keys are in one allocation
/// sized once ([`PrivateDocumentRecipientKeys::new`] collects them from a slice), which the
/// recipients' vector moves by pointer and wipes when dropped; a sender's outgoing viewing key
/// is boxed ([`ScanSender`]).
#[derive(Default)]
pub struct PrivateDocumentScanKeys {
    recipients: Vec<ScanRecipient>,
    senders: Vec<ScanSender>,
}

/// An entry to one of the wallet's identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceivedPrivateDocumentEntry {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// The time of the block that appended it, in milliseconds.
    pub block_time_ms: u64,
    /// The identity it is to.
    pub recipient_identity_id: Identifier,
    /// The key of that identity it is encrypted to.
    pub recipient_key_id: KeyID,
    /// The opened entry. Its sender is what it claims, until
    /// [`PrivateDocumentScanKeys::verify_sender`] checks it.
    pub entry: OpenedPrivateDocumentEntry,
}

/// An entry one of the wallet's identities sealed, recovered with its outgoing viewing key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentPrivateDocumentEntry {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// The time of the block that appended it, in milliseconds.
    pub block_time_ms: u64,
    /// The identity whose outgoing viewing key recovered it.
    pub sender_identity_id: Identifier,
    /// The key of that identity the outgoing viewing key is derived from. An entry naming
    /// another sender identity or key was built by someone else holding the key.
    pub outgoing_key_id: KeyID,
    /// The recovered entry, with its recipient and payload.
    pub entry: OpenedPrivateDocumentEntry,
}

/// An entry that decrypted under one of the wallet's keys but is refused: malformed, of an
/// envelope version this SDK does not read, or naming an identity other than the key's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPrivateDocumentEntry {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// Why it is refused.
    pub error: PrivateDocumentEnvelopeError,
}

/// What one page of a private document type's store holds for the wallet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivateDocumentScan {
    /// The contract of the store the page was read from: the target
    /// [`PrivateDocumentScanKeys::scan_page`] was given. A reader of one store refuses a scan
    /// of another, whatever entries it holds, so another store's cursor is never taken for its
    /// own.
    pub contract_id: Identifier,
    /// The private document type of that store.
    pub document_type_name: String,
    /// The entries to the wallet's identities, in store order.
    pub received: Vec<ReceivedPrivateDocumentEntry>,
    /// The entries the wallet's identities sealed, in store order.
    pub sent: Vec<SentPrivateDocumentEntry>,
    /// The entries refused, in store order.
    pub rejected: Vec<RejectedPrivateDocumentEntry>,
    /// The position to scan from next: the one after the page's last entry, or the page's
    /// start when the page is empty.
    pub next_position: u64,
    /// How many entries the store held when the page was read.
    pub total_count: u64,
}

impl PrivateDocumentScan {
    /// Whether the page was read from the store of `document_type_name` of `contract_id`.
    pub fn is_of_store(&self, contract_id: &Identifier, document_type_name: &str) -> bool {
        self.contract_id == *contract_id && self.document_type_name == document_type_name
    }

    /// Whether the scan reached the store's last entry. A later scan from
    /// [`Self::next_position`] reads the entries appended since.
    pub fn is_complete(&self) -> bool {
        self.next_position >= self.total_count
    }
}

impl PrivateDocumentScanKeys {
    /// No keys yet: add the wallet's identities with [`Self::with_recipient`] and
    /// [`Self::with_sender`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `identity_id`, which entries may be encrypted to under any of `keys`, each the
    /// identity's key id with its secret key. Entries to it are tried with the keys in this
    /// order.
    pub fn with_recipient(mut self, identity_id: Identifier, keys: &[(KeyID, SecretKey)]) -> Self {
        let mut secret_keys: Vec<SecretKey> =
            keys.iter().map(|(_, secret_key)| *secret_key).collect();
        self.recipients.push(ScanRecipient {
            identity_id,
            key_ids: keys.iter().map(|(key_id, _)| *key_id).collect(),
            keys: PrivateDocumentRecipientKeys::new(&secret_keys),
        });
        for secret_key in &mut secret_keys {
            secret_key.non_secure_erase();
        }
        self
    }

    /// Adds `identity_id` as a sender, which sealed its entries with `outgoing_viewing_key`
    /// (`platform_encryption::private_document_outgoing_viewing_key`), derived from its key
    /// `key_id`.
    pub fn with_sender(
        mut self,
        identity_id: Identifier,
        key_id: KeyID,
        mut outgoing_viewing_key: [u8; 32],
    ) -> Self {
        self.senders.push(ScanSender {
            identity_id,
            key_id,
            outgoing_viewing_key: Box::new(Zeroizing::new(outgoing_viewing_key)),
        });
        outgoing_viewing_key.zeroize();
        self
    }

    /// The scan keys a wallet's key paths derive to, with `derive` deriving the secret key at a
    /// path: each of `recipients` with the keys entries to it may be sealed to, and each of
    /// `senders` with the key its outgoing viewing key is derived from
    /// (`platform_encryption::private_document_outgoing_viewing_key`). The secret keys derived
    /// are wiped once used, on an error too. For a signer that keeps the keys: it builds the scan
    /// keys and scans inside itself.
    #[allow(clippy::type_complexity)]
    pub fn derive<P, E>(
        recipients: &[(Identifier, Vec<(KeyID, P)>)],
        senders: &[(Identifier, KeyID, P)],
        mut derive: impl FnMut(&P) -> Result<SecretKey, E>,
    ) -> Result<Self, E> {
        let mut keys = Self::new();
        for (identity_id, paths) in recipients {
            let mut secret_keys = Vec::with_capacity(paths.len());
            let mut derived = Ok(());
            for (key_id, path) in paths {
                match derive(path) {
                    Ok(secret_key) => secret_keys.push((*key_id, secret_key)),
                    Err(error) => {
                        derived = Err(error);
                        break;
                    }
                }
            }
            if derived.is_ok() {
                keys = keys.with_recipient(*identity_id, &secret_keys);
            }
            for (_, secret_key) in &mut secret_keys {
                secret_key.non_secure_erase();
            }
            derived?;
        }
        for (identity_id, key_id, path) in senders {
            let mut secret_key = derive(path)?;
            let mut secret = secret_key.to_secret_bytes();
            secret_key.non_secure_erase();
            keys = keys.with_sender(
                *identity_id,
                *key_id,
                private_document_outgoing_viewing_key(&secret),
            );
            secret.zeroize();
        }
        Ok(keys)
    }

    /// Sorts the entries of `page`, read from `start_position` of the store of `target`.
    pub fn scan_page(
        &self,
        target: &PrivateDocumentTarget,
        start_position: u64,
        page: &PrivateDocumentEntries,
    ) -> PrivateDocumentScan {
        let mut scan = PrivateDocumentScan {
            contract_id: Identifier::from(target.contract_id),
            document_type_name: target.document_type_name.to_string(),
            next_position: page
                .entries
                .last()
                .map_or(start_position, |entry| entry.position.saturating_add(1)),
            total_count: page.total_count,
            ..Default::default()
        };
        for entry in &page.entries {
            self.scan_entry(target, entry, &mut scan);
        }
        scan
    }

    /// Checks that the key a received entry names wrote it, as
    /// [`OpenedPrivateDocumentEntry::verify_sender`] does with the recipient key that opened
    /// it. `sender_public_key` is the key the sender identity holds under the entry's
    /// `sender_key_id`. An entry these keys did not open is refused with
    /// `RecipientSecretKeyRequired`.
    pub fn verify_sender(
        &self,
        received: &ReceivedPrivateDocumentEntry,
        sender_public_key: &PublicKey,
    ) -> Result<(), PrivateDocumentEnvelopeError> {
        // The identity may have been added more than once, with other keys each time: the
        // check is the one of the keys that opened the entry.
        self.recipients
            .iter()
            .filter(|recipient| recipient.identity_id == received.recipient_identity_id)
            .map(|recipient| {
                recipient
                    .keys
                    .verify_sender(&received.entry, sender_public_key)
            })
            .find(|result| {
                !matches!(
                    result,
                    Err(PrivateDocumentEnvelopeError::RecipientSecretKeyRequired)
                )
            })
            .unwrap_or(Err(
                PrivateDocumentEnvelopeError::RecipientSecretKeyRequired,
            ))
    }

    fn scan_entry(
        &self,
        target: &PrivateDocumentTarget,
        entry: &PrivateDocumentEntry,
        scan: &mut PrivateDocumentScan,
    ) {
        let reject = |error| RejectedPrivateDocumentEntry {
            position: entry.position,
            block_height: entry.block_height,
            error,
        };

        // An entry naming another identity than the key's is refused only when no other of the
        // wallet's identities opens it: two of them may share a key.
        let mut refusal = None;
        let mut opened_for = None;
        for recipient in &self.recipients {
            match recipient
                .keys
                .open(&entry.data, target, recipient.identity_id.as_bytes())
            {
                Ok(Some(opened)) => {
                    opened_for = Some((recipient, opened));
                    break;
                }
                Ok(None) => {}
                // A store's entries all take its entry size: one too short for the envelope
                // holds nothing any key opens.
                Err(PrivateDocumentEnvelopeError::InvalidEntryLength(_)) => break,
                Err(error) => {
                    refusal.get_or_insert(error);
                }
            }
        }
        let received = opened_for.and_then(|(recipient, opened)| {
            let recipient_key_id = opened
                .recipient_key_index()
                .and_then(|index| recipient.key_ids.get(index))
                .copied()?;
            Some(ReceivedPrivateDocumentEntry {
                position: entry.position,
                block_height: entry.block_height,
                block_time_ms: entry.block_time_ms,
                recipient_identity_id: recipient.identity_id,
                recipient_key_id,
                entry: opened,
            })
        });
        let mut rejected = false;
        match (received, refusal) {
            (Some(received), _) => scan.received.push(received),
            (None, Some(error)) => {
                scan.rejected.push(reject(error));
                rejected = true;
            }
            (None, None) => {}
        }

        for sender in &self.senders {
            match recover_sent_private_document_entry(
                &entry.data,
                target,
                &sender.outgoing_viewing_key,
            ) {
                Ok(Some(recovered)) => {
                    // Senders added with the same outgoing viewing key all recover the entry:
                    // it belongs to the one it names, when there is one.
                    let named = self
                        .senders
                        .iter()
                        .filter(|other| other.outgoing_viewing_key == sender.outgoing_viewing_key)
                        .find(|other| {
                            other.identity_id.as_bytes() == recovered.sender_identity_id()
                                && other.key_id == recovered.sender_key_id()
                        })
                        .unwrap_or(sender);
                    scan.sent.push(SentPrivateDocumentEntry {
                        position: entry.position,
                        block_height: entry.block_height,
                        block_time_ms: entry.block_time_ms,
                        sender_identity_id: named.identity_id,
                        outgoing_key_id: named.key_id,
                        entry: recovered,
                    });
                    break;
                }
                Ok(None) => {}
                Err(PrivateDocumentEnvelopeError::InvalidEntryLength(_)) => break,
                // An entry is listed once, however many of the wallet's keys refuse it.
                Err(error) => {
                    if !rejected {
                        scan.rejected.push(reject(error));
                    }
                    break;
                }
            }
        }
    }
}

/// How many pages of a private document type's store [`Sdk::private_document_pages`] keeps in
/// flight, as the shielded note sync keeps as many chunk fetches. A page costs the node a
/// proof, not the transfer, so pages in flight are what speeds a read up; and the stream holds
/// no more pages than this unread, so its memory stays bounded.
pub const MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES: usize = 16;

/// One proved page of a private document type's store, as [`Sdk::private_document_pages`]
/// yields it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateDocumentPage {
    /// The position the page was read from.
    pub start_position: u64,
    /// The page.
    pub entries: PrivateDocumentEntries,
}

type PageFuture = Pin<Box<dyn Future<Output = Result<PrivateDocumentEntries, Error>> + Send>>;
type PageStream = Pin<Box<dyn Stream<Item = Result<PrivateDocumentPage, Error>> + Send>>;

/// Where a walk over a store's pages is.
enum PageWalk {
    /// Reading one page at a time from `start`, until the chunk size is known. `total` is
    /// the store's size as the first page reported it, `None` before it; `aligned` says that
    /// `start` is a chunk boundary, as the end of a page short of the store's end is.
    Sequential {
        start: u64,
        total: Option<u64>,
        aligned: bool,
    },
    /// The chunk size is known: every page left is fetched, several at once.
    Concurrent(PageStream),
    Done,
}

/// The pages of a store, read with `fetch` from `start_position` to the store's end as the
/// first page reports it, in store order: one at a time until the chunk size is known, then
/// up to `max_concurrent` at once ([`Sdk::private_document_pages`]).
fn private_document_page_walk<F>(
    fetch: F,
    start_position: u64,
    max_concurrent: usize,
) -> impl Stream<Item = Result<PrivateDocumentPage, Error>> + Send
where
    F: Fn(u64) -> PageFuture + Clone + Send + Sync + 'static,
{
    let max_concurrent = max_concurrent.max(1);
    stream::unfold(
        PageWalk::Sequential {
            start: start_position,
            total: None,
            aligned: false,
        },
        move |walk| {
            let fetch = fetch.clone();
            async move {
                match walk {
                    PageWalk::Done => None,
                    PageWalk::Concurrent(mut pages) => match pages.next().await? {
                        Ok(page) => Some((Ok(page), PageWalk::Concurrent(pages))),
                        Err(error) => Some((Err(error), PageWalk::Done)),
                    },
                    PageWalk::Sequential {
                        start,
                        total,
                        aligned,
                    } => {
                        let entries = match fetch(start).await {
                            Ok(entries) => entries,
                            Err(error) => return Some((Err(error), PageWalk::Done)),
                        };
                        let total = total.unwrap_or(entries.total_count);
                        let len = entries.entries.len() as u64;
                        let end = start.saturating_add(len);
                        // A page read from an older snapshot than the first page's (a node
                        // behind, within the SDK's tolerance) may end at that snapshot's end,
                        // partway through a chunk: it sets no chunk size, and the next page
                        // is read from where it stopped.
                        let ended_its_snapshot = end >= entries.total_count;
                        let page = PrivateDocumentPage {
                            start_position: start,
                            entries,
                        };
                        if end >= total {
                            return Some((Ok(page), PageWalk::Done));
                        }
                        if len == 0 {
                            return Some((
                                Err(Error::Generic(format!(
                                    "the page from position {start} of a store of {total} \
                                     entries is empty"
                                ))),
                                PageWalk::Done,
                            ));
                        }
                        // A page from a chunk boundary that stops short of the end of the store
                        // it was read from is one whole chunk, a power of two, and so is every
                        // page after it.
                        let next = if aligned
                            && !ended_its_snapshot
                            && len.is_power_of_two()
                            && start % len == 0
                        {
                            PageWalk::Concurrent(Box::pin(whole_chunk_pages(
                                fetch,
                                end,
                                len,
                                total,
                                max_concurrent,
                            )))
                        } else {
                            PageWalk::Sequential {
                                start: end,
                                total: Some(total),
                                aligned: !ended_its_snapshot,
                            }
                        };
                        Some((Ok(page), next))
                    }
                }
            }
        },
    )
}

/// The pages of `chunk_size` entries from `first`, a chunk boundary, to `total`, fetched up to
/// `max_concurrent` at once and yielded in store order. A page that ends at the end of an older
/// snapshot of the store, partway through its chunk, is completed with the rest of the chunk
/// read from where it stopped. A page that holds more than its chunk, or stops short of it for
/// any other reason before the store's end, is an error: the walk would otherwise skip entries.
fn whole_chunk_pages<F>(
    fetch: F,
    first: u64,
    chunk_size: u64,
    total: u64,
    max_concurrent: usize,
) -> impl Stream<Item = Result<PrivateDocumentPage, Error>> + Send
where
    F: Fn(u64) -> PageFuture + Clone + Send + Sync + 'static,
{
    let starts = std::iter::successors(Some(first), move |start| {
        start.checked_add(chunk_size).filter(|next| *next < total)
    })
    .filter(move |start| *start < total);
    stream::iter(starts)
        .map(move |start| {
            let fetch = fetch.clone();
            async move {
                let mut entries = fetch(start).await?;
                // A page read from an older snapshot than the first page's (a node behind,
                // within the SDK's tolerance) may end at that snapshot's end, partway through
                // its chunk: the rest of the chunk is read from where it stopped, into the same
                // page, before any later chunk is yielded, so no entry is skipped.
                loop {
                    let len = entries.entries.len() as u64;
                    let end = start.saturating_add(len);
                    let first_position = entries.entries.first().map(|entry| entry.position);
                    let refused = || {
                        Error::Generic(format!(
                            "the page from position {start} holds {len} entries, not the chunk \
                             of {chunk_size} the store's earlier pages set"
                        ))
                    };
                    if len > chunk_size || first_position.is_some_and(|p| p != start) {
                        return Err(refused());
                    }
                    if len == chunk_size || end >= total {
                        break;
                    }
                    // Short of its chunk and of the walk's end: only the end of the store it
                    // was read from explains that.
                    if end < entries.total_count {
                        return Err(refused());
                    }
                    let rest = fetch(end).await?;
                    if rest.entries.first().map(|entry| entry.position) != Some(end) {
                        return Err(refused());
                    }
                    entries.entries.extend(rest.entries);
                    entries.total_count = rest.total_count;
                }
                Ok(PrivateDocumentPage {
                    start_position: start,
                    entries,
                })
            }
        })
        .buffered(max_concurrent)
}

impl Sdk {
    /// The proved pages of the store of the private type `document_type_name` of
    /// `data_contract_id`, from `start_position` to the store's end as the first page reports
    /// it, in store order, with up to `max_concurrent` fetched at once
    /// ([`MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES`] is the wallet's choice). Entries
    /// appended while the pages are read are left for the next read, unless the last page's
    /// chunk holds them.
    ///
    /// A page reads to the end of the chunk holding its start, which the client does not
    /// learn from the node: so the first page, and the next one from the chunk boundary it
    /// ends at, are read one at a time. That second page, short of the store's end, is a whole
    /// chunk, which sets the size of every page after it; those are then fetched at once. A
    /// page that ends at the end of the store it was read from, an older snapshot than the
    /// first page's, sets no size: the pages after it are read one at a time until one does. A
    /// store with two chunks or fewer past `start_position` is read one page at a time.
    ///
    /// The context provider is given the contract with the type first when it lacks it
    /// ([`Sdk::fetch_private_document_contract`]). A failed fetch ends the stream with its
    /// error, after the pages before it.
    pub fn private_document_pages(
        &self,
        data_contract_id: Identifier,
        document_type_name: &str,
        start_position: u64,
        max_concurrent: usize,
    ) -> impl Stream<Item = Result<PrivateDocumentPage, Error>> + Send {
        let sdk = self.clone();
        let document_type_name = document_type_name.to_string();
        let resolve = {
            let sdk = sdk.clone();
            let document_type_name = document_type_name.clone();
            async move {
                sdk.fetch_private_document_contract(data_contract_id, &document_type_name)
                    .await
            }
        };
        stream::once(resolve).flat_map(move |resolved| -> PageStream {
            match resolved {
                Err(error) => Box::pin(stream::once(async move { Err(error) })),
                Ok(_) => {
                    let sdk = sdk.clone();
                    let document_type_name = document_type_name.clone();
                    let fetch = move |start: u64| -> PageFuture {
                        let sdk = sdk.clone();
                        let query = PrivateDocumentEntriesQuery::new(
                            data_contract_id,
                            document_type_name.clone(),
                            start,
                        );
                        Box::pin(async move {
                            PrivateDocumentEntries::fetch(&sdk, query)
                                .await?
                                .ok_or_else(|| {
                                    Error::Generic(format!(
                                        "no page of the private document type of contract \
                                         {data_contract_id} from position {start}"
                                    ))
                                })
                        })
                    };
                    Box::pin(private_document_page_walk(
                        fetch,
                        start_position,
                        max_concurrent,
                    ))
                }
            }
        })
    }

    /// Fetches the proved page of the store of the private type `document_type_name` of
    /// `data_contract_id` from `start_position`, and sorts its entries with `keys`. The context
    /// provider is given the contract with the type first when it lacks it
    /// ([`Sdk::fetch_private_document_contract`]).
    ///
    /// A wallet keeps [`PrivateDocumentScan::next_position`] as its cursor once it has stored
    /// what the page held, and scans from it again until [`PrivateDocumentScan::is_complete`];
    /// the store only grows, so a later scan from the cursor reads what was appended since.
    pub async fn scan_private_document_page(
        &self,
        data_contract_id: Identifier,
        document_type_name: &str,
        start_position: u64,
        keys: &PrivateDocumentScanKeys,
    ) -> Result<PrivateDocumentScan, Error> {
        // The page is verified against the contract the context provider holds.
        self.fetch_private_document_contract(data_contract_id, document_type_name)
            .await?;
        let query =
            PrivateDocumentEntriesQuery::new(data_contract_id, document_type_name, start_position);
        let page = PrivateDocumentEntries::fetch(self, query)
            .await?
            .ok_or_else(|| {
                Error::Generic(format!(
                    "no page of the private document type \"{document_type_name}\" of contract \
                     {data_contract_id} from position {start_position}"
                ))
            })?;
        let target = PrivateDocumentTarget {
            contract_id: data_contract_id.to_buffer(),
            document_type_name,
        };
        Ok(keys.scan_page(&target, start_position, &page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_encryption::{
        private_document_outgoing_viewing_key, seal_private_document_entry,
        PrivateDocumentRecipient, PrivateDocumentSender,
    };

    const ENTRY_SIZE: usize = 589;
    const CONTRACT_ID: [u8; 32] = [7; 32];
    const TYPE_NAME: &str = "privateContactRequest";

    fn target() -> PrivateDocumentTarget<'static> {
        PrivateDocumentTarget {
            contract_id: CONTRACT_ID,
            document_type_name: TYPE_NAME,
        }
    }

    fn secret_key(byte: u8) -> SecretKey {
        SecretKey::from_secret_bytes([byte; 32]).expect("a valid secret key")
    }

    fn public_key(byte: u8) -> PublicKey {
        PublicKey::from_secret_key(&secret_key(byte))
    }

    fn ovk(byte: u8) -> [u8; 32] {
        private_document_outgoing_viewing_key(&[byte; 32])
    }

    /// An entry from `sender_byte`'s identity and key 4 to `recipient` under the key of
    /// `recipient_key`, sealed with the outgoing viewing key of `ovk_byte`.
    fn seal(
        sender_byte: u8,
        recipient: [u8; 32],
        recipient_key: u8,
        ovk_byte: u8,
        seed: u8,
    ) -> Vec<u8> {
        let sender_key = secret_key(sender_byte);
        seal_private_document_entry(
            &target(),
            ENTRY_SIZE,
            &PrivateDocumentRecipient {
                identity_id: recipient,
                public_key: &public_key(recipient_key),
            },
            &PrivateDocumentSender::DeniableMac {
                identity_id: [sender_byte; 32],
                key_id: 4,
                secret_key: &sender_key,
            },
            &[seed; 40],
            &ovk(ovk_byte),
            &[seed; 32],
        )
        .expect("the payload fits")
    }

    fn page(start: u64, entries: Vec<Vec<u8>>, total_count: u64) -> PrivateDocumentEntries {
        PrivateDocumentEntries {
            entries: entries
                .into_iter()
                .zip(start..)
                .map(|(data, position)| PrivateDocumentEntry {
                    position,
                    block_height: 100 + position,
                    block_time_ms: 1_000 * position,
                    data,
                })
                .collect(),
            total_count,
        }
    }

    /// A page sorts into what the wallet's identities received, what they sent, and what opened
    /// for one of their keys but is refused; the rest, for other wallets, is passed over.
    #[test]
    fn should_sort_a_page_into_received_sent_and_rejected_entries() {
        let alice = Identifier::from([0xA0; 32]);
        let bob = Identifier::from([0xB0; 32]);
        let keys = PrivateDocumentScanKeys::new()
            .with_recipient(alice, &[(2, secret_key(20)), (3, secret_key(21))])
            .with_recipient(bob, &[(1, secret_key(30))])
            .with_sender(alice, 6, ovk(0xA1));
        let entries = vec![
            // 10: to Alice's key 3, from Carol
            seal(0xC0, alice.to_buffer(), 21, 0xC1, 1),
            // 11: to someone else, from someone else
            seal(0xC0, [0xD0; 32], 40, 0xC1, 2),
            // 12: from Alice to Dave
            seal(0xA0, [0xD0; 32], 40, 0xA1, 3),
            // 13: to Bob's key, but naming Dave: a replay
            seal(0xC0, [0xD0; 32], 30, 0xC1, 4),
            // 14: from Alice to Bob, both the wallet's
            seal(0xA0, bob.to_buffer(), 30, 0xA1, 5),
        ];
        let scan = keys.scan_page(&target(), 10, &page(10, entries, 20));

        assert_eq!(
            scan.received
                .iter()
                .map(|received| (
                    received.position,
                    received.recipient_identity_id,
                    received.recipient_key_id
                ))
                .collect::<Vec<_>>(),
            vec![(10, alice, 3), (14, bob, 1)]
        );
        assert_eq!(scan.received[0].block_height, 110);
        assert_eq!(scan.received[0].block_time_ms, 10_000);
        assert_eq!(scan.received[0].entry.payload(), &[1; 40]);
        assert_eq!(
            scan.sent
                .iter()
                .map(|sent| (sent.position, sent.sender_identity_id, sent.outgoing_key_id))
                .collect::<Vec<_>>(),
            vec![(12, alice, 6), (14, alice, 6)]
        );
        assert_eq!(scan.sent[0].entry.recipient_identity_id(), &[0xD0; 32]);
        assert_eq!(
            scan.rejected,
            vec![RejectedPrivateDocumentEntry {
                position: 13,
                block_height: 113,
                error: PrivateDocumentEnvelopeError::WrongRecipient,
            }]
        );
        assert_eq!(scan.next_position, 15);
        assert!(!scan.is_complete());
    }

    /// An entry naming one of the wallet's identities opens for it even when another identity
    /// of the wallet holding the same key comes first.
    #[test]
    fn should_open_an_entry_for_whichever_identity_it_names_when_two_share_a_key() {
        let alice = Identifier::from([0xA0; 32]);
        let bob = Identifier::from([0xB0; 32]);
        let keys = PrivateDocumentScanKeys::new()
            .with_recipient(alice, &[(2, secret_key(20))])
            .with_recipient(bob, &[(5, secret_key(20))]);
        let scan = keys.scan_page(
            &target(),
            0,
            &page(0, vec![seal(0xC0, bob.to_buffer(), 20, 0xC1, 1)], 1),
        );
        assert_eq!(scan.rejected, vec![]);
        assert_eq!(scan.received.len(), 1);
        assert_eq!(scan.received[0].recipient_identity_id, bob);
        assert_eq!(scan.received[0].recipient_key_id, 5);
        assert!(scan.is_complete());
    }

    /// The scan keys check a received entry's sender with the key that opened it.
    #[test]
    fn should_verify_a_received_entry_s_sender_with_the_key_that_opened_it() {
        let alice = Identifier::from([0xA0; 32]);
        let keys = PrivateDocumentScanKeys::new().with_recipient(alice, &[(2, secret_key(20))]);
        let scan = keys.scan_page(
            &target(),
            0,
            &page(0, vec![seal(0xC0, alice.to_buffer(), 20, 0xC1, 1)], 1),
        );
        let received = &scan.received[0];
        assert_eq!(received.entry.sender_identity_id(), &[0xC0; 32]);
        assert_eq!(received.entry.sender_key_id(), 4);
        keys.verify_sender(received, &public_key(0xC0))
            .expect("Carol's key wrote it");
        assert_eq!(
            keys.verify_sender(received, &public_key(0xC1)),
            Err(PrivateDocumentEnvelopeError::SenderNotAuthenticated)
        );
        let others = PrivateDocumentScanKeys::new()
            .with_recipient(Identifier::from([0xB0; 32]), &[(2, secret_key(20))]);
        assert_eq!(
            others.verify_sender(received, &public_key(0xC0)),
            Err(PrivateDocumentEnvelopeError::RecipientSecretKeyRequired)
        );

        // Added twice with other keys each time, the identity is checked with the keys that
        // opened the entry, not the first it was added with.
        let twice = PrivateDocumentScanKeys::new()
            .with_recipient(alice, &[(1, secret_key(19))])
            .with_recipient(alice, &[(2, secret_key(20))]);
        let scan = twice.scan_page(
            &target(),
            0,
            &page(0, vec![seal(0xC0, alice.to_buffer(), 20, 0xC1, 1)], 1),
        );
        twice
            .verify_sender(&scan.received[0], &public_key(0xC0))
            .expect("the second key set opened it and checks it");
    }

    /// Adding senders, which grows their vector, moves no outgoing viewing key: each stays at
    /// the address it was given, so growth leaves no copy behind in a freed buffer.
    #[test]
    fn should_keep_each_outgoing_viewing_key_in_place_as_senders_are_added() {
        let mut keys =
            PrivateDocumentScanKeys::new().with_sender(Identifier::from([0xA0; 32]), 1, [0x11; 32]);
        let first: *const [u8; 32] = &**keys.senders[0].outgoing_viewing_key;
        for index in 0..64u8 {
            keys = keys.with_sender(Identifier::from([index; 32]), 1, [index; 32]);
        }
        assert!(keys.senders.capacity() > 1, "the vector grew");
        assert_eq!(
            first,
            &**keys.senders[0].outgoing_viewing_key as *const [u8; 32]
        );
        assert_eq!(**keys.senders[0].outgoing_viewing_key, [0x11; 32]);
    }

    /// Two senders added with the same outgoing viewing key: an entry recovers as the sender
    /// it names, not the first added.
    #[test]
    fn should_recover_an_entry_as_the_sender_it_names_when_two_share_a_key() {
        let alice = Identifier::from([0xA0; 32]);
        let carol = Identifier::from([0xC0; 32]);
        let keys = PrivateDocumentScanKeys::new()
            .with_sender(alice, 4, ovk(0xC1))
            .with_sender(carol, 4, ovk(0xC1));
        let scan = keys.scan_page(
            &target(),
            0,
            &page(0, vec![seal(0xC0, [0xD0; 32], 40, 0xC1, 1)], 1),
        );
        assert_eq!(scan.rejected, vec![]);
        assert_eq!(
            scan.sent
                .iter()
                .map(|sent| (sent.sender_identity_id, sent.outgoing_key_id))
                .collect::<Vec<_>>(),
            vec![(carol, 4)]
        );
    }

    /// Keys derived from paths scan as the same keys added directly, and a derivation that
    /// fails fails the whole set.
    #[test]
    fn should_derive_scan_keys_from_paths() {
        let alice = Identifier::from([0xA0; 32]);
        let derived = PrivateDocumentScanKeys::derive(
            &[(alice, vec![(2u32, 20u8)])],
            &[(alice, 6, 0xA1)],
            |byte| Ok::<_, ()>(secret_key(*byte)),
        )
        .expect("every path derives");
        let added = PrivateDocumentScanKeys::new()
            .with_recipient(alice, &[(2, secret_key(20))])
            .with_sender(
                alice,
                6,
                private_document_outgoing_viewing_key(&secret_key(0xA1).to_secret_bytes()),
            );
        let entries = page(
            0,
            vec![
                seal(0xC0, alice.to_buffer(), 20, 0xC1, 1),
                seal(0xA0, [0xD0; 32], 40, 0xA1, 2),
            ],
            2,
        );
        assert_eq!(
            derived.scan_page(&target(), 0, &entries),
            added.scan_page(&target(), 0, &entries)
        );
        assert!(PrivateDocumentScanKeys::derive(
            &[(alice, vec![(2u32, 20u8), (3, 0)])],
            &[],
            |byte| if *byte == 0 {
                Err(())
            } else {
                Ok(secret_key(*byte))
            },
        )
        .is_err());
    }

    /// A page at the store's end is empty: the cursor stays where it is.
    #[test]
    fn should_keep_the_cursor_on_an_empty_page_at_the_store_s_end() {
        let keys = PrivateDocumentScanKeys::new()
            .with_recipient(Identifier::from([0xA0; 32]), &[(2, secret_key(20))]);
        let scan = keys.scan_page(&target(), 7, &page(7, vec![], 7));
        assert_eq!(
            scan,
            PrivateDocumentScan {
                contract_id: Identifier::from(target().contract_id),
                document_type_name: target().document_type_name.to_string(),
                next_position: 7,
                total_count: 7,
                ..Default::default()
            }
        );
        assert!(scan.is_complete());
    }

    /// The page Drive serves from `start` of a store of `total` entries in chunks of
    /// `chunk_size`: the rest of the chunk holding `start`, or to the store's end.
    fn drive_page(chunk_size: u64, total: u64, start: u64) -> PrivateDocumentEntries {
        let end = if start >= total {
            start
        } else {
            (start - start % chunk_size + chunk_size).min(total)
        };
        PrivateDocumentEntries {
            entries: (start..end)
                .map(|position| PrivateDocumentEntry {
                    position,
                    block_height: position,
                    block_time_ms: position,
                    data: vec![position as u8; 4],
                })
                .collect(),
            total_count: total,
        }
    }

    /// Fetches with how many are in flight at once noted.
    #[derive(Clone, Default)]
    struct InFlight {
        now: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        most: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        starts: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
    }

    impl InFlight {
        /// A fetch of Drive's pages of a store of `total` entries in chunks of `chunk_size`,
        /// each taking `delay`, failing from `failing` when given.
        fn fetch(
            &self,
            chunk_size: u64,
            total: u64,
            delay: std::time::Duration,
            failing: Option<u64>,
        ) -> impl Fn(u64) -> PageFuture + Clone + Send + Sync + 'static {
            let in_flight = self.clone();
            move |start| {
                let in_flight = in_flight.clone();
                Box::pin(async move {
                    use std::sync::atomic::Ordering::SeqCst;
                    in_flight.starts.lock().expect("starts").push(start);
                    let now = in_flight.now.fetch_add(1, SeqCst) + 1;
                    in_flight.most.fetch_max(now, SeqCst);
                    tokio::time::sleep(delay).await;
                    in_flight.now.fetch_sub(1, SeqCst);
                    if failing == Some(start) {
                        return Err(Error::Generic(format!("no page from {start}")));
                    }
                    Ok(drive_page(chunk_size, total, start))
                })
            }
        }

        fn most(&self) -> usize {
            self.most.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    /// Every page a walk yields, and the positions of their entries.
    async fn walk(
        fetch: impl Fn(u64) -> PageFuture + Clone + Send + Sync + 'static,
        start: u64,
        max_concurrent: usize,
    ) -> Vec<Result<PrivateDocumentPage, Error>> {
        private_document_page_walk(fetch, start, max_concurrent)
            .collect()
            .await
    }

    /// From a chunk boundary, from inside a chunk and from the store's last partial chunk,
    /// the pages come in store order, each from where the last ended, and hold every entry
    /// from the cursor to the store's end once.
    #[tokio::test]
    async fn should_read_a_store_s_pages_in_order_from_any_cursor() {
        let (chunk_size, total) = (4, 30);
        for cursor in [0, 6, 16, 26, 29, 30] {
            let pages = walk(
                InFlight::default().fetch(chunk_size, total, std::time::Duration::ZERO, None),
                cursor,
                MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
            )
            .await;
            let mut expected_start = cursor;
            let mut positions = Vec::new();
            for page in pages {
                let page = page.expect("every page reads");
                assert_eq!(page.start_position, expected_start, "cursor {cursor}");
                expected_start += page.entries.entries.len() as u64;
                positions.extend(page.entries.entries.iter().map(|entry| entry.position));
            }
            assert_eq!(
                positions,
                (cursor..total).collect::<Vec<_>>(),
                "cursor {cursor}"
            );
        }
    }

    /// A store of two chunks or fewer past the cursor is read one page at a time.
    #[tokio::test]
    async fn should_read_a_short_store_one_page_at_a_time() {
        for (cursor, total) in [(1, 7), (0, 8), (3, 4)] {
            let in_flight = InFlight::default();
            let pages = walk(
                in_flight.fetch(4, total, std::time::Duration::from_millis(5), None),
                cursor,
                MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
            )
            .await;
            assert!(pages.iter().all(Result::is_ok));
            assert_eq!(
                pages
                    .iter()
                    .flatten()
                    .map(|page| page.start_position)
                    .collect::<Vec<_>>(),
                *in_flight.starts.lock().expect("starts")
            );
            assert_eq!(in_flight.most(), 1, "cursor {cursor} of {total}");
        }
    }

    /// Past the first two pages, several fetches are in flight at once, as many as allowed.
    #[tokio::test]
    async fn should_keep_several_page_fetches_in_flight() {
        let in_flight = InFlight::default();
        let pages = walk(
            in_flight.fetch(4, 64, std::time::Duration::from_millis(20), None),
            0,
            5,
        )
        .await;
        assert_eq!(pages.len(), 16);
        assert!(pages.iter().all(Result::is_ok));
        assert_eq!(in_flight.most(), 5);
    }

    /// A failed fetch ends the stream with its error, after every page before it.
    #[tokio::test]
    async fn should_end_with_a_later_page_s_error_after_the_pages_before_it() {
        let pages = walk(
            InFlight::default().fetch(4, 40, std::time::Duration::from_millis(1), Some(20)),
            0,
            MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
        )
        .await;
        let starts: Vec<u64> = pages
            .iter()
            .take_while(|page| page.is_ok())
            .flatten()
            .map(|page| page.start_position)
            .collect();
        assert_eq!(starts, vec![0, 4, 8, 12, 16]);
        assert_eq!(pages.len(), 6, "the error, then nothing");
        assert!(pages[5].is_err());
    }

    /// A fetch of Drive's pages of a store of `total` entries in chunks of 4 where the pages
    /// from 8, 12 and 16, fetched at once, complete in the reverse order: 16 first, which lets
    /// 12 complete, which lets 8. The page from `failing` fails instead of reading. The
    /// positions are noted as the fetches complete.
    fn completing_backwards(
        total: u64,
        failing: Option<u64>,
        completed: &std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
    ) -> impl Fn(u64) -> PageFuture + Clone + Send + Sync + 'static {
        use std::sync::Arc;
        use tokio::sync::Notify;

        let releases = Arc::new([Notify::new(), Notify::new()]);
        let completed = Arc::clone(completed);
        move |start| {
            let releases = Arc::clone(&releases);
            let completed = Arc::clone(&completed);
            Box::pin(async move {
                match start {
                    8 => releases[0].notified().await,
                    12 => releases[1].notified().await,
                    _ => {}
                }
                completed.lock().expect("completed").push(start);
                match start {
                    16 => releases[1].notify_one(),
                    12 => releases[0].notify_one(),
                    _ => {}
                }
                if failing == Some(start) {
                    return Err(Error::Generic(format!("no page from {start}")));
                }
                Ok(drive_page(4, total, start))
            })
        }
    }

    /// Pages fetched at once that complete out of order are yielded in store order.
    #[tokio::test]
    async fn should_yield_pages_in_store_order_when_later_ones_complete_first() {
        let completed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pages = walk(completing_backwards(24, None, &completed), 0, 3).await;
        assert_eq!(
            completed.lock().expect("completed")[..5],
            [0, 4, 16, 12, 8],
            "the later pages complete first"
        );
        assert_eq!(
            pages
                .into_iter()
                .map(|page| page.expect("every page reads").start_position)
                .collect::<Vec<_>>(),
            vec![0, 4, 8, 12, 16, 20]
        );
    }

    /// A later page's failure that completes before an earlier page's success is yielded
    /// after it, and ends the stream.
    #[tokio::test]
    async fn should_yield_a_later_failure_after_the_earlier_success_that_completes_after_it() {
        let completed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pages = walk(completing_backwards(24, Some(12), &completed), 0, 3).await;
        let completed = completed.lock().expect("completed").clone();
        let failed_at = completed.iter().position(|start| *start == 12);
        let read_at = completed.iter().position(|start| *start == 8);
        assert!(
            failed_at < read_at,
            "the failure completes first: {completed:?}"
        );
        let starts: Vec<u64> = pages
            .iter()
            .take_while(|page| page.is_ok())
            .flatten()
            .map(|page| page.start_position)
            .collect();
        assert_eq!(starts, vec![0, 4, 8]);
        assert_eq!(pages.len(), 4, "the failure, then nothing: {pages:?}");
        assert!(pages[3].is_err());
    }

    /// Entries appended during a walk, here a store of 30 entries in chunks of 4 that grows to
    /// 35 once the first two pages are read, are read as far as the last chunk the first
    /// page's count reaches, which now holds them to position 31: the positions come once
    /// each, in order. A walk from the cursor the pages leave reads the rest.
    #[tokio::test]
    async fn should_leave_entries_appended_during_a_walk_to_a_walk_from_its_cursor() {
        use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
        use std::sync::Arc;

        let total = Arc::new(AtomicU64::new(30));
        let fetched = Arc::new(AtomicUsize::new(0));
        let fetch = {
            let total = Arc::clone(&total);
            move |start: u64| -> PageFuture {
                let total = Arc::clone(&total);
                let fetched = Arc::clone(&fetched);
                Box::pin(async move {
                    let page = drive_page(4, total.load(Ordering::SeqCst), start);
                    if fetched.fetch_add(1, Ordering::SeqCst) + 1 == 2 {
                        total.store(35, Ordering::SeqCst);
                    }
                    Ok(page)
                })
            }
        };
        let positions = |pages: &[Result<PrivateDocumentPage, Error>]| -> Vec<u64> {
            pages
                .iter()
                .flat_map(|page| {
                    page.as_ref()
                        .expect("every page reads")
                        .entries
                        .entries
                        .iter()
                        .map(|entry| entry.position)
                })
                .collect()
        };

        let first = walk(
            fetch.clone(),
            0,
            MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
        )
        .await;
        assert_eq!(positions(&first), (0..32).collect::<Vec<_>>());
        let cursor = first
            .last()
            .and_then(|page| page.as_ref().ok())
            .map(|page| page.start_position + page.entries.entries.len() as u64)
            .expect("a last page");
        assert_eq!(cursor, 32);

        let rest = walk(fetch, cursor, MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES).await;
        assert_eq!(positions(&rest), (32..35).collect::<Vec<_>>());
    }

    /// A page from an older snapshot than the first page's, here the second page from a node
    /// whose store ended partway through its chunk, is no whole chunk: the walk reads on from
    /// where it stopped, learns the chunk size from a later page, and yields every page in
    /// order.
    #[tokio::test]
    async fn should_read_on_past_a_page_from_an_older_snapshot_that_ends_mid_chunk() {
        let fetch = |start: u64| -> PageFuture {
            Box::pin(async move {
                // The node answering the page from 4 is behind: its store held 6 entries.
                let total = if start == 4 { 6 } else { 30 };
                Ok(drive_page(4, total, start))
            })
        };
        let pages = walk(fetch, 0, MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES).await;
        let mut expected_start = 0;
        let mut positions = Vec::new();
        for page in pages {
            let page = page.expect("every page reads");
            assert_eq!(page.start_position, expected_start);
            expected_start += page.entries.entries.len() as u64;
            positions.extend(page.entries.entries.iter().map(|entry| entry.position));
        }
        assert_eq!(positions, (0..30).collect::<Vec<_>>());
    }

    /// A page fetched at once with others from a node behind, whose store ended partway
    /// through its chunk, is completed from where it stopped before any later chunk is
    /// yielded: every entry up to the first page's count comes once, in order.
    #[tokio::test]
    async fn should_complete_a_chunk_an_older_snapshot_ended_partway_through() {
        let fetch = |start: u64| -> PageFuture {
            Box::pin(async move {
                // The node answering the page from 12 is behind: its store held 14 entries.
                let total = if start == 12 { 14 } else { 30 };
                Ok(drive_page(4, total, start))
            })
        };
        let pages = walk(fetch, 0, MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES).await;
        let mut expected_start = 0;
        let mut positions = Vec::new();
        for page in pages {
            let page = page.expect("every page reads");
            assert_eq!(page.start_position, expected_start);
            expected_start += page.entries.entries.len() as u64;
            positions.extend(page.entries.entries.iter().map(|entry| entry.position));
        }
        assert_eq!(positions, (0..30).collect::<Vec<_>>());
    }

    /// A page short of its chunk, before the store's end, is an error rather than a gap.
    #[tokio::test]
    async fn should_refuse_a_page_short_of_its_chunk() {
        let fetch = |start: u64| -> PageFuture {
            Box::pin(async move {
                let mut page = drive_page(4, 40, start);
                if start == 12 {
                    page.entries.truncate(2);
                }
                Ok(page)
            })
        };
        let pages = walk(fetch, 0, 4).await;
        assert!(pages[..3].iter().all(Result::is_ok));
        let error = pages[3].as_ref().expect_err("a short page").to_string();
        assert!(error.contains("not the chunk of 4"), "{error}");
        assert_eq!(pages.len(), 4);
    }

    /// The pages read through the SDK, after the context provider is given the contract.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_stream_the_pages_of_a_store_through_the_sdk() {
        use crate::platform::private_document_entries::held_contract_provider::HeldContractProvider;
        use crate::SdkBuilder;
        use dpp::version::PlatformVersion;

        let mut sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(HeldContractProvider::new(
                Some(contract_with_the_private_type()),
                false,
            ))
            .build()
            .expect("expected a mock sdk");
        let contract_id = Identifier::from(CONTRACT_ID);
        for start in [2, 4, 8, 12] {
            sdk.mock()
                .expect_fetch(
                    PrivateDocumentEntriesQuery::new(contract_id, TYPE_NAME, start),
                    Some(drive_page(4, 14, start)),
                )
                .await
                .expect("expected to register the page");
        }
        let pages: Vec<PrivateDocumentPage> = sdk
            .private_document_pages(contract_id, TYPE_NAME, 2, 3)
            .map(|page| page.expect("every page reads"))
            .collect()
            .await;
        assert_eq!(
            pages
                .iter()
                .map(|page| page.start_position)
                .collect::<Vec<_>>(),
            vec![2, 4, 8, 12]
        );
        assert_eq!(
            pages
                .iter()
                .flat_map(|page| page.entries.entries.iter().map(|entry| entry.position))
                .collect::<Vec<_>>(),
            (2..14).collect::<Vec<_>>()
        );
    }

    /// DashPay as the latest version has it, under the test contract's id: a contract that
    /// declares the private type the test entries are sealed for.
    #[cfg(feature = "mocks")]
    fn contract_with_the_private_type() -> dpp::data_contract::DataContract {
        use dpp::data_contract::accessors::v0::DataContractV0Setters;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;

        let mut contract =
            load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())
                .expect("DashPay loads at the latest version");
        contract.set_id(Identifier::from(CONTRACT_ID));
        contract
    }

    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_scan_a_fetched_page_of_the_store() {
        use crate::platform::private_document_entries::held_contract_provider::HeldContractProvider;
        use crate::SdkBuilder;
        use dpp::version::PlatformVersion;

        let mut sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(HeldContractProvider::new(
                Some(contract_with_the_private_type()),
                false,
            ))
            .build()
            .expect("expected a mock sdk");
        let alice = Identifier::from([0xA0; 32]);
        let contract_id = Identifier::from(CONTRACT_ID);
        let query = PrivateDocumentEntriesQuery::new(contract_id, TYPE_NAME, 3);
        let fetched = page(
            3,
            vec![
                seal(0xC0, [0xD0; 32], 40, 0xC1, 1),
                seal(0xC0, alice.to_buffer(), 20, 0xC1, 2),
            ],
            5,
        );
        sdk.mock()
            .expect_fetch(query, Some(fetched))
            .await
            .expect("expected to register the page");

        let keys = PrivateDocumentScanKeys::new().with_recipient(alice, &[(2, secret_key(20))]);
        let scan = sdk
            .scan_private_document_page(contract_id, TYPE_NAME, 3, &keys)
            .await
            .expect("expected the page");
        assert_eq!(scan.received.len(), 1);
        assert_eq!(scan.received[0].position, 4);
        assert_eq!(scan.next_position, 5);
        assert!(scan.is_complete());
    }
}
