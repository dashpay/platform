//! `subscribeToStateTransitions`: stream committed state transitions matching a client's
//! filters.
//!
//! Each subscription owns a cursor (the next height to scan) and walks committed blocks in
//! height order through the shared [`BlockSource`], whether they are history or new: one loop
//! serves both, so there is no live/historical hand-over to get wrong. New blocks only wake
//! the loop up. A block matching nothing is skipped silently; a checkpoint at the highest
//! fully scanned height follows every block with a match and is repeated every
//! [`CHECKPOINT_INTERVAL`] otherwise, so a client always knows where to resume and quiet
//! streams stay open through proxies.
//!
//! The scan never moves past a height it could not read completely. A transaction that does
//! not decode, a block whose protocol version this build does not know, or results that do not
//! line up with the block's transactions end the stream instead: the platform executed that
//! transaction, so skipping it would silently lose a match.

mod block_source;
mod stream;

pub use block_source::{BlockSource, TenderdashBlocks};
pub use stream::{SubscriptionLimits, SubscriptionService, SubscriptionStream};

use crate::services::platform_service::PlatformServiceImpl;
use crate::services::platform_service::shielded_proof_failure_budget::{
    is_internal, last_forwarded_address,
};
use dapi_grpc::platform::v0::SubscribeToStateTransitionsRequest;
use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::Version;
use dapi_grpc::platform::v0::{
    GetDataContractRequest, get_data_contract_request, get_data_contract_response,
};
use dapi_grpc::tonic::{Request, Status};
use dash_platform_queries::subscriptions::{
    MAX_FILTERS, ResolvedFilters, StateTransitionFilter, SubscriptionFilterError,
};
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;
use std::net::IpAddr;
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
        let client_ip = client_ip(&request);
        let Some(Version::V0(request)) = request.into_inner().version else {
            return Err(Status::invalid_argument("missing request version"));
        };
        if request.from_block_height == Some(0) {
            return Err(Status::invalid_argument(
                "from_block_height starts at 1; leave it unset to start after the current tip",
            ));
        }
        // Bound the request before any work is done for it, Drive lookups included.
        if request.filters.is_empty() || request.filters.len() > MAX_FILTERS {
            return Err(Status::invalid_argument(format!(
                "a subscription takes 1 to {MAX_FILTERS} filters, got {}",
                request.filters.len()
            )));
        }
        let filters = request
            .filters
            .into_iter()
            .enumerate()
            .map(|(index, filter)| StateTransitionFilter::from_proto(index, filter))
            .collect::<Result<Vec<_>, _>>()
            .map_err(filter_error_status)?;

        // Admit before doing any work on the client's behalf.
        let admission = self.subscriptions.admit(client_ip)?;

        let mut contracts = BTreeMap::new();
        // The lowest height the contracts were read at: their bindings cover every update up
        // to it, and the scan follows those after it.
        let mut bound_at: Option<u64> = None;
        for data_contract_id in ResolvedFilters::data_contract_ids(&filters) {
            let (contract, read_at) = self.fetch_data_contract(data_contract_id).await?;
            if let Some(contract) = contract {
                contracts.insert(data_contract_id, Arc::new(contract));
            }
            if let Some(read_at) = read_at {
                bound_at = Some(bound_at.map_or(read_at, |bound_at| bound_at.min(read_at)));
            }
        }
        let filters = ResolvedFilters::resolve(
            filters,
            |id| contracts.get(id).cloned(),
            PlatformVersion::latest(),
        )
        .map_err(filter_error_status)?;

        self.subscriptions
            .start(admission, filters, request.from_block_height, bound_at)
            .await
    }

    /// The current version of a data contract, from Drive, and the height Drive read it at.
    /// Unproved: the client re-checks every delivered transition against filters it resolves
    /// from proved contracts.
    async fn fetch_data_contract(
        &self,
        data_contract_id: Identifier,
    ) -> Result<(Option<DataContract>, Option<u64>), Status> {
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
        let read_at = response.metadata.as_ref().map(|metadata| metadata.height);
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
        Ok((contract, read_at))
    }
}

fn filter_error_status(error: SubscriptionFilterError) -> Status {
    match error {
        SubscriptionFilterError::DataContractNotFound(_) => Status::not_found(error.to_string()),
        _ => Status::invalid_argument(error.to_string()),
    }
}

/// The client a request comes from: the connection's peer, or, when that peer is the gateway
/// (a loopback or private address), the last `X-Forwarded-For` entry, which Envoy appends from
/// the connection it accepted and a client cannot forge. A client connecting directly is its
/// own peer, whatever header it sends.
fn client_ip<T>(request: &Request<T>) -> Option<IpAddr> {
    let peer = request.remote_addr().map(|address| address.ip());
    let behind_gateway = peer.is_none_or(is_internal);
    let forwarded = behind_gateway
        .then(|| {
            request
                .metadata()
                .get_all("x-forwarded-for")
                .iter()
                .next_back()
                .and_then(|value| value.to_str().ok())
                .and_then(last_forwarded_address)
        })
        .flatten();
    forwarded.or(peer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn request_from(peer: &str, forwarded_for: Option<&str>) -> Request<()> {
        let mut request = Request::new(());
        request
            .extensions_mut()
            .insert(tonic::transport::server::TcpConnectInfo {
                local_addr: None,
                remote_addr: Some(peer.parse::<SocketAddr>().unwrap()),
            });
        if let Some(value) = forwarded_for {
            request
                .metadata_mut()
                .insert("x-forwarded-for", value.parse().unwrap());
        }
        request
    }

    #[test]
    fn should_read_the_client_address_the_gateway_forwards() {
        let request = request_from("10.0.0.2:41000", Some("10.0.0.1, 203.0.113.7"));
        assert_eq!(client_ip(&request), Some("203.0.113.7".parse().unwrap()));
    }

    #[test]
    fn should_use_the_peer_of_a_direct_connection_with_or_without_a_header() {
        let peer: IpAddr = "198.51.100.1".parse().unwrap();
        assert_eq!(
            client_ip(&request_from("198.51.100.1:41000", None)),
            Some(peer)
        );
        // A forged header from a client connecting directly is ignored.
        assert_eq!(
            client_ip(&request_from("198.51.100.1:41000", Some("203.0.113.7"))),
            Some(peer)
        );
    }
}
