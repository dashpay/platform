//! History selectors, authenticated lifecycle metadata, and composite revision keys.
use crate::drive::document::paths::{
    contract_document_type_path_vec, document_history_path, DOCUMENT_HISTORY_TREE_KEY,
};
use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::{drive::DriveError, query::QuerySyntaxError, Error};
use crate::util::common::encode::encode_u64;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, Query, SizedQuery};

/// Exactly one lower bound or revision position for a history page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DocumentHistorySelector {
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

/// Query for composite-keyed document history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHistoryQueryV1 {
    /// Contract identifier.
    pub contract_id: [u8; 32],
    /// Document type name.
    pub document_type_name: String,
    /// Document identifier.
    pub document_id: [u8; 32],
    /// Page selector.
    pub selector: DocumentHistorySelector,
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
}

/// Authenticated history metadata independent of the requested page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHistoryLifecycle {
    /// Current lifecycle state.
    pub state: DocumentHistoryState,
    /// Count authenticated by the per-document count-tree element.
    pub remaining_revisions: u64,
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

/// A history page and lifecycle metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentHistoryV1 {
    /// Ordered retained revisions in this page.
    pub entries: Vec<DocumentHistoryEntry>,
    /// Metadata for the whole history, including empty pages.
    pub lifecycle: DocumentHistoryLifecycle,
}

/// Separate leaf pagination and metadata proofs.
#[derive(Debug, Clone)]
pub struct DocumentHistoryProofV1 {
    /// Omitted only if metadata proves that the history tree is absent.
    pub entries_proof: Option<Vec<u8>>,
    /// Proves the current pointer and the raw history count-tree element.
    pub metadata_proof: Vec<u8>,
}

fn invalid(message: &str) -> Error {
    Error::Query(QuerySyntaxError::Unsupported(message.to_owned()))
}

fn corrupt(message: &'static str) -> Error {
    Error::Drive(DriveError::CorruptedDocumentPath(message))
}

impl DocumentHistoryQueryV1 {
    /// Validates the selector without accessing state.
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=MAX_DOCUMENT_HISTORY_FETCH_LIMIT)
            .contains(&self.limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT))
        {
            return Err(invalid("history limit must be between one and ten"));
        }
        match self.selector {
            DocumentHistorySelector::StartAtTime(time)
            | DocumentHistorySelector::StartAfter { time_ms: time, .. }
                if time >= (1u64 << 63) =>
            {
                Err(invalid("history time must be below 2^63"))
            }
            DocumentHistorySelector::StartAfter { revision: 0, .. } => {
                Err(invalid("history cursor revision must be positive"))
            }
            DocumentHistorySelector::StartAtRevision(revision)
            | DocumentHistorySelector::Revision(revision)
                if !(1..=u16::MAX as u64).contains(&revision) =>
            {
                Err(invalid("history revision must be between one and 65535"))
            }
            DocumentHistorySelector::Revision(_) if self.limit.is_some_and(|limit| limit != 1) => {
                Err(invalid("single revision queries require limit one"))
            }
            _ => Ok(()),
        }
    }

    /// The leaf-only query required for authenticated count-offset pagination.
    pub fn entries_query(&self, version: &PlatformVersion) -> Result<PathQuery, Error> {
        self.validate()?;
        if version
            .drive
            .methods
            .document
            .query
            .fetch_document_history_query
            != 1
        {
            return Err(invalid(
                "composite document history is unavailable at this protocol version",
            ));
        }
        let mut query = Query::new();
        let mut limit = self.limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT);
        let offset = match self.selector {
            DocumentHistorySelector::StartAtTime(time) => {
                query.insert_range_from(encode_u64(time)..);
                None
            }
            DocumentHistorySelector::StartAfter { time_ms, revision } => {
                let mut key = encode_u64(time_ms);
                key.extend(encode_u64(revision));
                query.insert_range_after(key..);
                None
            }
            DocumentHistorySelector::StartAtRevision(revision)
            | DocumentHistorySelector::Revision(revision) => {
                query.insert_all();
                if matches!(self.selector, DocumentHistorySelector::Revision(_)) {
                    limit = 1;
                }
                (revision > 1).then_some((revision - 1) as u16)
            }
        };
        Ok(PathQuery::new(
            document_history_path(
                &self.contract_id,
                &self.document_type_name,
                &self.document_id,
            ),
            SizedQuery::new(query, Some(limit), offset),
        ))
    }

    /// Queries the pointer, lifecycle reservation, and raw history tree separately.
    pub fn metadata_query(&self, version: &PlatformVersion) -> Result<PathQuery, Error> {
        self.validate()?;
        let queries = [0, 1, DOCUMENT_HISTORY_TREE_KEY].map(|branch| {
            let mut path =
                contract_document_type_path_vec(&self.contract_id, &self.document_type_name);
            path.push(vec![branch]);
            let mut query = Query::new();
            query.insert_key(self.document_id.to_vec());
            PathQuery::new(path, SizedQuery::new(query, None, None))
        });
        Ok(PathQuery::merge(
            queries.iter().collect(),
            &version.drive.grove_version,
        )?)
    }

    fn lifecycle(
        &self,
        metadata: Vec<grovedb::query_result_type::PathKeyOptionalElementTrio>,
    ) -> Result<(DocumentHistoryLifecycle, bool), Error> {
        let mut primary =
            contract_document_type_path_vec(&self.contract_id, &self.document_type_name);
        primary.push(vec![0]);
        let mut history = primary.clone();
        history[4] = vec![DOCUMENT_HISTORY_TREE_KEY];
        let mut active = false;
        let mut count = None;
        for (path, key, element) in metadata {
            let Some(element) = element else {
                continue;
            };
            if key != self.document_id {
                return Err(corrupt("history metadata key does not match the document"));
            }
            if path == primary {
                if !matches!(
                    element,
                    Element::Item(..)
                        | Element::ItemWithSumItem(..)
                        | Element::Reference(..)
                        | Element::ReferenceWithSumItem(..)
                ) {
                    return Err(corrupt(
                        "history current pointer has an invalid element type",
                    ));
                }
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
        if active && count.unwrap_or_default() == 0 || !active && count.unwrap_or_default() > 0 {
            return Err(corrupt("current document and retained history disagree"));
        }
        Ok((
            DocumentHistoryLifecycle {
                state: if active {
                    DocumentHistoryState::Active
                } else {
                    DocumentHistoryState::Absent
                },
                remaining_revisions: count.unwrap_or_default(),
            },
            count.is_some(),
        ))
    }

    fn decode_entries(
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
                if let DocumentHistorySelector::StartAtRevision(requested)
                | DocumentHistorySelector::Revision(requested) = self.selector
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
}

#[cfg(feature = "server")]
impl Drive {
    /// Fetches a composite-keyed page and its current lifecycle metadata.
    pub fn fetch_document_history_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryV1, Error> {
        self.fetch_document_history_with_presence(query, document_type, transaction, version)
            .map(|(history, _)| history)
    }

    fn fetch_document_history_with_presence(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<(DocumentHistoryV1, bool), Error> {
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let entries_query = query.entries_query(version)?;
        if version.drive.methods.document.query.fetch_document_history != 1 {
            return Err(invalid("document history fetch version is unsupported"));
        }
        let metadata_query = query.metadata_query(version)?;
        let (metadata, _) = self.grove_get_raw_path_query(
            &metadata_query,
            transaction,
            grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &version.drive,
        )?;
        let (lifecycle, present) = query.lifecycle(
            metadata
                .to_path_key_elements()
                .into_iter()
                .map(|(path, key, element)| (path, key, Some(element)))
                .collect(),
        )?;
        let entries = if present {
            let (entries, _) = self.grove_get_raw_path_query(
                &entries_query,
                transaction,
                grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &version.drive,
            )?;
            query.decode_entries(entries.to_key_elements(), document_type, version)?
        } else {
            vec![]
        };
        Ok((DocumentHistoryV1 { entries, lifecycle }, present))
    }

    /// Produces independent pagination and metadata proofs from the same state.
    pub fn prove_document_history_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<(DocumentHistoryV1, DocumentHistoryProofV1), Error> {
        if version.drive.methods.document.query.prove_document_history != 1 {
            return Err(invalid("document history proof version is unsupported"));
        }
        let (history, present) =
            self.fetch_document_history_with_presence(query, document_type, transaction, version)?;
        let metadata_proof = self.grove_get_proved_path_query(
            &query.metadata_query(version)?,
            transaction,
            &mut vec![],
            &version.drive,
        )?;
        let entries_proof = if !present {
            None
        } else {
            Some(self.grove_get_proved_path_query(
                &query.entries_query(version)?,
                transaction,
                &mut vec![],
                &version.drive,
            )?)
        };
        Ok((
            history,
            DocumentHistoryProofV1 {
                entries_proof,
                metadata_proof,
            },
        ))
    }
}

#[cfg(feature = "verify")]
impl Drive {
    /// Verifies both proofs, their common root, and the revision positions.
    pub fn verify_document_history_v1(
        query: &DocumentHistoryQueryV1,
        proof: &DocumentHistoryProofV1,
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<([u8; 32], DocumentHistoryV1), Error> {
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let entries_query = query.entries_query(version)?;
        if version
            .drive
            .methods
            .verify
            .document
            .verify_document_history
            != 1
        {
            return Err(invalid("document history verifier version is unsupported"));
        }
        let (root, metadata) = grovedb::GroveDb::verify_query_with_options(
            &proof.metadata_proof,
            &query.metadata_query(version)?,
            grovedb::VerifyOptions {
                absence_proofs_for_non_existing_searched_keys: false,
                verify_proof_succinctness: true,
                include_empty_trees_in_result: true,
            },
            &version.drive.grove_version,
        )?;
        let (lifecycle, present) = query.lifecycle(metadata)?;
        let entries = match (&proof.entries_proof, present) {
            (None, false) => vec![],
            (Some(bytes), true) => {
                let (entries_root, entries) = grovedb::GroveDb::verify_query(
                    bytes,
                    &entries_query,
                    &version.drive.grove_version,
                )?;
                if root != entries_root {
                    return Err(corrupt("history proofs have different roots"));
                }
                let entries = entries
                    .into_iter()
                    .map(|(path, key, element)| {
                        if path != entries_query.path {
                            return Err(corrupt("history proof returned a different path"));
                        }
                        Ok((
                            key,
                            element.ok_or_else(|| {
                                corrupt("history entry proof contains an absent element")
                            })?,
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                query.decode_entries(entries, document_type, version)?
            }
            _ => {
                return Err(corrupt(
                    "entries proof presence contradicts history metadata",
                ))
            }
        };
        Ok((root, DocumentHistoryV1 { entries, lifecycle }))
    }
}

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;
