//! A page of a document's retained revisions with its lifecycle, read from
//! whichever history layout the protocol version stores.

use crate::drive::document::lifecycle::DocumentLifecycleRecord;
use crate::drive::document::paths::{
    contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY, DOCUMENT_LIFECYCLE_TREE_KEY,
};
use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{Element, PathQuery, Query, SizedQuery};
use std::collections::BTreeMap;

/// Exactly one lower bound or revision position for a history page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DocumentHistoryFilter {
    /// Inclusive time bound for the first page.
    StartAtTime(u64),
    /// Exclusive composite cursor for subsequent pages.
    StartAfter {
        /// Timestamp of the last entry received.
        time_ms: u64,
        /// History sequence of the last entry received.
        revision: u64,
    },
    /// Inclusive retained revision position.
    StartAtRevision(u64),
    /// One retained revision position.
    Revision(u64),
}

/// A page of one historical document's retained revisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHistoryDriveQuery {
    /// Contract identifier.
    pub contract_id: [u8; 32],
    /// Document type name.
    pub document_type_name: String,
    /// Document identifier.
    pub document_id: [u8; 32],
    /// Page filter.
    pub filter: DocumentHistoryFilter,
    /// Page length, at most ten.
    pub limit: Option<u16>,
}

/// Lifecycle states supported by live-pointer storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentHistoryState {
    /// A current document is present.
    Active,
    /// Neither a current document nor a retained history exists.
    Absent,
    /// The document has been deleted; its revisions are still retained.
    Deleted,
    /// An authorized erasure has started and has not finished.
    Erasing,
}

/// Authenticated lifecycle timestamps independent of the requested page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentHistoryLifecycleTimes {
    /// Block time the document was deleted at, zero while it is active.
    pub deleted_at_ms: u64,
    /// Block time an authorized erasure started at, zero while none has.
    pub erasing_started_at_ms: u64,
    /// Timestamp of the newest revision retained when the erasure started.
    pub erasing_from_time_ms: u64,
    /// History sequence of the newest revision retained when the erasure
    /// started.
    pub erasing_from_revision: u64,
}

/// Authenticated history metadata independent of the requested page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHistoryLifecycle {
    /// Current lifecycle state.
    pub state: DocumentHistoryState,
    /// Count authenticated by the per-document count-tree element.
    pub remaining_revisions: u64,
    /// Times recorded by the lifecycle record, all zero unless one exists.
    pub times: DocumentHistoryLifecycleTimes,
}

/// One retained edit, including its complete pagination cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentHistoryEntry {
    /// Block timestamp in milliseconds.
    pub time_ms: u64,
    /// History sequence, equal to the document revision when present.
    pub revision: u64,
    /// Document body at this revision.
    pub document: Document,
}

/// The result of a history query: a page and the lifecycle of the whole
/// history.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentHistoryDriveQueryExecutionResult {
    /// Ordered retained revisions in this page.
    pub entries: Vec<DocumentHistoryEntry>,
    /// Metadata for the whole history, including empty pages. Absent when
    /// the page was read from the layout that predates protocol version 15,
    /// which keeps no lifecycle record.
    pub lifecycle: Option<DocumentHistoryLifecycle>,
}

impl DocumentHistoryDriveQueryExecutionResult {
    /// Builds a page from revisions read from the per-document history
    /// subtree used before protocol version 15, keyed by block time.
    pub(crate) fn from_legacy(revisions: BTreeMap<u64, Document>) -> Result<Self, Error> {
        let entries = revisions
            .into_iter()
            .map(|(time_ms, document)| {
                let revision = document.revision().unwrap_or(1);
                Ok(DocumentHistoryEntry {
                    time_ms,
                    revision,
                    document,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            entries,
            lifecycle: None,
        })
    }
}

pub(crate) fn invalid(message: &str) -> Error {
    Error::Query(QuerySyntaxError::Unsupported(message.to_owned()))
}

pub(crate) fn corrupt(message: &'static str) -> Error {
    Error::Drive(DriveError::CorruptedDocumentPath(message))
}

impl DocumentHistoryDriveQuery {
    /// Validates the filter without accessing state.
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=MAX_DOCUMENT_HISTORY_FETCH_LIMIT)
            .contains(&self.limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT))
        {
            return Err(invalid("history limit must be between one and ten"));
        }
        match self.filter {
            DocumentHistoryFilter::StartAtTime(time)
            | DocumentHistoryFilter::StartAfter { time_ms: time, .. }
                if time >= (1u64 << 63) =>
            {
                Err(invalid("history time must be below 2^63"))
            }
            DocumentHistoryFilter::StartAfter { revision: 0, .. } => {
                Err(invalid("history cursor revision must be positive"))
            }
            DocumentHistoryFilter::StartAtRevision(revision)
            | DocumentHistoryFilter::Revision(revision)
                if !(1..=u16::MAX as u64).contains(&revision) =>
            {
                Err(invalid("history revision must be between one and 65535"))
            }
            DocumentHistoryFilter::Revision(_) if self.limit.is_some_and(|limit| limit != 1) => {
                Err(invalid("single revision queries require limit one"))
            }
            _ => Ok(()),
        }
    }

    /// The path query of the page's entries in the layout the protocol
    /// version stores.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history_query
        {
            0 => {
                let (start_at_ms, limit) = self.legacy_read()?;
                Drive::fetch_document_history_query(
                    self.contract_id,
                    &self.document_type_name,
                    self.document_id,
                    start_at_ms,
                    limit,
                    None,
                    platform_version,
                )
            }
            1 => Drive::fetch_document_history_drive_query_v1(self),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentHistoryDriveQuery::construct_path_query".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Maps the query onto the per-document subtree used before protocol
    /// version 15, which is keyed by block time only: a time filter and a
    /// page length. Revision and cursor filters need the history tree.
    ///
    /// That layout's reader takes an exclusive time bound, so the inclusive
    /// `StartAtTime` bound is lowered by one millisecond to keep the filter's
    /// meaning the same on both sides of activation. A revision stored at
    /// block time zero stays unreachable through it, as it always was.
    pub(crate) fn legacy_read(&self) -> Result<(u64, Option<u16>), Error> {
        self.validate()?;
        match self.filter {
            DocumentHistoryFilter::StartAtTime(time_ms) => {
                Ok((time_ms.saturating_sub(1), self.limit))
            }
            DocumentHistoryFilter::StartAfter { .. }
            | DocumentHistoryFilter::StartAtRevision(_)
            | DocumentHistoryFilter::Revision(_) => Err(invalid(
                "revision and cursor filters need the protocol version 15 history layout",
            )),
        }
    }

    /// Queries the pointer, lifecycle reservation, and raw history tree separately.
    pub fn metadata_path_query(&self, version: &PlatformVersion) -> Result<PathQuery, Error> {
        self.validate()?;
        let queries = [
            0,
            DOCUMENT_LIFECYCLE_TREE_KEY,
            DOCUMENT_HISTORY_TREE_KEY,
        ]
        .map(|branch| {
            let mut path =
                contract_document_type_path_vec(&self.contract_id, &self.document_type_name);
            path.push(vec![branch]);
            let mut query = Query::new();
            query.insert_key(self.document_id.to_vec());
            PathQuery::new(path, SizedQuery::new(query, None, None))
        });
        let mut merged = PathQuery::merge(queries.iter().collect(), &version.drive.grove_version)?;
        // One exact key per branch. The finite limit lets the verifier map
        // every queried key to a proven element or a proven absence.
        merged.query.limit = Some(queries.len() as u16);
        Ok(merged)
    }

    pub(crate) fn lifecycle(
        &self,
        metadata: Vec<grovedb::query_result_type::PathKeyOptionalElementTrio>,
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<(DocumentHistoryLifecycle, bool), Error> {
        let mut primary =
            contract_document_type_path_vec(&self.contract_id, &self.document_type_name);
        primary.push(vec![0]);
        let mut history = primary.clone();
        history[4] = vec![DOCUMENT_HISTORY_TREE_KEY];
        let mut lifecycle = primary.clone();
        lifecycle[4] = vec![DOCUMENT_LIFECYCLE_TREE_KEY];
        let mut active = false;
        let mut latest_revision = None;
        let mut count = None;
        let mut record = None;
        for (path, key, element) in metadata {
            let Some(element) = element else {
                continue;
            };
            if key != self.document_id {
                return Err(corrupt("history metadata key does not match the document"));
            }
            if path == lifecycle {
                let Element::Item(bytes, _) = element else {
                    return Err(corrupt("a lifecycle record is not an item"));
                };
                if record
                    .replace(DocumentLifecycleRecord::deserialize(&bytes)?)
                    .is_some()
                {
                    return Err(corrupt("duplicate lifecycle record"));
                }
            } else if path == primary {
                let bytes = match element {
                    Element::Item(bytes, _) | Element::ItemWithSumItem(bytes, _, _) => bytes,
                    _ => {
                        return Err(corrupt(
                            "history current pointer did not resolve to a document",
                        ))
                    }
                };
                let document = Document::from_bytes(&bytes, document_type, version)?;
                if document.id().as_slice() != self.document_id {
                    return Err(corrupt(
                        "history current document id does not match the query",
                    ));
                }
                latest_revision = Some(document.revision().unwrap_or(1));
                if active {
                    return Err(corrupt("duplicate history current pointer"));
                }
                active = true;
            } else if path == history {
                let Element::ProvableCountTree(_, revisions, _) = element else {
                    return Err(corrupt("history metadata is not a provable count tree"));
                };
                if count.replace(revisions).is_some() {
                    return Err(corrupt("duplicate history metadata"));
                }
            } else {
                return Err(corrupt(
                    "unexpected lifecycle record or history metadata path",
                ));
            }
        }
        if active && record.is_some() {
            return Err(corrupt(
                "a current document cannot also carry a lifecycle record",
            ));
        }
        if active && count.unwrap_or_default() == 0
            || !active && record.is_none() && count.unwrap_or_default() > 0
        {
            return Err(corrupt("current document and retained history disagree"));
        }
        if record.is_some() && count.unwrap_or_default() == 0 {
            return Err(corrupt("a lifecycle record survives its retained history"));
        }
        if matches!(
            self.filter,
            DocumentHistoryFilter::Revision(_) | DocumentHistoryFilter::StartAtRevision(_)
        ) && active
            && latest_revision != count
        {
            return Err(invalid(
                "retained history contains a revision gap; use time pagination",
            ));
        }
        let state = match (active, &record) {
            (true, _) => DocumentHistoryState::Active,
            (false, Some(record)) if record.is_erasing() => DocumentHistoryState::Erasing,
            (false, Some(_)) => DocumentHistoryState::Deleted,
            // Retained history with no record and no pointer was rejected as
            // inconsistent above, so nothing is left here but an unused id.
            (false, None) => DocumentHistoryState::Absent,
        };
        let times = record
            .map(|record| DocumentHistoryLifecycleTimes {
                deleted_at_ms: record.deleted_at_ms(),
                erasing_started_at_ms: record.erasing_started_at_ms(),
                erasing_from_time_ms: record.erasing_from_time_ms(),
                erasing_from_revision: record.erasing_from_revision(),
            })
            .unwrap_or_default();
        Ok((
            DocumentHistoryLifecycle {
                state,
                remaining_revisions: count.unwrap_or_default(),
                times,
            },
            count.is_some(),
        ))
    }

    pub(crate) fn decode_entries(
        &self,
        entries: Vec<(Vec<u8>, Element)>,
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<Vec<DocumentHistoryEntry>, Error> {
        entries
            .into_iter()
            .enumerate()
            .map(|(position, (key, element))| {
                if key.len() != 16 {
                    return Err(corrupt(
                        "history revision keys must contain exactly sixteen bytes",
                    ));
                }
                let time_ms = DocumentPropertyType::decode_date_timestamp(&key[..8])
                    .ok_or_else(|| corrupt("invalid history timestamp"))?;
                let revision = DocumentPropertyType::decode_date_timestamp(&key[8..])
                    .ok_or_else(|| corrupt("invalid history revision"))?;
                if time_ms >= (1u64 << 63) || revision == 0 {
                    return Err(corrupt(
                        "history key contains an invalid timestamp or revision",
                    ));
                }
                let Element::Item(bytes, _) = element else {
                    return Err(corrupt("history revision is not a plain item"));
                };
                let document = Document::from_bytes(&bytes, document_type, version)?;
                if document.id().as_slice() != self.document_id
                    || document.revision().unwrap_or(1) != revision
                {
                    return Err(corrupt("history revision does not match its key"));
                }
                if let DocumentHistoryFilter::StartAtRevision(requested)
                | DocumentHistoryFilter::Revision(requested) = self.filter
                {
                    if revision != requested + position as u64 {
                        return Err(invalid(
                            "retained history contains a revision gap; use time pagination",
                        ));
                    }
                }
                Ok(DocumentHistoryEntry {
                    time_ms,
                    revision,
                    document,
                })
            })
            .collect()
    }

    /// Fetches the page in the layout the protocol version stores.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentHistoryDriveQueryExecutionResult, Error> {
        drive.fetch_document_history(self, document_type, transaction, platform_version)
    }

    /// Proves the page in the layout the protocol version stores.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        drive.prove_document_history(self, document_type, transaction, platform_version)
    }

    /// Verifies a proof produced by [`Self::execute_with_proof`].
    pub fn verify_document_history_proof(
        &self,
        proof: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
        Drive::verify_document_history(self, proof, document_type, platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_sequence_one_for_a_legacy_immutable_document_without_a_revision() {
        let document = Document::V0(Default::default());
        assert_eq!(document.revision(), None);

        let history = DocumentHistoryDriveQueryExecutionResult::from_legacy(BTreeMap::from([(
            2000,
            document.clone(),
        )]))
        .expect("an immutable legacy document has sequence one");

        assert_eq!(
            history.entries,
            vec![DocumentHistoryEntry {
                time_ms: 2000,
                revision: 1,
                document,
            }]
        );
    }
}
