//! Identity management

use dapi_grpc::platform::v0::{self as platform_proto};
use dpp::prelude::Identifier;
use drive_proof_verifier::proof::from_proof::FromProof;
use rs_dapi_client::{DapiRequest, RequestSettings};

use crate::{crud::Readable, dapi::DAPI, error::Error};

/// Dash Platform Identity object wrapper
pub struct Identity {
    /// Identity object
    pub inner: dpp::prelude::Identity,
}

impl From<Identity> for dpp::prelude::Identity {
    fn from(id: Identity) -> Self {
        id.inner
    }
}

impl From<dpp::prelude::Identity> for Identity {
    fn from(id: dpp::prelude::Identity) -> Self {
        Self { inner: id }
    }
}

#[async_trait::async_trait]
impl<A: DAPI> Readable<A, [u8; 32], Identifier> for Identity {
    async fn read(api: &A, id: &Identifier) -> Result<Self, Error> {
        let request = platform_proto::GetIdentityRequest {
            id: id.to_vec(),
            prove: true,
        };

        let mut client = api.platform_client().await;
        let response: platform_proto::GetIdentityResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await?;

        let inner =
            dpp::prelude::Identity::from_proof(&request, &response, api.quorum_info_provider()?)?;

        Ok(Identity { inner })
    }
}
