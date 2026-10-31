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
