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
    /// such contract. A fetched contract is registered with the context provider, so the proof
    /// confirming a transition built against it can be verified.
    ///
    /// Limitation: registration goes to the provider installed when the fetch returns, not
    /// necessarily the one that verified the proof. A provider swapped in with
    /// [`Sdk::set_context_provider`] while the fetch is in flight receives a contract it did
    /// not authenticate itself; avoid swapping providers while a fetch is pending.
    pub async fn fetch_system_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Option<Arc<DataContract>>, Error> {
        if let Some(provider) = self.context_provider() {
            if let Some(contract) = provider.get_data_contract(&contract_id, self.version())? {
                return Ok(Some(contract));
            }
        }
        let contract = DataContract::fetch(self, contract_id).await?.map(Arc::new);
        if let (Some(provider), Some(contract)) = (self.context_provider(), &contract) {
            provider.register_data_contract(Arc::clone(contract));
        }
        Ok(contract)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::platform::DataContract;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::identity::SecurityLevel;
    use dpp::platform_value::Value;
    use dpp::version::{PlatformVersion, TryFromPlatformVersioned};

    /// `contract` with its `document_type_name` type requiring a `level` key to sign: a contract
    /// an upgrade could store in place of the one the SDK bundles.
    pub(crate) fn with_document_security_level(
        contract: DataContract,
        document_type_name: &str,
        level: SecurityLevel,
    ) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let mut format = DataContractInSerializationFormat::try_from_platform_versioned(
            contract,
            platform_version,
        )
        .expect("the serialization format")
        .into_schema_form();
        format
            .document_schemas_mut()
            .expect("the schema form")
            .get_mut(document_type_name)
            .expect("the document type's schema")
            .set_value("signatureSecurityLevelRequirement", Value::U8(level as u8))
            .expect("the schema is a map");
        DataContract::try_from_platform_versioned(format, true, &mut vec![], platform_version)
            .expect("a valid contract")
    }
}
