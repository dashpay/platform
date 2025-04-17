use super::{types::evonode::EvoNode, Query};
use crate::mock::MockResponse;
use crate::Sdk;
use crate::{error::Error, sync::retry};
use dapi_grpc::platform::v0::{
    self as platform_proto, GetStatusRequest, GetStatusResponse, ResponseMetadata,
};
use dpp::{dashcore::Network, version::PlatformVersion};
use drive_proof_verifier::types::evonode_status::EvoNodeStatus;
use drive_proof_verifier::unproved::FromUnproved;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use rs_dapi_client::{ExecutionError, ExecutionResponse, InnerInto, IntoInner};
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait FetchUnproved
where
    Self: Sized + Debug + MockResponse,
{
    /// Type of request used to fetch data from Platform.
    type Request: TransportRequest;

    /// Fetch unproved data from the Platform.
    ///
    /// ## Parameters
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: Query used to fetch data from the Platform.
    ///
    /// ## Returns
    /// Returns:
    /// * `Ok(Some(Self))` when object is found.
    /// * `Ok(None)` when object is not found.
    /// * [`Err(Error)`](Error) when an error occurs.
    async fn fetch_unproved<Q: Query<<Self as FetchUnproved>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<Option<Self>, Error>
    where
        Self: FromUnproved<
            <Self as FetchUnproved>::Request,
            Request = <Self as FetchUnproved>::Request,
            Response = <<Self as FetchUnproved>::Request as TransportRequest>::Response,
        >,
    {
        let (obj, _mtd) =
            Self::fetch_unproved_with_settings(sdk, query, RequestSettings::default()).await?;
        Ok(obj)
    }

    /// Fetch unproved data from the Platform with custom settings.
    ///
    /// ## Parameters
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: Query used to fetch data from the Platform.
    /// - `settings`: Request settings for the connection to Platform.
    ///
    /// ## Returns
    /// * `Ok(Some(Self))` when object is found.
    /// * `Ok(None)` when object is not found.
    /// * [`Err(Error)`](Error) when an error occurs.
    async fn fetch_unproved_with_settings<Q: Query<<Self as FetchUnproved>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: RequestSettings,
    ) -> Result<(Option<Self>, ResponseMetadata), Error>
    where
        Self: FromUnproved<
            <Self as FetchUnproved>::Request,
            Request = <Self as FetchUnproved>::Request,
            Response = <<Self as FetchUnproved>::Request as TransportRequest>::Response,
        >,
    {
        // Default implementation
        let request: &<Self as FetchUnproved>::Request = &query.query(false)?;
        let closure = move |local_settings: RequestSettings| async move {
            // Execute the request using the Sdk instance
            let ExecutionResponse {
                inner: response,
                address,
                retries,
            } = request
                .clone()
                .execute(sdk, local_settings)
                .await
                .map_err(|e| e.inner_into())?;

            // Parse the response into the appropriate type along with metadata
            let (object, metadata): (Option<Self>, platform_proto::ResponseMetadata) =
                Self::maybe_from_unproved_with_metadata(
                    request.clone(),
                    response,
                    sdk.network,
                    sdk.version(),
                )
                .map_err(|e| ExecutionError {
                    inner: e.into(),
                    address: Some(address.clone()),
                    retries,
                })?;

            Ok(ExecutionResponse {
                inner: (object, metadata),
                address,
                retries,
            })
        };

        let settings = sdk.dapi_client_settings.override_by(settings);
        retry(sdk.address_list(), settings, closure)
            .await
            .into_inner()
    }
}

impl FetchUnproved for drive_proof_verifier::types::CurrentQuorumsInfo {
    type Request = platform_proto::GetCurrentQuorumsInfoRequest;
}

impl FetchUnproved for EvoNodeStatus {
    type Request = EvoNode;
}

// We need to delegate FromUnproved for the impl FetchUnproved for EvonodeStatus.
#[async_trait::async_trait]
impl FromUnproved<EvoNode> for EvoNodeStatus {
    type Request = EvoNode;
    type Response = GetStatusResponse;

    fn maybe_from_unproved_with_metadata<I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Self>, ResponseMetadata), drive_proof_verifier::Error>
    where
        Self: Sized,
    {
        // delegate to the FromUnproved<GetStatusResponse>
        <Self as FromUnproved<GetStatusRequest>>::maybe_from_unproved_with_metadata(
            request.into(),
            response,
            network,
            platform_version,
        )
    }
}
