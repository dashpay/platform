//! `subscribeToStateTransitions`: stream committed state transitions matching a client's
//! filters; see `packages/dapi/doc/endpoints/streams/subscribeToStateTransitions.md`.

mod block_source;
mod stream;

pub use block_source::{BlockSource, TenderdashBlocks};
pub use stream::{SubscriptionLimits, SubscriptionService, SubscriptionStream};

use crate::services::platform_service::PlatformServiceImpl;
use crate::services::platform_service::shielded_proof_failure_budget::SourceKey;
use dapi_grpc::platform::v0::SubscribeToStateTransitionsRequest;
use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::Version;
use dapi_grpc::platform::v0::{
    GetDataContractRequest, get_data_contract_request, get_data_contract_response,
};
use dapi_grpc::tonic::{Request, Status};
use dash_platform_queries::subscriptions::{
    ResolvedFilters, StateTransitionFilter, SubscriptionFilterError, check_filter_count,
};
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

/// How often a checkpoint is sent while no block matches.
pub const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(10);

impl PlatformServiceImpl {
    /// Validate a subscription request and start its stream.
    pub async fn subscribe_to_state_transitions_impl(
        &self,
        request: Request<SubscribeToStateTransitionsRequest>,
    ) -> Result<stream::SubscriptionStream, Status> {
        let source = SourceKey::of_request(&request);
        let Some(Version::V0(request)) = request.into_inner().version else {
            return Err(Status::invalid_argument("missing request version"));
        };
        if request.from_block_height == Some(0) {
            return Err(Status::invalid_argument(
                "from_block_height starts at 1; leave it unset to start after the current tip",
            ));
        }
        // Bound the filter count before any work is done, Drive lookups included. Per-filter
        // limits are checked by `resolve`, after the Drive fetches.
        check_filter_count(request.filters.len())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let filters = request
            .filters
            .into_iter()
            .enumerate()
            .map(|(index, filter)| StateTransitionFilter::from_proto(index, filter))
            .collect::<Result<Vec<_>, _>>()
            .map_err(filter_error_status)?;

        // Admit before doing any work on the client's behalf.
        let admission = self.subscriptions.admit(source)?;

        let mut contracts = BTreeMap::new();
        // Resolve under the version Drive decoded the contracts with, the lowest if they came
        // from different blocks.
        let mut platform_version: Option<&'static PlatformVersion> = None;
        for data_contract_id in ResolvedFilters::data_contract_ids(&filters) {
            let (contract, version) = self.fetch_data_contract(data_contract_id).await?;
            if platform_version
                .is_none_or(|lowest| version.protocol_version < lowest.protocol_version)
            {
                platform_version = Some(version);
            }
            if let Some(contract) = contract {
                contracts.insert(data_contract_id, Arc::new(contract));
            }
        }
        // Without document filters no contract is read, and address filters are not versioned.
        let platform_version = platform_version.unwrap_or_else(PlatformVersion::latest);
        let filters =
            ResolvedFilters::resolve(filters, |id| contracts.get(id).cloned(), platform_version)
                .map_err(filter_error_status)?;

        self.subscriptions
            .start(admission, filters, request.from_block_height)
            .await
    }

    /// The current version of a data contract, from Drive (unproved), and the platform version
    /// Drive reported, which it is decoded with.
    async fn fetch_data_contract(
        &self,
        data_contract_id: Identifier,
    ) -> Result<(Option<DataContract>, &'static PlatformVersion), Status> {
        let request = GetDataContractRequest {
            version: Some(get_data_contract_request::Version::V0(
                get_data_contract_request::GetDataContractRequestV0 {
                    id: data_contract_id.to_vec(),
                    prove: false,
                },
            )),
        };
        let response = self
            .drive_client
            .get_client()
            .get_data_contract(request)
            .await?
            .into_inner();
        let Some(get_data_contract_response::Version::V0(response)) = response.version else {
            return Err(Status::internal(
                "Drive returned an unversioned data contract",
            ));
        };
        let protocol_version = response
            .metadata
            .as_ref()
            .map(|metadata| metadata.protocol_version)
            .unwrap_or(PlatformVersion::latest().protocol_version);
        let platform_version = PlatformVersion::get(protocol_version)
            .map_err(|e| Status::failed_precondition(e.to_string()))?;
        let contract = match response.result {
            Some(
                get_data_contract_response::get_data_contract_response_v0::Result::DataContract(
                    bytes,
                ),
            ) if !bytes.is_empty() => Some(
                DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
                    .map_err(|e| Status::internal(format!("cannot decode data contract: {e}")))?,
            ),
            _ => None,
        };
        Ok((contract, platform_version))
    }
}

fn filter_error_status(error: SubscriptionFilterError) -> Status {
    match error {
        SubscriptionFilterError::DataContractNotFound(_) => Status::not_found(error.to_string()),
        _ => Status::invalid_argument(error.to_string()),
    }
}
