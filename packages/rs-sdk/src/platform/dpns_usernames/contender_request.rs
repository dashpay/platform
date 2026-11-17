//! What a contender of a contested DPNS name asked for.
//!
//! Every contender of a contested name carries the `domain` document it
//! submitted. The contest is keyed by the normalized label ("p1zza"), while
//! the document records the spelling the requester typed ("pizza") and when
//! the request was made. Bindings read both from here rather than decoding
//! the document themselves.

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use std::collections::HashSet;
use tracing::debug;

/// One contender's request, read from its own document. Both fields are
/// `None` when the document could not be decoded.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContenderRequest {
    /// The label exactly as the contender typed it. `None` when empty.
    pub label: Option<String>,
    /// The document's `$createdAt`: when this request was submitted. Passed
    /// on as the document holds it, so a present 0 stays `Some(0)`.
    pub created_at: Option<TimestampMillis>,
}

impl ContenderRequest {
    /// Decode `contender`'s document against `document_type` and read the
    /// request from it.
    ///
    /// Best-effort: a missing document type, a contender without a serialized
    /// document, or a document that does not decode gives an empty request
    /// rather than an error, so one bad contender never fails a whole query.
    pub fn decode(
        contender: &ContenderWithSerializedDocument,
        document_type: Option<DocumentTypeRef>,
        platform_version: &PlatformVersion,
    ) -> Self {
        let Some(document_type) = document_type else {
            return Self::default();
        };
        let decoded = match contender.try_to_contender(document_type, platform_version) {
            Ok(decoded) => decoded,
            Err(error) => {
                debug!(
                    contender = %contender.identity_id_ref(),
                    %error,
                    "contender document did not decode; its request is left empty"
                );
                return Self::default();
            }
        };
        Self::from_document(decoded.document().as_ref())
    }

    /// Read the request from an already decoded `domain` document.
    fn from_document(document: Option<&Document>) -> Self {
        let Some(document) = document else {
            return Self::default();
        };
        Self {
            label: document
                .get("label")
                .and_then(|value| value.as_str())
                .filter(|label| !label.is_empty())
                .map(str::to_string),
            created_at: document.created_at(),
        }
    }
}

/// Every contender's request in one contest, and the spellings they asked for.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContestRequests {
    /// One request per contender, in the order the contenders were given.
    pub requests: Vec<ContenderRequest>,
    /// The distinct requested spellings, each once, in contender order.
    /// Contenders whose label did not decode contribute nothing.
    pub requested_labels: Vec<String>,
}

impl ContestRequests {
    /// Decode each contender's document once (see [`ContenderRequest::decode`])
    /// and derive the contest's requested labels from the results.
    pub fn decode<'a>(
        contenders: impl IntoIterator<Item = &'a ContenderWithSerializedDocument>,
        document_type: Option<DocumentTypeRef>,
        platform_version: &PlatformVersion,
    ) -> Self {
        let requests: Vec<ContenderRequest> = contenders
            .into_iter()
            .map(|contender| ContenderRequest::decode(contender, document_type, platform_version))
            .collect();
        let requested_labels = requested_labels(&requests);
        Self {
            requests,
            requested_labels,
        }
    }
}

/// The spellings requested for one contest, each once, in contender order.
///
/// Contenders whose label did not decode contribute nothing.
fn requested_labels<'a>(requests: impl IntoIterator<Item = &'a ContenderRequest>) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut labels: Vec<String> = Vec::new();
    for label in requests
        .into_iter()
        .filter_map(|request| request.label.as_deref())
    {
        if seen.insert(label) {
            labels.push(label.to_string());
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::DocumentV0;
    use dpp::platform_value::{Identifier, Value};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::voting::contender_structs::{
        Contender, ContenderV0, ContenderWithSerializedDocumentV0,
    };
    use std::collections::BTreeMap;

    const CREATED_AT_MS: u64 = 1_760_140_800_000;

    fn domain_document(label: &str, created_at: Option<u64>) -> Document {
        let owner_id = Identifier::from([7u8; 32]);
        let text = |value: &str| Value::Text(value.to_string());
        let properties = BTreeMap::from([
            ("parentDomainName".to_string(), text("dash")),
            ("normalizedParentDomainName".to_string(), text("dash")),
            ("label".to_string(), text(label)),
            ("normalizedLabel".to_string(), text("p1zza")),
            ("preorderSalt".to_string(), Value::Bytes32([3u8; 32])),
            (
                "records".to_string(),
                Value::Map(vec![(
                    text("identity"),
                    Value::Identifier(owner_id.to_buffer()),
                )]),
            ),
            (
                "subdomainRules".to_string(),
                Value::Map(vec![(text("allowSubdomains"), Value::Bool(false))]),
            ),
        ]);
        Document::V0(DocumentV0 {
            id: Identifier::from([107u8; 32]),
            owner_id,
            properties,
            revision: Some(1),
            created_at,
            updated_at: created_at,
            transferred_at: created_at,
            ..Default::default()
        })
    }

    fn decode(contender: &ContenderWithSerializedDocument) -> ContenderRequest {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS system contract");
        let document_type = contract.document_type_for_name("domain").ok();
        ContenderRequest::decode(contender, document_type, platform_version)
    }

    fn serialized(document: Document) -> ContenderWithSerializedDocument {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS system contract");
        let document_type = contract.document_type_for_name("domain").expect("domain");
        Contender::V0(ContenderV0 {
            identity_id: document.owner_id(),
            document: Some(document),
            vote_tally: Some(3),
        })
        .try_into_contender_with_serialized_document(document_type, &contract, platform_version)
        .expect("serialize contender")
    }

    fn request(label: Option<&str>) -> ContenderRequest {
        ContenderRequest {
            label: label.map(str::to_string),
            created_at: None,
        }
    }

    #[test]
    fn should_read_label_and_created_at() {
        let contender = serialized(domain_document("pizza", Some(CREATED_AT_MS)));
        assert_eq!(
            decode(&contender),
            ContenderRequest {
                label: Some("pizza".to_string()),
                created_at: Some(CREATED_AT_MS),
            }
        );
    }

    #[test]
    fn should_keep_a_present_zero_created_at_through_serialization() {
        let contender = serialized(domain_document("pizza", Some(0)));
        assert_eq!(decode(&contender).created_at, Some(0));
    }

    #[test]
    fn should_keep_the_label_without_created_at() {
        let document = domain_document("pizza", None);
        assert_eq!(
            ContenderRequest::from_document(Some(&document)),
            ContenderRequest {
                label: Some("pizza".to_string()),
                created_at: None,
            }
        );
    }

    #[test]
    fn should_pass_a_present_zero_created_at_through() {
        let document = domain_document("pizza", Some(0));
        assert_eq!(
            ContenderRequest::from_document(Some(&document)).created_at,
            Some(0)
        );
    }

    #[test]
    fn should_leave_an_empty_label_out() {
        let document = domain_document("", Some(CREATED_AT_MS));
        assert_eq!(ContenderRequest::from_document(Some(&document)).label, None);
    }

    #[test]
    fn should_give_an_empty_request_for_an_undecodable_document() {
        let contender = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: Identifier::from([7u8; 32]),
            serialized_document: Some(vec![0xde, 0xad, 0xbe, 0xef]),
            vote_tally: Some(1),
        });
        assert_eq!(decode(&contender), ContenderRequest::default());
    }

    #[test]
    fn should_give_an_empty_request_without_a_document() {
        let contender = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: Identifier::from([7u8; 32]),
            serialized_document: None,
            vote_tally: Some(1),
        });
        assert_eq!(decode(&contender), ContenderRequest::default());
    }

    #[test]
    fn should_give_an_empty_request_without_a_document_type() {
        let contender = serialized(domain_document("pizza", Some(CREATED_AT_MS)));
        assert_eq!(
            ContenderRequest::decode(&contender, None, PlatformVersion::latest()),
            ContenderRequest::default()
        );
    }

    #[test]
    fn should_list_each_requested_label_once_in_contender_order() {
        let requests = [
            request(Some("pizza")),
            request(None),
            request(Some("Pizza")),
            request(Some("pizza")),
        ];
        assert_eq!(requested_labels(&requests), vec!["pizza", "Pizza"]);
    }

    #[test]
    fn should_decode_a_contest_in_contender_order_with_its_labels() {
        let undecodable = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: Identifier::from([9u8; 32]),
            serialized_document: Some(vec![0xde, 0xad]),
            vote_tally: Some(1),
        });
        let contenders = [
            serialized(domain_document("pizza", Some(CREATED_AT_MS))),
            undecodable,
            serialized(domain_document("Pizza", Some(CREATED_AT_MS + 2))),
            serialized(domain_document("pizza", Some(CREATED_AT_MS + 1))),
        ];
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS system contract");
        let document_type = contract.document_type_for_name("domain").ok();

        let contest = ContestRequests::decode(&contenders, document_type, platform_version);

        assert_eq!(
            contest
                .requests
                .iter()
                .map(|request| request.created_at)
                .collect::<Vec<_>>(),
            vec![
                Some(CREATED_AT_MS),
                None,
                Some(CREATED_AT_MS + 2),
                Some(CREATED_AT_MS + 1)
            ]
        );
        assert_eq!(contest.requested_labels, vec!["pizza", "Pizza"]);
    }

    #[test]
    fn should_list_no_labels_when_none_decoded() {
        assert!(requested_labels(&[request(None)]).is_empty());
    }
}
