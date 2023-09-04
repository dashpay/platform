//! Data contract features

use crate::{
    crud::{Readable, SdkQuery},
    dapi::DashAPI,
    error::Error,
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
impl<API: DashAPI> Readable<API> for SdkDataContract {
    type Identifier = [u8; 32];

    async fn read<Q: SdkQuery<Self::Identifier>>(api: &API, id: &Q) -> Result<Self, Error> {
        let query = id.query()?;
        let request = platform_proto::GetDataContractRequest {
            id: query.to_vec(),
            prove: true,
        };

        let mut client = api.platform_client().await;
        let mut response: platform_proto::GetDataContractResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await?;

        if let Some(mtd) = &mut response.metadata {
            mtd.chain_id = "dashmate_local_32".to_string();
        }

        let inner = dpp::prelude::DataContract::from_proof(
            &request,
            &response,
            api.quorum_info_provider()?,
        )?;
        Ok(SdkDataContract { inner })
    }
}
