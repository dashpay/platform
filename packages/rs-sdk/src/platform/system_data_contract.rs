//! Resolving a system data contract for the helpers built on one (DPNS, DashPay, moderation
//! charters).

use crate::platform::{DataContract, Fetch, Identifier};
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use std::sync::Arc;

impl Sdk {
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
