//! A page of retained revisions with the document's lifecycle.

use super::corrupt;
use crate::error::Error;
use dpp::document::{Document, DocumentV0Getters};
use std::collections::BTreeMap;

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
    /// Metadata for the whole history, including empty pages. Absent when
    /// the page was read from the layout that predates protocol version 14,
    /// which keeps no lifecycle record.
    pub lifecycle: Option<DocumentHistoryLifecycle>,
}

impl DocumentHistoryV1 {
    /// Builds a page from revisions read from the per-document history
    /// subtree used before protocol version 14, keyed by block time.
    pub(crate) fn from_legacy(revisions: BTreeMap<u64, Document>) -> Result<Self, Error> {
        let entries = revisions
            .into_iter()
            .map(|(time_ms, document)| {
                let revision = document
                    .revision()
                    .ok_or_else(|| corrupt("historical document has no revision"))?;
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
