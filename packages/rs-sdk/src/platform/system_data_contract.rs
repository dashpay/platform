//! Resolving a system data contract for the helpers built on one (DPNS, DashPay, moderation
//! charters).

use crate::platform::{DataContract, Fetch, Identifier};
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use dpp::version::ProtocolVersion;
use std::sync::Arc;

impl Sdk {
    /// Fails unless the network runs `initial_protocol_version` or later, the version
    /// `feature` exists from.
    ///
    /// An unpinned SDK starts mainnet and testnet at 13 and only learns a newer version from a
    /// verified response, so below `initial_protocol_version` the SDK first learns the
    /// network's version; a network still below it does not have the feature.
    pub(crate) async fn require_protocol_version(
        &self,
        initial_protocol_version: ProtocolVersion,
        feature: &str,
    ) -> Result<(), Error> {
        if self.protocol_version_number() < initial_protocol_version {
            let version = self.refresh_protocol_version().await?;
            if version < initial_protocol_version {
                return Err(Error::Generic(format!(
                    "{feature} exists from protocol version {initial_protocol_version}; the SDK \
                     runs protocol version {version}"
                )));
            }
        }
        Ok(())
    }

    /// The system data contract `contract_id`, from the context provider when it holds it at
    /// the SDK's protocol version, fetched and proved otherwise. `None` when the network has no
    /// such contract.
    pub async fn fetch_system_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Option<Arc<DataContract>>, Error> {
        if let Some(provider) = self.context_provider() {
            if let Some(contract) = provider.get_data_contract(&contract_id, self.version())? {
                return Ok(Some(contract));
            }
        }
        Ok(DataContract::fetch(self, contract_id).await?.map(Arc::new))
    }
}
