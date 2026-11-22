//! Where an identity's withdrawals to Core stand.
//!
//! An identity credit withdrawal stores a `withdrawal` document in the withdrawals system
//! contract, owned by the identity. Platform moves its `status` as the withdrawal travels to
//! Core: queued; pooled into an asset unlock transaction; broadcasted, which Platform records
//! when it builds that transaction for a quorum to sign and hands it on to Core, before Core
//! has accepted anything; then complete once Platform sees the transaction in a chain-locked
//! Core block, or expired when it does not and Platform builds it again. This module reads
//! those documents, proved, and returns them as typed rows.
//!
//! Withdrawals from Platform addresses and from the shielded pool store the same document,
//! but owned by the contract owner (the all-zero identifier), with an id derived from the
//! transition's first input or first spent note. Nothing indexes them by anything a wallet
//! keeps after the submit, so they are not found here.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::page::DocumentsPage;
use crate::platform::{Document, FetchMany, Identifier};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dpp::dashcore::{Address, Network, ScriptBuf};
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::Value;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::system_data_contracts::withdrawals_contract::{self, WithdrawalStatus};
use dpp::version::ProtocolVersion;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;

/// One withdrawal of an identity to Core, as the withdrawals contract stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityWithdrawal {
    /// The id of the withdrawal document.
    pub id: Identifier,
    /// The identity the credits were withdrawn from.
    pub owner_id: Identifier,
    /// The credits withdrawn, before the Core transaction fee.
    pub amount: Credits,
    /// The Core fee rate the withdrawal offered, in duffs per byte.
    pub core_fee_per_byte: u32,
    /// The Core output script the withdrawal pays.
    pub output_script: ScriptBuf,
    /// The Core address of `output_script` on the SDK's network; `None` for a script that is
    /// neither pay-to-public-key-hash nor pay-to-script-hash.
    pub destination: Option<Address>,
    /// Where the withdrawal stands.
    pub status: WithdrawalStatus,
    /// The index of the asset unlock transaction that carries the withdrawal to Core, set
    /// once Platform pooled it. Core identifies the transaction by it.
    pub transaction_index: Option<u64>,
    /// The Core chain-locked height Platform built that asset unlock transaction at (its
    /// request height), set when Platform marked the withdrawal broadcasted. It is not proof
    /// that a quorum signed the transaction or that Core accepted it; a withdrawal that
    /// expires and is built again gets a new one.
    pub transaction_sign_height: Option<u32>,
    /// When the withdrawal was submitted, block time in milliseconds.
    pub created_at: u64,
    /// When its status last changed, block time in milliseconds.
    pub updated_at: u64,
}

impl IdentityWithdrawal {
    /// Whether the withdrawal has reached a status it never leaves: complete, or failed
    /// (signed, but its payout is below Core's dust threshold, so Core never mines it).
    /// An expired withdrawal is not final: Platform signs it again.
    pub fn is_final(&self) -> bool {
        matches!(
            self.status,
            WithdrawalStatus::COMPLETE | WithdrawalStatus::FAILED
        )
    }

    /// The withdrawal `document` stores, with its destination read on `network`.
    pub fn try_from_document(document: &Document, network: Network) -> Result<Self, Error> {
        let id = document.id();
        let malformed = |what: &str| {
            Error::Generic(format!(
                "withdrawal document {} {what}",
                id.to_string(Encoding::Base58)
            ))
        };
        let properties = document.properties();
        let read = |key: &str| -> Result<u64, Error> {
            properties
                .get_integer(key)
                .map_err(|e| malformed(&format!("has no valid {key}: {e}")))
        };
        let read_optional = |key: &str| -> Result<Option<u64>, Error> {
            properties
                .get_optional_integer(key)
                .map_err(|e| malformed(&format!("has an invalid {key}: {e}")))
        };

        let status_code: u8 = properties
            .get_integer(withdrawal::properties::STATUS)
            .map_err(|e| malformed(&format!("has no valid status: {e}")))?;
        let status = WithdrawalStatus::try_from(status_code)
            .map_err(|_| malformed(&format!("has the unknown status {status_code}")))?;
        let core_fee_per_byte = u32::try_from(read(withdrawal::properties::CORE_FEE_PER_BYTE)?)
            .map_err(|_| malformed("has a Core fee rate above u32"))?;
        let transaction_sign_height =
            read_optional(withdrawal::properties::TRANSACTION_SIGN_HEIGHT)?
                .map(u32::try_from)
                .transpose()
                .map_err(|_| malformed("has a sign height above u32"))?;
        let output_script = ScriptBuf::from_bytes(
            properties
                .get_bytes(withdrawal::properties::OUTPUT_SCRIPT)
                .map_err(|e| malformed(&format!("has no valid output script: {e}")))?,
        );
        let destination = Address::from_script(&output_script, network).ok();

        Ok(Self {
            id,
            owner_id: document.owner_id(),
            amount: read(withdrawal::properties::AMOUNT)?,
            core_fee_per_byte,
            destination,
            output_script,
            status,
            transaction_index: read_optional(withdrawal::properties::TRANSACTION_INDEX)?,
            transaction_sign_height,
            created_at: document
                .created_at()
                .ok_or_else(|| malformed("has no creation time"))?,
            updated_at: document
                .updated_at()
                .ok_or_else(|| malformed("has no update time"))?,
        })
    }
}

/// One page of an identity's withdrawals, most recently updated first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityWithdrawalsPage {
    /// The withdrawals on this page.
    pub withdrawals: Vec<IdentityWithdrawal>,
    /// The page after this one, `None` when this was the last.
    pub next: Option<DocumentsPage>,
}

/// The query for one page of the withdrawals of `identity_id`, most recently updated first,
/// served by the contract's `identityRecent` index (`$ownerId`, `$updatedAt`, `status`).
fn identity_withdrawals_query(
    contract: Arc<DataContract>,
    identity_id: Identifier,
    page: DocumentsPage,
) -> Result<DocumentQuery, Error> {
    let mut query = DocumentQuery::new(contract, withdrawal::NAME)?
        .with_where(WhereClause {
            field: withdrawal::properties::OWNER_ID.to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(identity_id.to_buffer()),
        })
        // The order clause pins the index: a bare equality on `$ownerId` alone matches the
        // first property of two indexes.
        .with_order_by(OrderClause {
            field: withdrawal::properties::UPDATED_AT.to_string(),
            ascending: false,
        })
        .with_limit(page.page_size());
    query.start = page
        .start_after
        .map(|id| Start::StartAfter(id.to_buffer().to_vec()));
    Ok(query)
}

/// The rows of `documents`, the page that answered `page`.
fn identity_withdrawals_page(
    documents: &Documents,
    page: DocumentsPage,
    network: Network,
) -> Result<IdentityWithdrawalsPage, Error> {
    let withdrawals = documents
        .values()
        .flatten()
        .map(|document| IdentityWithdrawal::try_from_document(document, network))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IdentityWithdrawalsPage {
        withdrawals,
        next: page.after(documents),
    })
}

/// The first protocol version whose query lowering serves this listing. The listing is an
/// equality on `$ownerId` ordered by `$updatedAt` descending with a `startAfter` cursor; the
/// lowering before 14 walks the first page in the index's ascending order and treats an
/// empty branch at the cursor as a result, so it returns the oldest withdrawals and can stop
/// early.
const IDENTITY_WITHDRAWALS_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;

/// How many times [`Sdk::fetch_all_identity_withdrawals`] walks a listing of more than one
/// page before it gives up because every walk spanned more than one block.
const IDENTITY_WITHDRAWALS_READ_ATTEMPTS: usize = 3;

impl Sdk {
    /// One page of the withdrawals of `identity_id` to Core, most recently updated first,
    /// read with a proof from the withdrawals system contract. [`IdentityWithdrawalsPage::next`]
    /// gives the next page. Withdrawals move between pages as their status changes, so pages
    /// read at different heights neither list every withdrawal nor each one once; use
    /// [`Self::fetch_all_identity_withdrawals`] for the whole listing.
    ///
    /// Only withdrawals from the identity's own credit balance are found: withdrawals from
    /// Platform addresses and from the shielded pool are owned by the contract owner (see the
    /// module documentation). Needs protocol version 14 or later.
    pub async fn fetch_identity_withdrawals(
        &self,
        identity_id: Identifier,
        page: DocumentsPage,
    ) -> Result<IdentityWithdrawalsPage, Error> {
        self.fetch_identity_withdrawals_page_with_height(identity_id, page)
            .await
            .map(|(page, _)| page)
    }

    /// Every withdrawal of `identity_id` to Core, each listed once, most recently updated
    /// first, read with proofs as the listing stood at one block height.
    ///
    /// The listing is ordered by `$updatedAt`, which moves whenever Platform advances a
    /// withdrawal's status, so only pages proved at the same height make up the whole
    /// listing. When a walk's pages come back at different heights, the walk starts over, at
    /// most [`IDENTITY_WITHDRAWALS_READ_ATTEMPTS`] times; then the read fails and can be tried
    /// again. A listing whose first page is not full is one query, so it never needs a second
    /// walk. Needs protocol version 14 or later.
    pub async fn fetch_all_identity_withdrawals(
        &self,
        identity_id: Identifier,
    ) -> Result<Vec<IdentityWithdrawal>, Error> {
        read_all_withdrawals(|page| {
            self.fetch_identity_withdrawals_page_with_height(identity_id, page)
        })
        .await
    }

    /// One page of the withdrawals of `identity_id` and the block height it was proved at.
    async fn fetch_identity_withdrawals_page_with_height(
        &self,
        identity_id: Identifier,
        page: DocumentsPage,
    ) -> Result<(IdentityWithdrawalsPage, u64), Error> {
        self.require_protocol_version(
            IDENTITY_WITHDRAWALS_INITIAL_PROTOCOL_VERSION,
            "listing an identity's withdrawals",
        )
        .await?;
        let contract = self
            .fetch_system_data_contract(withdrawals_contract::ID)
            .await?
            .ok_or_else(|| {
                Error::MissingDependency(
                    "withdrawals contract".to_string(),
                    withdrawals_contract::ID.to_string(Encoding::Base58),
                )
            })?;
        let (documents, metadata) = Document::fetch_many_with_metadata(
            self,
            identity_withdrawals_query(contract, identity_id, page)?,
            None,
        )
        .await?;
        Ok((
            identity_withdrawals_page(&documents, page, self.network)?,
            metadata.height,
        ))
    }
}

/// Read the whole listing through `fetch_page`, which answers a page with the block height
/// it was proved at.
///
/// A walk is accepted only when every page came back at the same height and no withdrawal
/// appeared twice: at one height the listing does not change, so following the cursor lists
/// each withdrawal exactly once, in order. A walk that sees a second height or a repeated
/// withdrawal stops at once and starts over from the first page, at most
/// [`IDENTITY_WITHDRAWALS_READ_ATTEMPTS`] times; then the read fails.
async fn read_all_withdrawals<F, Fut>(mut fetch_page: F) -> Result<Vec<IdentityWithdrawal>, Error>
where
    F: FnMut(DocumentsPage) -> Fut,
    Fut: Future<Output = Result<(IdentityWithdrawalsPage, u64), Error>>,
{
    'walk: for _ in 0..IDENTITY_WITHDRAWALS_READ_ATTEMPTS {
        let mut withdrawals = Vec::new();
        let mut listed = BTreeSet::new();
        let mut walk_height = None;
        let mut page = Some(DocumentsPage::default());
        while let Some(current) = page {
            let (fetched, height) = fetch_page(current).await?;
            if *walk_height.get_or_insert(height) != height {
                continue 'walk;
            }
            for withdrawal in fetched.withdrawals {
                if !listed.insert(withdrawal.id) {
                    continue 'walk;
                }
                withdrawals.push(withdrawal);
            }
            page = fetched.next;
        }
        return Ok(withdrawals);
    }
    Err(Error::Generic(format!(
        "the identity's withdrawals span more than one page and could not be read at one \
         block height in {IDENTITY_WITHDRAWALS_READ_ATTEMPTS} attempts; read them again"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::PubkeyHash;
    use dpp::document::DocumentV0;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use drive::query::DriveDocumentQuery;
    use std::cell::Cell;
    use std::future::{ready, Ready};
    use std::rc::Rc;

    fn withdrawals_contract() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(SystemDataContract::Withdrawals, PlatformVersion::latest())
                .expect("the withdrawals contract loads at the latest version"),
        )
    }

    fn p2pkh_script() -> ScriptBuf {
        ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([7; 20]))
    }

    fn withdrawal_document(id: u8, properties: Value) -> Document {
        DocumentV0 {
            id: Identifier::from([id; 32]),
            owner_id: Identifier::from([3; 32]),
            properties: properties.into_btree_string_map().expect("a map"),
            revision: Some(1),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_600_000),
            ..Default::default()
        }
        .into()
    }

    #[test]
    fn should_read_a_queued_withdrawal_with_its_destination_and_no_core_transaction_yet() {
        let script = p2pkh_script();
        let document = withdrawal_document(
            1,
            platform_value!({
                "amount": 150_000u64,
                "coreFeePerByte": 1u32,
                "pooling": 0u8,
                "outputScript": Value::Bytes(script.to_bytes()),
                "status": 0u8,
            }),
        );

        let row = IdentityWithdrawal::try_from_document(&document, Network::Testnet)
            .expect("a valid withdrawal document");

        assert_eq!(row.id, Identifier::from([1; 32]));
        assert_eq!(row.owner_id, Identifier::from([3; 32]));
        assert_eq!(row.amount, 150_000);
        assert_eq!(row.core_fee_per_byte, 1);
        assert_eq!(row.status, WithdrawalStatus::QUEUED);
        assert_eq!(row.transaction_index, None);
        assert_eq!(row.transaction_sign_height, None);
        assert_eq!(row.created_at, 1_700_000_000_000);
        assert_eq!(row.updated_at, 1_700_000_600_000);
        assert_eq!(
            row.destination,
            Some(Address::from_script(&script, Network::Testnet).expect("p2pkh"))
        );
        assert_eq!(row.output_script, script);
        assert!(!row.is_final());
    }

    #[test]
    fn should_read_the_core_transaction_of_a_broadcast_withdrawal() {
        let document = withdrawal_document(
            2,
            platform_value!({
                "amount": 150_000u64,
                "coreFeePerByte": 1u32,
                "pooling": 0u8,
                "outputScript": Value::Bytes(p2pkh_script().to_bytes()),
                "status": 2u8,
                "transactionIndex": 42u64,
                "transactionSignHeight": 1_234_567u32,
            }),
        );

        let row = IdentityWithdrawal::try_from_document(&document, Network::Testnet)
            .expect("a valid withdrawal document");

        assert_eq!(row.status, WithdrawalStatus::BROADCASTED);
        assert_eq!(row.transaction_index, Some(42));
        assert_eq!(row.transaction_sign_height, Some(1_234_567));
        assert!(!row.is_final());
    }

    #[test]
    fn should_treat_only_complete_and_failed_as_final() {
        let finality = |status: u8| {
            let document = withdrawal_document(
                3,
                platform_value!({
                    "amount": 150_000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": 0u8,
                    "outputScript": Value::Bytes(p2pkh_script().to_bytes()),
                    "status": status,
                }),
            );
            IdentityWithdrawal::try_from_document(&document, Network::Testnet)
                .expect("a valid withdrawal document")
                .is_final()
        };
        assert!(!finality(0), "queued");
        assert!(!finality(1), "pooled");
        assert!(!finality(2), "broadcasted");
        assert!(finality(3), "complete");
        assert!(!finality(4), "expired is signed again");
        assert!(finality(5), "failed");
    }

    #[test]
    fn should_refuse_a_document_with_an_unknown_status() {
        let document = withdrawal_document(
            4,
            platform_value!({
                "amount": 150_000u64,
                "coreFeePerByte": 1u32,
                "pooling": 0u8,
                "outputScript": Value::Bytes(p2pkh_script().to_bytes()),
                "status": 9u8,
            }),
        );

        let error = IdentityWithdrawal::try_from_document(&document, Network::Testnet)
            .expect_err("an unknown status is refused");
        assert!(error.to_string().contains("unknown status 9"), "{error}");
    }

    #[test]
    fn should_leave_the_destination_empty_for_a_script_that_is_not_an_address() {
        let document = withdrawal_document(
            5,
            platform_value!({
                "amount": 150_000u64,
                "coreFeePerByte": 1u32,
                "pooling": 0u8,
                "outputScript": Value::Bytes(vec![0x6a; 23]),
                "status": 0u8,
            }),
        );

        let row = IdentityWithdrawal::try_from_document(&document, Network::Testnet)
            .expect("a valid withdrawal document");
        assert_eq!(row.destination, None);
    }

    #[test]
    fn should_serve_the_listing_through_the_identity_recent_index_newest_first() {
        let platform_version = PlatformVersion::latest();
        let page = DocumentsPage {
            limit: Some(20),
            start_after: Some(Identifier::from([5; 32])),
        };
        let query =
            identity_withdrawals_query(withdrawals_contract(), Identifier::from([3; 32]), page)
                .expect("builds");

        let drive_query = DriveDocumentQuery::try_from(&query).expect("converts");
        drive_query
            .construct_path_query(None, platform_version)
            .expect("a provable path query");
        let index = drive_query
            .find_best_index(platform_version)
            .expect("an index serves it");
        assert_eq!(index.name, "identityRecent");
        assert_eq!(query.limit, 20);
        assert_eq!(
            query.start,
            Some(Start::StartAfter(
                Identifier::from([5; 32]).to_buffer().to_vec()
            ))
        );
        assert!(!query.order_by_clauses[0].ascending);
    }

    #[test]
    fn should_offer_a_next_page_only_after_a_full_page() {
        let row = |id: u8| {
            withdrawal_document(
                id,
                platform_value!({
                    "amount": 150_000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": 0u8,
                    "outputScript": Value::Bytes(p2pkh_script().to_bytes()),
                    "status": 0u8,
                }),
            )
        };
        let page = DocumentsPage {
            limit: Some(2),
            start_after: None,
        };
        let full: Documents = [row(1), row(2)]
            .into_iter()
            .map(|document| (document.id(), Some(document)))
            .collect();
        let short: Documents = [row(1)]
            .into_iter()
            .map(|document| (document.id(), Some(document)))
            .collect();

        let full_page = identity_withdrawals_page(&full, page, Network::Testnet).expect("decodes");
        assert_eq!(full_page.withdrawals.len(), 2);
        assert_eq!(
            full_page.next,
            Some(DocumentsPage {
                limit: Some(2),
                start_after: Some(Identifier::from([2; 32])),
            })
        );
        let short_page =
            identity_withdrawals_page(&short, page, Network::Testnet).expect("decodes");
        assert_eq!(short_page.withdrawals.len(), 1);
        assert_eq!(short_page.next, None);
    }

    fn row_updated(id: u8, status: u8, updated_at: u64) -> IdentityWithdrawal {
        let mut row = IdentityWithdrawal::try_from_document(
            &withdrawal_document(
                id,
                platform_value!({
                    "amount": 150_000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": 0u8,
                    "outputScript": Value::Bytes(p2pkh_script().to_bytes()),
                    "status": status,
                }),
            ),
            Network::Testnet,
        )
        .expect("a valid withdrawal document");
        row.updated_at = updated_at;
        row
    }

    /// The listing as it stood at one block height: most recently updated first.
    struct Snapshot {
        height: u64,
        withdrawals: Vec<IdentityWithdrawal>,
    }

    /// A page reader over `snapshots` that honours the requested cursor and limit: call `n`
    /// is answered from the snapshot `schedule[n]` names (the last one once the schedule runs
    /// out), the way a node at that height resolves `startAfter` against its own listing.
    fn ledger(
        snapshots: Vec<Snapshot>,
        schedule: Vec<usize>,
        calls: Rc<Cell<usize>>,
    ) -> impl FnMut(DocumentsPage) -> Ready<Result<(IdentityWithdrawalsPage, u64), Error>> {
        move |page: DocumentsPage| {
            let call = calls.get();
            calls.set(call + 1);
            let snapshot = &snapshots[*schedule.get(call).or(schedule.last()).expect("a schedule")];
            let start = match page.start_after {
                None => 0,
                Some(cursor) => snapshot
                    .withdrawals
                    .iter()
                    .position(|w| w.id == cursor)
                    .map_or(snapshot.withdrawals.len(), |at| at + 1),
            };
            let size = page.page_size() as usize;
            let withdrawals: Vec<_> = snapshot
                .withdrawals
                .iter()
                .skip(start)
                .take(size)
                .cloned()
                .collect();
            let next = (withdrawals.len() == size).then(|| DocumentsPage {
                limit: page.limit,
                start_after: withdrawals.last().map(|w| w.id),
            });
            ready(Ok((
                IdentityWithdrawalsPage { withdrawals, next },
                snapshot.height,
            )))
        }
    }

    /// Withdrawals 1..=count, the higher id the more recently updated, all queued.
    fn listing(count: u8) -> Vec<IdentityWithdrawal> {
        (1..=count)
            .rev()
            .map(|id| row_updated(id, 0, id as u64))
            .collect()
    }

    #[tokio::test]
    async fn should_read_a_listing_that_fits_one_page_with_one_query() {
        let calls = Rc::new(Cell::new(0));
        let snapshots = vec![Snapshot {
            height: 7,
            withdrawals: listing(3),
        }];
        let withdrawals = read_all_withdrawals(ledger(snapshots, vec![0], calls.clone()))
            .await
            .expect("reads");
        assert_eq!(withdrawals, listing(3));
        assert_eq!(calls.get(), 1);
    }

    #[tokio::test]
    async fn should_list_every_withdrawal_once_in_order_when_every_page_is_at_one_height() {
        let calls = Rc::new(Cell::new(0));
        let snapshots = vec![Snapshot {
            height: 7,
            withdrawals: listing(250),
        }];
        let withdrawals = read_all_withdrawals(ledger(snapshots, vec![0], calls.clone()))
            .await
            .expect("reads");
        assert_eq!(withdrawals, listing(250));
        // 100 + 100 + 50.
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test]
    async fn should_walk_again_when_a_pending_withdrawal_moves_to_the_top_between_pages() {
        // At height 7, 100 final withdrawals sit above the only pending one. At height 8 the
        // pending one has advanced to the top, so a continuation read there after the first
        // page's cursor finds nothing below it and the pending withdrawal would be lost.
        let mut at_7: Vec<_> = (2..=101u8)
            .rev()
            .map(|id| row_updated(id, 3, 100 + id as u64))
            .collect();
        at_7.push(row_updated(1, 0, 1));
        let mut at_8 = at_7.clone();
        let pending = at_8.pop().expect("the pending withdrawal");
        at_8.insert(
            0,
            IdentityWithdrawal {
                status: WithdrawalStatus::POOLED,
                updated_at: 500,
                ..pending
            },
        );
        let snapshots = vec![
            Snapshot {
                height: 7,
                withdrawals: at_7,
            },
            Snapshot {
                height: 8,
                withdrawals: at_8.clone(),
            },
        ];
        let calls = Rc::new(Cell::new(0));
        // The first page comes from height 7, every later read from height 8.
        let withdrawals = read_all_withdrawals(ledger(snapshots, vec![0, 1], calls.clone()))
            .await
            .expect("reads");

        assert_eq!(withdrawals, at_8);
        assert_eq!(withdrawals[0].status, WithdrawalStatus::POOLED);
        // Two reads of the first walk, then two of the walk at height 8.
        assert_eq!(calls.get(), 4);
    }

    #[tokio::test]
    async fn should_fail_rather_than_return_a_partial_listing_when_no_walk_stays_at_one_height() {
        let snapshots = vec![
            Snapshot {
                height: 7,
                withdrawals: listing(150),
            },
            Snapshot {
                height: 8,
                withdrawals: listing(150),
            },
        ];
        let calls = Rc::new(Cell::new(0));
        // Every walk's pages alternate between heights 7 and 8.
        let schedule = (0..100).map(|call| call % 2).collect();
        let error = read_all_withdrawals(ledger(snapshots, schedule, calls.clone()))
            .await
            .expect_err("no walk is at one height");

        assert!(
            error
                .to_string()
                .contains("could not be read at one block height"),
            "{error}"
        );
        // Each walk stops at its second page.
        assert_eq!(calls.get(), 2 * IDENTITY_WITHDRAWALS_READ_ATTEMPTS);
    }

    #[tokio::test]
    async fn should_walk_again_when_a_page_repeats_a_withdrawal_already_listed() {
        // A node answering every page with the first one: the second page repeats the first.
        let first = listing(150)[..100].to_vec();
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        let fetch = move |_page: DocumentsPage| {
            counter.set(counter.get() + 1);
            let next = Some(DocumentsPage {
                limit: None,
                start_after: first.last().map(|w| w.id),
            });
            ready(Ok((
                IdentityWithdrawalsPage {
                    withdrawals: first.clone(),
                    next,
                },
                7,
            )))
        };
        let error = read_all_withdrawals(fetch)
            .await
            .expect_err("never consistent");
        assert!(
            error
                .to_string()
                .contains("could not be read at one block height"),
            "{error}"
        );
        assert_eq!(calls.get(), 2 * IDENTITY_WITHDRAWALS_READ_ATTEMPTS);
    }
}
