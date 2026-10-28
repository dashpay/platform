//! Resolving a system data contract for the helpers built on one (DPNS, DashPay, moderation
//! charters).

use crate::platform::{DataContract, Fetch, Identifier};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::ResponseMetadata;
use dash_context_provider::ContextProvider;
use std::future::Future;
use std::sync::Arc;

/// How many responses [`Sdk::fetch_current_system_data_contract`] reads for one proved at the
/// protocol version the SDK parsed it under.
pub(crate) const SYSTEM_CONTRACT_FETCH_ATTEMPTS: usize = 3;

/// The `name` contract of the first response proved at the protocol version the SDK parsed it
/// under, reading the SDK's version through `sdk_version` and each response through `fetch`,
/// at most [`SYSTEM_CONTRACT_FETCH_ATTEMPTS`] times. The SDK's version only rises, and SDK
/// clones share it, so a version unchanged across the fetch is the one the response was parsed
/// under; one that rose meanwhile (this response, or another request, taught the SDK a newer
/// version) leaves the response unused. A response proved at a newer version was parsed under
/// an older one, which can drop declarations the newer contract makes. One proved at an older
/// version comes from a node behind the network, which can still hold the contract an upgrade
/// replaced.
pub(crate) async fn contract_proved_at_parsed_version<V, F, R>(
    name: &str,
    sdk_version: V,
    mut fetch: F,
) -> Result<DataContract, Error>
where
    V: Fn() -> u32,
    F: FnMut() -> R,
    R: Future<Output = Result<(Option<DataContract>, ResponseMetadata), Error>>,
{
    let mut last_mismatch = None;
    for _ in 0..SYSTEM_CONTRACT_FETCH_ATTEMPTS {
        let parsed_under = sdk_version();
        let (contract, metadata) = fetch().await?;
        if metadata.protocol_version == parsed_under && sdk_version() == parsed_under {
            return contract.ok_or_else(|| Error::Generic(format!("{name} contract not found")));
        }
        last_mismatch = Some((metadata.protocol_version, parsed_under));
    }
    let (proved_at, parsed_under) = last_mismatch.unwrap_or_default();
    Err(Error::Generic(format!(
        "no {name} contract response was proved at the protocol version the SDK parsed it \
         under (last proved at {proved_at}, parsed under {parsed_under})"
    )))
}

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

    /// The system data contract `contract_id` (named `name` in errors) the network stores now,
    /// fetched and proved, for a transition whose rules come from it. Neither a contract the
    /// context provider holds for the SDK's protocol version nor one cached before an upgrade
    /// will do: either can be older than the network's. The SDK first learns the network's
    /// protocol version (a proven refresh), then takes a response proved at the version it
    /// parsed it under ([`contract_proved_at_parsed_version`]).
    pub(crate) async fn fetch_current_system_data_contract(
        &self,
        contract_id: Identifier,
        name: &str,
    ) -> Result<DataContract, Error> {
        self.refresh_protocol_version().await?;
        contract_proved_at_parsed_version(
            name,
            || self.protocol_version_number(),
            || DataContract::fetch_with_metadata(self, contract_id, None),
        )
        .await
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
