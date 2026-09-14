use crate::types::RetrievedObjects;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;

/// One page of the contract enumeration (`getDataContractsByRange`), in ascending contract id
/// order.
///
/// Keys are contract ids. A value is `Some(contract)` on a full page and `None` on an
/// ids-only page (the request had `ids_only` set). `None` never means the contract is
/// absent: a range proof only carries contracts that exist.
#[derive(Debug, Default, Clone)]
pub struct DataContractsByRange(pub RetrievedObjects<Identifier, DataContract>);

impl DataContractsByRange {
    /// The cursor for the next page: the last contract id of this page, to be sent as
    /// `start_after`. `None` when the page is empty, which means there is no next page.
    pub fn next_start_after(&self) -> Option<Identifier> {
        self.0.keys().last().copied()
    }

    /// Whether no contract follows this page for the requested `limit`: a page shorter
    /// than the limit is the last page.
    pub fn is_last_page(&self, limit: usize) -> bool {
        self.0.len() < limit
    }
}

impl FromIterator<(Identifier, Option<DataContract>)> for DataContractsByRange {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<DataContract>)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
