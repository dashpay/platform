use crate::error::Error;
use crate::platform::proto;
use crate::Sdk;
use dapi_grpc::platform::v0::get_current_quorums_info_request::GetCurrentQuorumsInfoRequestV0;
use dapi_grpc::platform::v0::{self as platform_proto};
use drive_proof_verifier::unproved::FromUnproved;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait FetchUnproved
where
    Self: Sized + Debug,
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
    async fn fetch_unproved(sdk: &Sdk) -> Result<Option<Self>, Error> {
        Self::fetch_unproved_with_settings(sdk, RequestSettings::default()).await
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
    async fn fetch_unproved_with_settings(
        sdk: &Sdk,
        settings: RequestSettings,
    ) -> Result<Option<Self>, Error>;
}

#[async_trait::async_trait]
impl FetchUnproved for drive_proof_verifier::types::CurrentQuorumsInfo {
    type Request = platform_proto::GetCurrentQuorumsInfoRequest;

    async fn fetch_unproved_with_settings(
        sdk: &Sdk,
        settings: RequestSettings,
    ) -> Result<Option<Self>, Error> {
        // Create the request from the query
        let request = proto::GetCurrentQuorumsInfoRequest {
            version: Some(proto::get_current_quorums_info_request::Version::V0(
                GetCurrentQuorumsInfoRequestV0 {},
            )),
        };

        // Execute the request using the Sdk instance
        let response = request.clone().execute(sdk, settings).await?;

        // Parse the response into a CurrentQuorumsInfo object along with metadata
        match Self::maybe_from_unproved_with_metadata(request, response, sdk.network, sdk.version())
        {
            Ok((Some(info), _metadata)) => Ok(Some(info)),
            Ok((None, _metadata)) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
