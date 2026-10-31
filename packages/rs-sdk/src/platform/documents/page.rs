//! One page of a document listing that continues after the last document of the page before.

use crate::platform::Identifier;
use drive_proof_verifier::types::Documents;

/// The most documents one query returns, the platform's cap.
pub const MAX_DOCUMENTS_PAGE_SIZE: u32 = 100;

/// One page of a listing: at most `limit` documents (the platform's cap of 100 when `None` or
/// 0), after the document `start_after` in the index's order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentsPage {
    /// How many documents at most; `None` and 0 are 100.
    pub limit: Option<u32>,
    /// The id of the last document of the previous page; `None` for the first page.
    pub start_after: Option<Identifier>,
}

impl DocumentsPage {
    /// The page after the one `page` holds, `None` when `page` was the last: it held fewer
    /// documents than the limit.
    pub fn after(&self, page: &Documents) -> Option<Self> {
        let limit = self.page_size();
        if page.len() < limit as usize {
            return None;
        }
        page.keys().last().map(|last| Self {
            limit: self.limit,
            start_after: Some(*last),
        })
    }

    /// The number of documents a page holds when it is full.
    pub fn page_size(&self) -> u32 {
        match self.limit {
            None | Some(0) => MAX_DOCUMENTS_PAGE_SIZE,
            Some(limit) => limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_page_after_the_last_document_of_a_full_page_only() {
        let page = DocumentsPage {
            limit: Some(2),
            start_after: None,
        };
        let full: Documents = [Identifier::from([1; 32]), Identifier::from([2; 32])]
            .into_iter()
            .map(|id| (id, None))
            .collect();
        assert_eq!(
            page.after(&full),
            Some(DocumentsPage {
                limit: Some(2),
                start_after: Some(Identifier::from([2; 32])),
            })
        );
        let short: Documents = [(Identifier::from([1; 32]), None)].into_iter().collect();
        assert_eq!(page.after(&short), None);
        // A limit of 0 is the platform's default of 100, so two documents are a short page
        let default_size = DocumentsPage {
            limit: Some(0),
            start_after: None,
        };
        assert_eq!(default_size.after(&full), None);
    }
}
