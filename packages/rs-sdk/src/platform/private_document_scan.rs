//! Scanning a private document type's store for the entries a wallet received or sent.
//!
//! An entry names nobody in the clear, so a wallet finds its own by trying every entry: with
//! the secret keys its identities receive at, and with the outgoing viewing keys its identities
//! send with, which recover what the wallet sent after a restore from seed.
//! [`PrivateDocumentScanKeys`] prepares both once, and [`Sdk::scan_private_document_page`]
//! fetches one proved page of the store from a cursor and sorts its entries.
//!
//! A scan only opens entries. What an entry claims about its sender is checked against the
//! sender identity's key with [`PrivateDocumentScanKeys::verify_sender`], and the payload is the
//! type's to decode and validate. `crate::platform::dashpay` does both for DashPay's private
//! contact requests.

use crate::platform::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntriesQuery, PrivateDocumentEntry,
};
use crate::platform::{Fetch, Identifier};
use crate::{Error, Sdk};
use dpp::identity::KeyID;
use dpp::secp256k1::{PublicKey, SecretKey};
use platform_encryption::{
    recover_sent_private_document_entry, OpenedPrivateDocumentEntry, PrivateDocumentEnvelopeError,
    PrivateDocumentRecipientKeys, PrivateDocumentTarget,
};
use zeroize::Zeroize;

/// One identity of the wallet and the keys entries to it may be encrypted to.
struct ScanRecipient {
    identity_id: Identifier,
    key_ids: Vec<KeyID>,
    keys: PrivateDocumentRecipientKeys,
}

/// One identity of the wallet and the outgoing viewing key it seals entries with.
struct ScanSender {
    identity_id: Identifier,
    outgoing_viewing_key: [u8; 32],
}

impl Drop for ScanSender {
    fn drop(&mut self) {
        self.outgoing_viewing_key.zeroize();
    }
}

/// The keys a wallet scans a private document type's store with: for each of its identities,
/// the secret keys entries to it may be encrypted to, and the outgoing viewing key it sealed its
/// own entries with.
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
        let secret_keys: Vec<SecretKey> = keys.iter().map(|(_, secret_key)| *secret_key).collect();
        self.recipients.push(ScanRecipient {
            identity_id,
            key_ids: keys.iter().map(|(key_id, _)| *key_id).collect(),
            keys: PrivateDocumentRecipientKeys::new(&secret_keys),
        });
        self
    }

    /// Adds `identity_id` as a sender, which sealed its entries with `outgoing_viewing_key`
    /// (`platform_encryption::private_document_outgoing_viewing_key`).
    pub fn with_sender(mut self, identity_id: Identifier, outgoing_viewing_key: [u8; 32]) -> Self {
        self.senders.push(ScanSender {
            identity_id,
            outgoing_viewing_key,
        });
        self
    }

    /// Sorts the entries of `page`, read from `start_position` of the store of `target`.
    pub fn scan_page(
        &self,
        target: &PrivateDocumentTarget,
        start_position: u64,
        page: &PrivateDocumentEntries,
    ) -> PrivateDocumentScan {
        let mut scan = PrivateDocumentScan {
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
        self.recipients
            .iter()
            .find(|recipient| recipient.identity_id == received.recipient_identity_id)
            .ok_or(PrivateDocumentEnvelopeError::RecipientSecretKeyRequired)?
            .keys
            .verify_sender(&received.entry, sender_public_key)
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
                    scan.sent.push(SentPrivateDocumentEntry {
                        position: entry.position,
                        block_height: entry.block_height,
                        block_time_ms: entry.block_time_ms,
                        sender_identity_id: sender.identity_id,
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

impl Sdk {
    /// Fetches the proved page of the store of the private type `document_type_name` of
    /// `data_contract_id` from `start_position`, and sorts its entries with `keys`.
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
            .with_sender(alice, ovk(0xA1));
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
                .map(|sent| (sent.position, sent.sender_identity_id))
                .collect::<Vec<_>>(),
            vec![(12, alice), (14, alice)]
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
                next_position: 7,
                total_count: 7,
                ..Default::default()
            }
        );
        assert!(scan.is_complete());
    }

    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_scan_a_fetched_page_of_the_store() {
        use crate::SdkBuilder;
        use dpp::version::PlatformVersion;

        let mut sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
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
