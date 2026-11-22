//! Where an identity's withdrawals to Core stand.
//!
//! An identity credit withdrawal stores a `withdrawal` document in the withdrawals system
//! contract, owned by the identity. Platform moves its `status` as the withdrawal travels to
//! Core: queued, pooled into an asset unlock transaction that a quorum signs, broadcast to
//! Core, then complete once Core includes it in a chain-locked block, or expired when Core
//! never does and Platform signs it again. This module reads those documents, proved, and
//! returns them as typed rows.
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
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
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
    /// The Core height at which a quorum signed that transaction, set once it was signed.
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

impl Sdk {
    /// One page of the withdrawals of `identity_id` to Core, most recently updated first,
    /// read with a proof from the withdrawals system contract. [`IdentityWithdrawalsPage::next`]
    /// gives the next page.
    ///
    /// Only withdrawals from the identity's own credit balance are found: withdrawals from
    /// Platform addresses and from the shielded pool are owned by the contract owner (see the
    /// module documentation).
    pub async fn fetch_identity_withdrawals(
        &self,
        identity_id: Identifier,
        page: DocumentsPage,
    ) -> Result<IdentityWithdrawalsPage, Error> {
        let contract = self
            .fetch_system_data_contract(withdrawals_contract::ID)
            .await?
            .ok_or_else(|| {
                Error::MissingDependency(
                    "withdrawals contract".to_string(),
                    withdrawals_contract::ID.to_string(Encoding::Base58),
                )
            })?;
        let documents = Document::fetch_many(
            self,
            identity_withdrawals_query(contract, identity_id, page)?,
        )
        .await?;
        identity_withdrawals_page(&documents, page, self.network)
    }
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
}
