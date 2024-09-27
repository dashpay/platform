use super::{types::evonode::EvoNode, Query};
use crate::error::Error;
use crate::mock::MockResponse;
use crate::Sdk;
use dapi_grpc::platform::v0::{
    self as platform_proto, get_status_request::GetStatusRequestV0, GetStatusRequest,
    GetStatusResponse, ResponseMetadata,
};
use dpp::{dashcore::Network, version::PlatformVersion};
use drive_proof_verifier::types::EvonodeStatus;
use drive_proof_verifier::unproved::FromUnproved;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
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
    /// - `settings`: Request settings for the connection to Platform.
    ///
    /// ## Returns
    /// Returns:
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
        let request: <Self as FetchUnproved>::Request = query.query(false)?;

        // Execute the request using the Sdk instance
        let response = request.clone().execute(sdk, settings).await?;

        // Parse the response into the appropriate type along with metadata
        let (object, mtd): (Option<Self>, platform_proto::ResponseMetadata) =
            Self::maybe_from_unproved_with_metadata(request, response, sdk.network, sdk.version())?;

        Ok((object, mtd))
    }
}

impl FetchUnproved for drive_proof_verifier::types::CurrentQuorumsInfo {
    type Request = platform_proto::GetCurrentQuorumsInfoRequest;
}

impl FetchUnproved for EvonodeStatus {
    type Request = EvoNode;
}

// We need to delegate FromUnproved for the impl FetchUnproved for EvonodeStatus.
#[async_trait::async_trait]
impl FromUnproved<EvoNode> for EvonodeStatus {
    type Request = EvoNode;
    type Response = GetStatusResponse;

    fn maybe_from_unproved_with_metadata<I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Self>, ResponseMetadata), drive_proof_verifier::Error>
    where
        Self: Sized,
    {
        <Self as FromUnproved<GetStatusRequest>>::maybe_from_unproved_with_metadata(
            GetStatusRequestV0 {},
            response,
            network,
            platform_version,
        )
    }
}
