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
/// # What the answer holds
///
/// [`IdentitiesContractKeys`] is read from a verified proof. For each identity and purpose it
/// holds the key Drive's current-key reference names for the contract (or document type) and the
/// purpose. What that reference holds depends on the storage requirement for the purpose of the
/// contract, or of the document type for a key bound to one (as Drive writes it from protocol
/// version 14):
/// - `MultipleReferenceToLatest` (every bound authentication key, a bound encryption or
///   decryption key when the contract, or for a key bound to a document type the document type,
///   asks for it, and every key bound to a contract group, whose ID can be passed as
///   `contract_id`): the key of the latest registration, and of the keys one identity create or
///   update registers together, the one with the highest ID. It stays the current one when it is
///   disabled (its `disabled_at` is then set).
/// - `Unique`: the identity's one key bound to that contract or document type for the purpose;
///   Platform refuses a second one later, also once the first is disabled. (If one identity create
///   or update registers several, the reference names the last one Drive queued.)
/// - `Multiple`: there is no reference, so there is never a key, whatever keys exist.
///
/// Only authentication, encryption and decryption keys can be bound to a contract, so any other
/// purpose (transfer, system, voting, owner) never has a key. Nor does a contract or document type
/// that does not exist.
///
/// # The request sent
///
/// A repeated identity or purpose is sent once, in the order it first appears: Drive sizes the
/// proof by the number of identities times the number of purposes, so every caller of this query
/// sends, and has the proof verified against, the same request whatever repeats it was given. A
/// `GetIdentitiesContractKeysRequest` built by hand goes out as it is; Drive and the proof check
/// then both read it as sent.
///
/// A query for no identities asks about nothing: [`Fetch::fetch`] and
/// [`Fetch::fetch_with_settings`] answer it with an empty map, whatever its purposes, and send
/// nothing. [`Fetch::fetch_with_metadata`] and [`Fetch::fetch_with_metadata_and_proof`] have no
/// response to take metadata and a proof from, so they send it and are refused, as every node
/// refuses a proof of nothing (see [`TransportRequest::names_nothing`]). A query for identities
/// but no purpose is refused the same way by every fetch.
///
/// [`IdentitiesContractKeys`]: dpp::identity::identities_contract_keys::IdentitiesContractKeys
/// [`Fetch::fetch`]: crate::platform::Fetch::fetch
/// [`Fetch::fetch_with_settings`]: crate::platform::Fetch::fetch_with_settings
/// [`Fetch::fetch_with_metadata`]: crate::platform::Fetch::fetch_with_metadata
/// [`Fetch::fetch_with_metadata_and_proof`]: crate::platform::Fetch::fetch_with_metadata_and_proof
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

    /// The request this query sends. It always asks for a proof: the answer
    /// is only read from a verified proof, whatever the SDK's proof setting.
    fn request(&self) -> GetIdentitiesContractKeysRequest {
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
                prove: true,
            })),
        }
    }
}

/// Whether `request` asks about no identity, so that nothing needs to be sent for it; see
/// [`IdentitiesContractKeysQuery`].
pub(crate) fn asks_about_no_identity(request: &GetIdentitiesContractKeysRequest) -> bool {
    matches!(&request.version, Some(V0(v0)) if v0.identities_ids.is_empty())
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
        Ok(dapi_request.request())
    }
}

impl Query<GetIdentitiesContractKeysRequest> for IdentitiesContractKeysQuery {
    fn query(
        &self,
        _settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentitiesContractKeysRequest, Error> {
        // Proved whatever `settings.prove` says: a response without a proof
        // would only be refused by the verifier.
        Ok(self.request())
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
        self.request().names_nothing()
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        self.request().execute_transport(client, settings)
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
        assert_eq!(query.request(), expected);
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

    /// A proofs-disabled SDK still sends a proved request: the answer is only
    /// read from a verified proof, so an unproved one could only be refused.
    #[test]
    fn should_ask_for_a_proof_whatever_the_query_settings_say() {
        let query = IdentitiesContractKeysQuery::new(
            vec![Identifier::from([1; 32])],
            Identifier::from([3; 32]),
            None,
            vec![Purpose::AUTHENTICATION],
        )
        .expect("valid query");
        let request_settings = RequestSettings::default();
        let settings = QuerySettings {
            request_settings: &request_settings,
            protocol_version: PlatformVersion::latest(),
            prove: false,
        };
        let sent: GetIdentitiesContractKeysRequest = query.query(&settings).expect("request");
        let Some(V0(sent)) = sent.version else {
            panic!("a v0 request");
        };
        assert!(sent.prove);
    }

    fn query_for(
        identities_ids: Vec<Identifier>,
        purposes: Vec<Purpose>,
    ) -> IdentitiesContractKeysQuery {
        IdentitiesContractKeysQuery::new(identities_ids, Identifier::from([3; 32]), None, purposes)
            .expect("valid query")
    }

    /// The mock knows no request, so anything sent fails and is counted.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_answer_no_identities_with_an_empty_map_and_send_nothing() {
        use crate::platform::Fetch;
        use dpp::identity::identities_contract_keys::IdentitiesContractKeys;

        let mut sdk = crate::Sdk::new_mock();
        for purposes in [vec![Purpose::ENCRYPTION], vec![]] {
            let query = query_for(vec![], purposes);
            let keys = IdentitiesContractKeys::fetch(&sdk, query.clone())
                .await
                .expect("an answer for no identities");
            assert_eq!(keys, Some(IdentitiesContractKeys::new()));
            let sent = sdk.mock().request_count(&query.request()).await;
            assert_eq!(sent, 0, "nothing is sent for no identities");
        }

        // A request built by hand, unproved, is answered the same way.
        let mut request = query_for(vec![], vec![Purpose::ENCRYPTION]).request();
        if let Some(V0(v0)) = request.version.as_mut() {
            v0.prove = false;
        }
        let keys = IdentitiesContractKeys::fetch(&sdk, request.clone())
            .await
            .expect("an answer for no identities");
        assert_eq!(keys, Some(IdentitiesContractKeys::new()));
        assert_eq!(sdk.mock().request_count(&request).await, 0);
    }

    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_send_a_query_that_names_an_identity() {
        use crate::platform::Fetch;
        use dpp::identity::identities_contract_keys::IdentitiesContractKeys;

        let mut sdk = crate::Sdk::new_mock();
        // With a purpose, and without one: the transport refuses the latter.
        for purposes in [vec![Purpose::ENCRYPTION], vec![]] {
            let query = query_for(vec![Identifier::from([1; 32])], purposes);
            IdentitiesContractKeys::fetch(&sdk, query.clone())
                .await
                .expect_err("the mock knows no request");
            assert_eq!(sdk.mock().request_count(&query.request()).await, 1);
        }
    }

    /// With nothing sent there is no metadata or proof to return, so these still send it.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_send_no_identities_when_metadata_or_a_proof_is_asked_for() {
        use crate::platform::Fetch;
        use dpp::identity::identities_contract_keys::IdentitiesContractKeys;

        let mut sdk = crate::Sdk::new_mock();
        let query = query_for(vec![], vec![Purpose::ENCRYPTION]);
        IdentitiesContractKeys::fetch_with_metadata(&sdk, query.clone(), None)
            .await
            .expect_err("no metadata without a response");
        IdentitiesContractKeys::fetch_with_metadata_and_proof(&sdk, query.clone(), None)
            .await
            .expect_err("no proof without a response");
        assert_eq!(sdk.mock().request_count(&query.request()).await, 2);
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
