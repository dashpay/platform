//! The fund a contested document create pays to join its contest.
//!
//! From protocol version 14 the fund doubles once a contest holds 250 contenders and again for
//! every 50 more, and a create states the most it pays: Platform charges it the fund to join
//! and refuses it, paid, when it stated less. A create that names no maximum states the fund to
//! join read here just before it is signed.

use crate::platform::fetch_many::FetchMany;
use crate::{Error, Sdk};
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dpp::voting::vote_polls::VotePoll;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};

impl Sdk {
    /// The fund a contested create of `document` pays to join its contest now, or `None` when
    /// the document joins no contest: the contest's fund, doubled once the contest holds 250
    /// contenders and again for every 50 more (protocol version 14; before it the fund never
    /// doubles).
    ///
    /// The contenders are counted with proved contested resource vote state queries of up to
    /// 100 contenders each, so a contest of `n` contenders costs `n / 100 + 1` of them. The
    /// fund is priced at the protocol version known once they are read, which the first
    /// response of an SDK that auto-detects the version brings.
    pub async fn contest_fund_to_join(
        &self,
        document_type: DocumentTypeRef<'_>,
        document: &Document,
    ) -> Result<Option<Credits>, Error> {
        Ok(self
            .prefunded_voting_balance_to_join(document_type, document)
            .await?
            .map(|(_, contest_fund)| contest_fund))
    }

    /// The prefunded voting balance a create of `document` states to join its contest now:
    /// the name of the contested index the document falls under and [`Self::contest_fund_to_join`],
    /// or `None` when the document joins no contest. It is what a create transition built by
    /// hand carries, the pair [`DocumentCreateTransition`] keeps as `prefunded_voting_balance`.
    ///
    /// [`DocumentCreateTransition`]: dpp::state_transition::batch_transition::DocumentCreateTransition
    pub async fn prefunded_voting_balance_to_join(
        &self,
        document_type: DocumentTypeRef<'_>,
        document: &Document,
    ) -> Result<Option<(String, Credits)>, Error> {
        // The contest is resolved on the document the transition builder sends, with every
        // `generatedFrom` property generated from its params as the platform generates it
        let mut document = document.clone();
        document_type.regenerate_generated_properties(document.properties_mut(), self.version())?;
        let Some(VotePoll::ContestedDocumentResourceVotePoll(vote_poll)) =
            document_type.contested_vote_poll_for_document(&document, self.version())?
        else {
            return Ok(None);
        };

        let mut contenders: u16 = 0;
        let mut start_at = None;
        loop {
            let page = ContenderWithSerializedDocument::fetch_many(
                self,
                ContestedDocumentVotePollDriveQuery {
                    vote_poll: vote_poll.clone(),
                    result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                    offset: None,
                    limit: Some(DEFAULT_QUERY_LIMIT),
                    start_at,
                    allow_include_locked_and_abstaining_vote_tally: false,
                },
            )
            .await?;
            let read = page.contenders.len();
            contenders = contenders.saturating_add(u16::try_from(read).unwrap_or(u16::MAX));
            // A contest accepts at most `max_contenders_per_contest`, and a join past it is
            // refused whatever it states, so counting stops there
            let max_contenders = self.version().system_limits.max_contenders_per_contest;
            match page.contenders.keys().next_back() {
                Some(last)
                    if read >= DEFAULT_QUERY_LIMIT as usize && contenders < max_contenders =>
                {
                    start_at = Some((last.to_buffer(), false));
                }
                _ => break,
            }
        }

        let contest_fund =
            vote_poll.required_vote_resolution_fund_to_join(contenders, self.version());
        Ok(Some((vote_poll.index_name, contest_fund)))
    }
}

/// `options` naming the most a create of `document` pays into the contest it joins: the fund
/// to join the contest now, when the options name no maximum and the document is contested.
/// Callers read it before they reserve an identity contract nonce, so a failed read spends none.
pub(crate) async fn with_contest_fund_to_join(
    sdk: &Sdk,
    document_type: DocumentTypeRef<'_>,
    document: &Document,
    options: Option<StateTransitionCreationOptions>,
) -> Result<Option<StateTransitionCreationOptions>, Error> {
    if options.and_then(|options| options.contest_fund).is_some() {
        return Ok(options);
    }
    let Some(contest_fund) = sdk.contest_fund_to_join(document_type, document).await? else {
        return Ok(options);
    };
    Ok(Some(StateTransitionCreationOptions {
        contest_fund: Some(contest_fund),
        ..options.unwrap_or_default()
    }))
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::DocumentV0;
    use dpp::platform_value::{Identifier, Value};
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use dpp::voting::contender_structs::ContenderWithSerializedDocumentV0;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use drive_proof_verifier::types::Contenders;
    use std::collections::BTreeMap;

    fn dpns_contract() -> dpp::data_contract::DataContract {
        get_dpns_data_contract_fixture(Some(Identifier::new([7; 32])), 0, 14).data_contract_owned()
    }

    /// A DPNS domain document for `label`, which contests the name
    fn domain(label: &str) -> Document {
        DocumentV0 {
            properties: BTreeMap::from([
                ("label".to_string(), Value::Text(label.to_string())),
                (
                    "normalizedLabel".to_string(),
                    Value::Text(label.to_string()),
                ),
                (
                    "parentDomainName".to_string(),
                    Value::Text("dash".to_string()),
                ),
                (
                    "normalizedParentDomainName".to_string(),
                    Value::Text("dash".to_string()),
                ),
            ]),
            ..Default::default()
        }
        .into()
    }

    /// The contest query reading the page of contenders after `start_at`
    fn contenders_page_query(
        contract: &dpp::data_contract::DataContract,
        label: &str,
        start_at: Option<[u8; 32]>,
    ) -> ContestedDocumentVotePollDriveQuery {
        ContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id: contract.id(),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text(label.to_string()),
                ],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            offset: None,
            limit: Some(DEFAULT_QUERY_LIMIT),
            start_at: start_at.map(|start_at| (start_at, false)),
            allow_include_locked_and_abstaining_vote_tally: false,
        }
    }

    fn contender_id(n: u16) -> [u8; 32] {
        let mut id = [0u8; 32];
        id[30..].copy_from_slice(&n.to_be_bytes());
        id
    }

    /// Contenders `from` up to `to`, exclusive
    fn contenders(from: u16, to: u16) -> Contenders {
        Contenders::from_iter((from..to).map(|n| {
            let identity_id = Identifier::new(contender_id(n));
            (
                identity_id,
                Some(ContenderWithSerializedDocument::V0(
                    ContenderWithSerializedDocumentV0 {
                        identity_id,
                        serialized_document: None,
                        vote_tally: Some(0),
                    },
                )),
            )
        }))
    }

    /// The fund to join a contest of 250 contenders is twice the contest's fund, read page by
    /// page, and it is what a create naming no maximum states, on the contested index
    #[tokio::test]
    async fn should_state_the_fund_to_join_a_contest_read_page_by_page() {
        let mut sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk");
        let contract = dpns_contract();
        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        let fund = sdk
            .version()
            .fee_version
            .vote_resolution_fund_fees
            .contested_document_vote_resolution_fund_required_amount;

        for (start_at, page) in [
            (None, contenders(0, 100)),
            (Some(contender_id(99)), contenders(100, 200)),
            (Some(contender_id(199)), contenders(200, 250)),
        ] {
            sdk.mock()
                .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                    contenders_page_query(&contract, "quantum", start_at),
                    Some(page),
                )
                .await
                .expect("expected to register the page");
        }

        let options = with_contest_fund_to_join(&sdk, document_type, &domain("quantum"), None)
            .await
            .expect("expected to read the fund to join");
        assert_eq!(
            options.and_then(|options| options.contest_fund),
            Some(2 * fund)
        );
        assert_eq!(
            sdk.prefunded_voting_balance_to_join(document_type, &domain("quantum"))
                .await
                .expect("expected to read the prefunded voting balance to join"),
            Some(("parentNameAndLabel".to_string(), 2 * fund))
        );
    }

    /// Nothing is read for a create joining no contest (its type has no contested index, or its
    /// values match none), or one naming the most it pays
    #[tokio::test]
    async fn should_read_nothing_when_the_create_joins_no_contest_or_names_its_maximum() {
        let sdk = SdkBuilder::new_mock().build().expect("expected a mock sdk");
        let contract = dpns_contract();
        let domain_type = contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        let preorder_type = contract
            .document_type_for_name("preorder")
            .expect("expected the preorder document type");

        let preorder: Document = DocumentV0 {
            properties: BTreeMap::from([("saltedDomainHash".to_string(), Value::Bytes32([1; 32]))]),
            ..Default::default()
        }
        .into();
        assert_eq!(
            with_contest_fund_to_join(&sdk, preorder_type, &preorder, None)
                .await
                .expect("expected no read"),
            None
        );
        assert_eq!(
            sdk.prefunded_voting_balance_to_join(preorder_type, &preorder)
                .await
                .expect("expected no read"),
            None
        );
        // A label of 20 characters or with digits other than 0 and 1 is not contested
        for label in ["quantumexplorerdashx", "quantum2"] {
            assert_eq!(
                sdk.prefunded_voting_balance_to_join(domain_type, &domain(label))
                    .await
                    .expect("expected no read"),
                None
            );
        }

        let naming_its_maximum = Some(StateTransitionCreationOptions {
            contest_fund: Some(5),
            ..Default::default()
        });
        assert_eq!(
            with_contest_fund_to_join(&sdk, domain_type, &domain("quantum"), naming_its_maximum)
                .await
                .expect("expected no read"),
            naming_its_maximum
        );
    }
}
