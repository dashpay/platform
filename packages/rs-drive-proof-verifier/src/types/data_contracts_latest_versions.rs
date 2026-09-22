use crate::types::RetrievedObjects;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;

/// The current version of one data contract, one entry of `getDataContractsLatestVersions`.
///
/// `data_contract` is `Some` only when the request set `include_contracts`. The query exists
/// so that a client can check the contracts it already holds against `version` without
/// transferring them again: from protocol version 14 a request without the contracts is
/// answered from, and proved by, the four-byte version item each contract carries in state.
#[derive(Debug, Clone, PartialEq)]
pub struct DataContractLatestVersion {
    /// The contract's current version number.
    pub version: u32,
    /// The contract itself, only when the request asked for it.
    pub data_contract: Option<DataContract>,
}

/// The current versions of the requested data contracts (`getDataContractsLatestVersions`),
/// keyed by contract id.
///
/// Every distinct requested id is a key. A value is `None` when no contract has that id, and
/// `Some(entry)` otherwise; the entry carries the contract only when the request asked for
/// it.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DataContractsLatestVersions(pub RetrievedObjects<Identifier, DataContractLatestVersion>);

impl DataContractsLatestVersions {
    /// The current version of the contract with `id`: `None` when it was not requested or
    /// does not exist.
    pub fn version_of(&self, id: &Identifier) -> Option<u32> {
        self.0.get(id)?.as_ref().map(|entry| entry.version)
    }
}

impl FromIterator<(Identifier, Option<DataContractLatestVersion>)> for DataContractsLatestVersions {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<DataContractLatestVersion>)>>(
        iter: T,
    ) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for DataContractsLatestVersions {
    type Item = (Identifier, Option<DataContractLatestVersion>);
    type IntoIter =
        <RetrievedObjects<Identifier, DataContractLatestVersion> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
