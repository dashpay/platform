//! Data contract features

use crate::{
    crud::{Readable, SdkQuery},
    error::Error,
    sdk::Sdk,
};
use dapi_grpc::platform::v0::{self as platform_proto};
use drive_proof_verifier::proof::from_proof::FromProof;
use rs_dapi_client::{DapiRequest, RequestSettings};

/// Data contract object wrapper
pub struct SdkDataContract {
    /// Data contract object
    pub inner: dpp::prelude::DataContract,
}

impl From<SdkDataContract> for dpp::prelude::DataContract {
    fn from(dc: SdkDataContract) -> Self {
        dc.inner
    }
}

impl From<dpp::prelude::DataContract> for SdkDataContract {
    fn from(dc: dpp::prelude::DataContract) -> Self {
        Self { inner: dc }
    }
}

#[async_trait::async_trait]
impl<API: Sdk> Readable<API> for SdkDataContract {
    type Identifier = [u8; 32];

    async fn read<Q: SdkQuery<Self::Identifier>>(api: &API, id: &Q) -> Result<Self, Error> {
        let query = id.query()?;
        let request = platform_proto::GetDataContractRequest {
            id: query.to_vec(),
            prove: true,
        };

        let mut client = api.platform_client().await;
        let response: platform_proto::GetDataContractResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await?;

        tracing::trace!(request = ?request, response = ?response, "read data contract");

        let contract = dpp::prelude::DataContract::maybe_from_proof(
            &request,
            &response,
            api.quorum_info_provider()?,
        )?;

        match contract {
            Some(contract) => Ok(contract.into()),
            None => Err(Error::NotFound(format!(
                "data contract not found: {:?}",
                query
            ))),
        }
    }
}
