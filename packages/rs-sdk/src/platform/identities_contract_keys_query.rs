use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
use dapi_grpc::platform::v0::get_identities_contract_keys_request::Version::V0;
use dapi_grpc::platform::v0::GetIdentitiesContractKeysRequest;
use dpp::identity::Purpose;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportRequest,
};

use crate::platform::Identifier;
use crate::Error;

/// Request that is used to query identities' contract keys
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, dapi_grpc_macros::Mockable)]
pub struct IdentitiesContractKeysQuery {
    /// The identities' identifiers that we want to query
    pub identities_ids: Vec<Identifier>,
    /// The contract identifier
    pub contract_id: Identifier,
    /// An optional document type if the keys are on a document type instead of the contract
    pub document_type_name: Option<String>,
    /// The purposes we want to query for
    pub purposes: Vec<Purpose>,
}

impl IdentitiesContractKeysQuery {
    /// Create new IdentitiesContractKeysQuery for provided identities ids, contract id, the document
    /// type (if we want to make the query on the document type level), and purposes
    pub fn new(
        identities_ids: Vec<Identifier>,
        contract_id: Identifier,
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
    ) -> Result<Self, Error> {
        Ok(Self {
            identities_ids,
            contract_id,
            document_type_name,
            purposes,
        })
    }
}

impl TryFrom<IdentitiesContractKeysQuery> for GetIdentitiesContractKeysRequest {
    type Error = Error;
    fn try_from(dapi_request: IdentitiesContractKeysQuery) -> Result<Self, Self::Error> {
        let IdentitiesContractKeysQuery {
            identities_ids,
            contract_id,
            document_type_name,
            purposes,
        } = dapi_request;
        //todo: transform this into PlatformVersionedTryFrom
        Ok(GetIdentitiesContractKeysRequest {
            version: Some(V0(GetIdentitiesContractKeysRequestV0 {
                identities_ids: identities_ids.into_iter().map(|a| a.to_vec()).collect(),
                contract_id: contract_id.to_vec(),
                document_type_name,
                purposes: purposes.into_iter().map(|purpose| purpose as i32).collect(),
                prove: true,
            })),
        })
    }
}

impl TransportRequest for IdentitiesContractKeysQuery {
    type Client = <GetIdentitiesContractKeysRequest as TransportRequest>::Client;
    type Response = <GetIdentitiesContractKeysRequest as TransportRequest>::Response;
    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings =
        <GetIdentitiesContractKeysRequest as TransportRequest>::SETTINGS_OVERRIDES;

    fn request_name(&self) -> &'static str {
        "getIdentitiesContractKeysRequest"
    }

    fn method_name(&self) -> &'static str {
        "get_identities_contract_keys"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>> {
        let request: GetIdentitiesContractKeysRequest = self
            .try_into()
            .expect("IdentitiesContractKeysQuery should always be valid");
        request.execute_transport(client, settings)
    }
}
