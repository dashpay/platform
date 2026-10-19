use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
use dapi_grpc::platform::v0::get_identities_contract_keys_request::Version::V0;
use dapi_grpc::platform::v0::GetIdentitiesContractKeysRequest;
use dpp::identity::Purpose;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};
use std::collections::BTreeSet;

use crate::platform::query::Query;
use crate::platform::Identifier;
use crate::Error;

/// Request that is used to query identities' contract keys.
///
/// A repeated identity or purpose is sent once, in the order it first appears: Drive sizes the
/// proof by the number of identities times the number of purposes, so every caller of this query
/// sends, and has the proof verified against, the same request whatever repeats it was given. A
/// `GetIdentitiesContractKeysRequest` built by hand goes out as it is; Drive and the proof check
/// then both read it as sent.
#[derive(Debug, Clone, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
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

    /// The request this query sends, proved or not.
    fn request(&self, prove: bool) -> GetIdentitiesContractKeysRequest {
        GetIdentitiesContractKeysRequest {
            version: Some(V0(GetIdentitiesContractKeysRequestV0 {
                identities_ids: first_of_each(&self.identities_ids)
                    .into_iter()
                    .map(|id| id.to_vec())
                    .collect(),
                contract_id: self.contract_id.to_vec(),
                document_type_name: self.document_type_name.clone(),
                purposes: first_of_each(&self.purposes)
                    .into_iter()
                    .map(|purpose| purpose as i32)
                    .collect(),
                prove,
            })),
        }
    }
}

/// `items` in the order they first appear, each once.
fn first_of_each<T: Ord + Copy>(items: &[T]) -> Vec<T> {
    let mut seen = BTreeSet::new();
    items
        .iter()
        .copied()
        .filter(|item| seen.insert(*item))
        .collect()
}

impl TryFrom<IdentitiesContractKeysQuery> for GetIdentitiesContractKeysRequest {
    type Error = Error;
    fn try_from(dapi_request: IdentitiesContractKeysQuery) -> Result<Self, Self::Error> {
        //todo: transform this into PlatformVersionedTryFrom
        Ok(dapi_request.request(true))
    }
}

impl Query<GetIdentitiesContractKeysRequest> for IdentitiesContractKeysQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentitiesContractKeysRequest, Error> {
        Ok(self.request(settings.prove))
    }
}

impl crate::platform::query::WireQuery for IdentitiesContractKeysQuery {}

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

    fn names_nothing(&self) -> Option<&'static str> {
        self.request(true).names_nothing()
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        self.request(true).execute_transport(client, settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::QuerySettings;
    use dpp::version::PlatformVersion;
    use rs_dapi_client::RequestSettings;

    #[test]
    fn should_send_each_identity_and_purpose_once_in_first_order() {
        let a = Identifier::from([1; 32]);
        let b = Identifier::from([2; 32]);
        let query = IdentitiesContractKeysQuery::new(
            vec![b, a, b, a],
            Identifier::from([3; 32]),
            None,
            vec![
                Purpose::DECRYPTION,
                Purpose::ENCRYPTION,
                Purpose::DECRYPTION,
            ],
        )
        .expect("valid query");
        let expected = GetIdentitiesContractKeysRequest {
            version: Some(V0(GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![b.to_vec(), a.to_vec()],
                contract_id: vec![3; 32],
                document_type_name: None,
                purposes: vec![Purpose::DECRYPTION as i32, Purpose::ENCRYPTION as i32],
                prove: true,
            })),
        };

        // The transport and names_nothing build it through `request`; the request sent and the
        // one the proof is verified against are the same.
        assert_eq!(query.request(true), expected);
        assert_eq!(
            GetIdentitiesContractKeysRequest::try_from(query.clone()).expect("request"),
            expected
        );
        let request_settings = RequestSettings::default();
        let settings = QuerySettings {
            request_settings: &request_settings,
            protocol_version: PlatformVersion::latest(),
            prove: true,
        };
        let sent: GetIdentitiesContractKeysRequest = query.query(&settings).expect("request");
        assert_eq!(sent, expected);
    }

    #[test]
    fn should_name_nothing_like_the_request_it_sends() {
        let query = |identities_ids: Vec<Identifier>, purposes: Vec<Purpose>| {
            IdentitiesContractKeysQuery::new(
                identities_ids,
                Identifier::from([2; 32]),
                None,
                purposes,
            )
            .expect("valid query")
        };

        assert_eq!(
            query(vec![], vec![Purpose::AUTHENTICATION]).names_nothing(),
            Some("identities_ids must contain at least one identifier when requesting a proof")
        );
        assert_eq!(
            query(vec![Identifier::from([1; 32])], vec![]).names_nothing(),
            Some("purposes must contain at least one purpose when requesting a proof")
        );
        assert_eq!(
            query(
                vec![Identifier::from([1; 32])],
                vec![Purpose::AUTHENTICATION]
            )
            .names_nothing(),
            None
        );
    }
}
