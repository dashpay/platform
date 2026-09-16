//! The history query and its validation and path queries.

use super::{
    corrupt, invalid, DocumentHistoryEntry, DocumentHistoryFilter, DocumentHistoryLifecycle,
    DocumentHistoryState,
};
use crate::drive::document::paths::{
    contract_document_type_path_vec, document_history_path, DOCUMENT_HISTORY_TREE_KEY,
};
use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::common::encode::encode_u64;
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, Query, SizedQuery};

/// Query for composite-keyed document history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHistoryQueryV1 {
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

impl DocumentHistoryQueryV1 {
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

    /// The leaf-only query required for authenticated count-offset pagination.
    pub fn entries_query(&self, version: &PlatformVersion) -> Result<PathQuery, Error> {
        match version
            .drive
            .methods
            .document
            .query
            .fetch_document_history_query
        {
            1 => self.entries_query_v1(),
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "entries_query".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    pub(crate) fn entries_query_v1(&self) -> Result<PathQuery, Error> {
        self.validate()?;
        let mut query = Query::new();
        let mut limit = self.limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT);
        let offset = match self.filter {
            DocumentHistoryFilter::StartAtTime(time) => {
                query.insert_range_from(encode_u64(time)..);
                None
            }
            DocumentHistoryFilter::StartAfter { time_ms, revision } => {
                let mut key = encode_u64(time_ms);
                key.extend(encode_u64(revision));
                query.insert_range_after(key..);
                None
            }
            DocumentHistoryFilter::StartAtRevision(revision)
            | DocumentHistoryFilter::Revision(revision) => {
                query.insert_all();
                if matches!(self.filter, DocumentHistoryFilter::Revision(_)) {
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
        let mut merged = PathQuery::merge(queries.iter().collect(), &version.drive.grove_version)?;
        // One exact key per branch. The finite limit lets the verifier map
        // every queried key to a proven element or a proven absence.
        merged.query.limit = Some(queries.len() as u16);
        Ok(merged)
    }

    pub(super) fn lifecycle(
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
        let mut active = false;
        let mut latest_revision = None;
        let mut count = None;
        for (path, key, element) in metadata {
            let Some(element) = element else {
                continue;
            };
            if key != self.document_id {
                return Err(corrupt("history metadata key does not match the document"));
            }
            if path == primary {
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
        if active && count.unwrap_or_default() == 0 || !active && count.unwrap_or_default() > 0 {
            return Err(corrupt("current document and retained history disagree"));
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

    pub(super) fn decode_entries(
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
}
