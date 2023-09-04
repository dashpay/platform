//! Identity management

use dapi_grpc::platform::v0::{self as platform_proto};
use drive_proof_verifier::proof::from_proof::FromProof;
use rs_dapi_client::{DapiRequest, RequestSettings};

use crate::{
    crud::{Readable, SdkQuery},
    dapi::DashAPI,
    error::Error,
};

/// Dash Platform Identity object wrapper
pub struct SdkIdentity {
    /// Identity object
    pub inner: dpp::prelude::Identity,
}

impl From<SdkIdentity> for dpp::prelude::Identity {
    fn from(id: SdkIdentity) -> Self {
        id.inner
    }
}

impl From<dpp::prelude::Identity> for SdkIdentity {
    fn from(id: dpp::prelude::Identity) -> Self {
        Self { inner: id }
    }
}

#[async_trait::async_trait]
impl<API: DashAPI> Readable<API> for SdkIdentity {
    type Identifier = [u8; 32];

    async fn read<Q: SdkQuery<Self::Identifier>>(api: &API, id: &Q) -> Result<Self, Error> {
        let request = platform_proto::GetIdentityRequest {
            id: id.query()?.to_vec(),
            prove: true,
        };

        let mut client = api.platform_client().await;
        let response: platform_proto::GetIdentityResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await?;

        let inner =
            dpp::prelude::Identity::from_proof(&request, &response, api.quorum_info_provider()?)?;

        Ok(SdkIdentity { inner })
    }
}
