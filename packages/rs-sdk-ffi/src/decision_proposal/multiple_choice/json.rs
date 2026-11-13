//! The JSON the multiple choice proposal functions return, one shape per kind, written from
//! the rs-sdk types. Field names follow the wasm bindings
//! (`packages/wasm-sdk/src/decision_proposals/multiple_choice.rs`) and the yes/no proposal
//! JSON of `super::super::json`: camelCase, base58 ids, absent values as null.

use super::super::json::block_json;
use crate::FFIError;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::platform::decision_proposals::multiple_choice::{
    Choice, MultipleChoiceProposal, MultipleChoiceProposalVote, MultipleChoiceVoteChoice,
};
use drive_proof_verifier::types::Documents;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};

/// The document a multiple choice proposal points at.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DescriptionDocumentJson<'a> {
    /// The document, base58.
    id: String,
    /// The contract it lives in, base58.
    contract_id: String,
    document_type: &'a str,
}

/// A multiple choice proposal as every multiple choice proposal read returns it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultipleChoiceProposalJson<'a> {
    /// The proposal's id, base58.
    id: String,
    /// The proposer, base58.
    owner_id: String,
    revision: Option<u64>,
    /// Milliseconds since the Unix epoch.
    created_at: Option<u64>,
    /// Milliseconds since the Unix epoch.
    updated_at: Option<u64>,
    title: &'a str,
    description_document: Option<DescriptionDocumentJson<'a>>,
    url: Option<&'a str>,
    open: bool,
}

/// A choice as every read of one returns it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChoiceJson<'a> {
    /// The choice's id, base58: the id a masternode vote towards it names.
    id: String,
    /// The identity that entered it, base58.
    owner_id: String,
    /// The proposal it answers, base58.
    proposal_id: String,
    revision: Option<u64>,
    /// Milliseconds since the Unix epoch.
    created_at: Option<u64>,
    /// Milliseconds since the Unix epoch.
    updated_at: Option<u64>,
    title: &'a str,
    description: Option<&'a str>,
}

/// `document`, a `multipleChoiceProposal` document, as the proposal JSON object.
pub(super) fn multiple_choice_proposal_json(document: Document) -> Result<JsonValue, FFIError> {
    let proposal = MultipleChoiceProposal::from_document(document)?;
    let document = &proposal.document;
    Ok(serde_json::to_value(MultipleChoiceProposalJson {
        id: document.id().to_string(Encoding::Base58),
        owner_id: document.owner_id().to_string(Encoding::Base58),
        revision: document.revision(),
        created_at: document.created_at(),
        updated_at: document.updated_at(),
        title: &proposal.title,
        description_document: proposal.description_document.as_ref().map(|described| {
            DescriptionDocumentJson {
                id: described.id.to_string(Encoding::Base58),
                contract_id: described.contract_id.to_string(Encoding::Base58),
                document_type: &described.document_type,
            }
        }),
        url: proposal.url.as_deref(),
        open: proposal.open,
    })?)
}

/// One page of a listing as a JSON array of proposals, in the listing's order.
pub(super) fn multiple_choice_proposals_json(documents: Documents) -> Result<JsonValue, FFIError> {
    documents
        .into_values()
        .flatten()
        .map(multiple_choice_proposal_json)
        .collect::<Result<Vec<_>, _>>()
        .map(JsonValue::Array)
}

/// `choice` as the choice JSON object.
fn choice_json(choice: &Choice) -> Result<JsonValue, FFIError> {
    let document = &choice.document;
    Ok(serde_json::to_value(ChoiceJson {
        id: document.id().to_string(Encoding::Base58),
        owner_id: document.owner_id().to_string(Encoding::Base58),
        proposal_id: choice.proposal_id.to_string(Encoding::Base58),
        revision: document.revision(),
        created_at: document.created_at(),
        updated_at: document.updated_at(),
        title: &choice.title,
        description: choice.description.as_deref(),
    })?)
}

/// `document`, a `choice` document, as the choice JSON object.
pub(super) fn choice_document_json(document: Document) -> Result<JsonValue, FFIError> {
    choice_json(&Choice::from_document(document)?)
}

/// The vote among a proposal's choices: every choice by id with its tally, the other
/// tallies, and how the vote ended once it did.
pub(super) fn multiple_choice_proposal_vote_json(
    vote: &MultipleChoiceProposalVote,
) -> Result<JsonValue, FFIError> {
    let choices = vote
        .choices
        .iter()
        .map(|(choice_id, choice_in_vote)| {
            Ok(json!({
                "id": choice_id.to_string(Encoding::Base58),
                "choice": choice_in_vote.choice.as_ref().map(choice_json).transpose()?,
                "voteTally": choice_in_vote.vote_tally,
            }))
        })
        .collect::<Result<Vec<_>, FFIError>>()?;
    let result = vote.winner.as_ref().map(|(winner, finalization_block)| {
        let outcome = match winner {
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(_) => "keptChoice",
            ContestedDocumentVotePollWinnerInfo::Locked => "noneOfTheChoices",
            ContestedDocumentVotePollWinnerInfo::NoWinner => "noWinner",
        };
        json!({
            "outcome": outcome,
            "keptChoiceId": vote
                .kept_choice_id()
                .map(|choice_id| choice_id.to_string(Encoding::Base58)),
            "finalizationBlock": block_json(finalization_block),
        })
    });
    Ok(json!({
        "choices": choices,
        "abstainVoteTally": vote.abstain_vote_tally,
        "noneOfTheChoicesVoteTally": vote.none_of_the_choices_vote_tally,
        "result": result,
    }))
}

/// The name a masternode's answer is written with: `"choice"`, `"noneOfTheChoices"` or
/// `"abstain"`.
pub(super) fn vote_choice_name(vote_choice: &MultipleChoiceVoteChoice) -> &'static str {
    match vote_choice {
        MultipleChoiceVoteChoice::Choice(_) => "choice",
        MultipleChoiceVoteChoice::NoneOfTheChoices => "noneOfTheChoices",
        MultipleChoiceVoteChoice::Abstain => "abstain",
    }
}

/// A masternode's answer; `None` when it has not voted in the vote.
pub(super) fn masternode_vote_json(
    vote_choice: Option<MultipleChoiceVoteChoice>,
) -> Option<JsonValue> {
    vote_choice.map(|vote_choice| {
        let choice_id = match vote_choice {
            MultipleChoiceVoteChoice::Choice(choice_id) => {
                Some(choice_id.to_string(Encoding::Base58))
            }
            MultipleChoiceVoteChoice::NoneOfTheChoices | MultipleChoiceVoteChoice::Abstain => None,
        };
        json!({
            "choice": vote_choice_name(&vote_choice),
            "choiceId": choice_id,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::json::tests::{proposals_contract, CREATED_AT};
    use super::*;
    use dash_sdk::dpp::block::block_info::BlockInfo;
    use dash_sdk::dpp::block::epoch::Epoch;
    use dash_sdk::dpp::decision_proposal::multiple_choice::proposal_properties;
    use dash_sdk::dpp::document::DocumentV0Setters;
    use dash_sdk::dpp::platform_value::{Bytes32, Identifier};
    use dash_sdk::platform::decision_proposals::multiple_choice::{
        build_choice_document, build_multiple_choice_proposal_document, ChoiceInVote, ChoiceInput,
        DescriptionDocument, MultipleChoiceProposalInput,
    };
    use std::collections::BTreeMap;

    fn base58(id: Identifier) -> String {
        id.to_string(Encoding::Base58)
    }

    fn input() -> MultipleChoiceProposalInput {
        MultipleChoiceProposalInput {
            title: "Which block size?".to_string(),
            description_document: Some(DescriptionDocument {
                id: Identifier::from([5; 32]),
                contract_id: Identifier::from([6; 32]),
                document_type: "post".to_string(),
            }),
            url: Some("https://forum.dash.org/t/block-size".to_string()),
            open: true,
        }
    }

    /// A multiple choice proposal of identity `[3; 32]` built by rs-sdk, as Platform stores
    /// it.
    fn stored_proposal(input: &MultipleChoiceProposalInput) -> Document {
        let mut document = build_multiple_choice_proposal_document(
            &proposals_contract(),
            Identifier::from([3; 32]),
            input,
            Bytes32::new([9; 32]),
        )
        .expect("builds")
        .document;
        document.set_created_at(Some(CREATED_AT));
        document.set_updated_at(Some(CREATED_AT));
        document
    }

    /// A choice of identity `[4; 32]` for the proposal `[7; 32]`, as Platform stores it.
    fn stored_choice(description: Option<&str>) -> Document {
        let mut document = build_choice_document(
            &proposals_contract(),
            Identifier::from([4; 32]),
            &ChoiceInput {
                proposal_id: Identifier::from([7; 32]),
                title: "Double it".to_string(),
                description: description.map(str::to_string),
            },
            Bytes32::new([8; 32]),
        )
        .expect("builds")
        .document;
        document.set_created_at(Some(CREATED_AT));
        document
    }

    fn block(height: u64) -> BlockInfo {
        BlockInfo {
            time_ms: 1_000 * height,
            height,
            core_height: 42,
            epoch: Epoch::new(3).expect("an epoch"),
        }
    }

    #[test]
    fn should_write_a_proposal_with_every_field() {
        let document = stored_proposal(&input());
        let id = document.id();
        assert_eq!(
            multiple_choice_proposal_json(document).expect("a proposal"),
            json!({
                "id": base58(id),
                "ownerId": base58(Identifier::from([3; 32])),
                "revision": 1,
                "createdAt": CREATED_AT,
                "updatedAt": CREATED_AT,
                "title": "Which block size?",
                "descriptionDocument": {
                    "id": base58(Identifier::from([5; 32])),
                    "contractId": base58(Identifier::from([6; 32])),
                    "documentType": "post",
                },
                "url": "https://forum.dash.org/t/block-size",
                "open": true,
            })
        );
    }

    #[test]
    fn should_write_an_absent_description_document_and_url_as_null() {
        let document = stored_proposal(&MultipleChoiceProposalInput {
            description_document: None,
            url: None,
            open: false,
            ..input()
        });
        let proposal = multiple_choice_proposal_json(document).expect("a proposal");
        assert_eq!(proposal["descriptionDocument"], JsonValue::Null);
        assert_eq!(proposal["url"], JsonValue::Null);
        assert_eq!(proposal["open"], json!(false));
    }

    #[test]
    fn should_refuse_a_document_that_is_not_a_multiple_choice_proposal() {
        let mut document = stored_proposal(&input());
        document.remove(proposal_properties::TITLE);
        assert!(multiple_choice_proposal_json(document).is_err());
        assert!(choice_document_json(stored_proposal(&input())).is_err());
    }

    #[test]
    fn should_write_a_listing_in_its_order_without_absent_entries() {
        let first = stored_proposal(&input());
        let mut second = stored_proposal(&input());
        second.set_id(Identifier::from([1; 32]));
        let documents: Documents = [
            (first.id(), Some(first.clone())),
            (Identifier::from([2; 32]), None),
            (second.id(), Some(second.clone())),
        ]
        .into_iter()
        .collect();
        let listing = multiple_choice_proposals_json(documents).expect("proposals");
        let ids: Vec<&str> = listing
            .as_array()
            .expect("an array")
            .iter()
            .map(|proposal| proposal["id"].as_str().expect("an id"))
            .collect();
        assert_eq!(ids, [base58(first.id()), base58(second.id())]);
        assert_eq!(
            multiple_choice_proposals_json(Documents::default()).expect("empty"),
            json!([])
        );
    }

    #[test]
    fn should_write_a_choice_with_every_field_and_an_absent_description_as_null() {
        let document = stored_choice(Some("Twice the size it is today."));
        let id = document.id();
        assert_eq!(
            choice_document_json(document).expect("a choice"),
            json!({
                "id": base58(id),
                "ownerId": base58(Identifier::from([4; 32])),
                "proposalId": base58(Identifier::from([7; 32])),
                "revision": null,
                "createdAt": CREATED_AT,
                "updatedAt": null,
                "title": "Double it",
                "description": "Twice the size it is today.",
            })
        );
        assert_eq!(
            choice_document_json(stored_choice(None)).expect("a choice")["description"],
            JsonValue::Null
        );
    }

    #[test]
    fn should_write_a_running_vote_with_its_choices_and_tallies() {
        let double = stored_choice(Some("Twice the size it is today."));
        let left_id = Identifier::from([2; 32]);
        let vote = MultipleChoiceProposalVote {
            choices: BTreeMap::from([
                (
                    double.id(),
                    ChoiceInVote {
                        choice: Some(Choice::from_document(double.clone()).expect("reads")),
                        vote_tally: Some(12),
                    },
                ),
                (
                    left_id,
                    ChoiceInVote {
                        choice: None,
                        vote_tally: None,
                    },
                ),
            ]),
            abstain_vote_tally: Some(3),
            none_of_the_choices_vote_tally: Some(1),
            winner: None,
        };
        // The choices are written in the order of their ids
        let mut choices = vec![
            (
                double.id(),
                json!({
                    "id": base58(double.id()),
                    "choice": choice_document_json(double.clone()).expect("a choice"),
                    "voteTally": 12,
                }),
            ),
            (
                left_id,
                json!({"id": base58(left_id), "choice": null, "voteTally": null}),
            ),
        ];
        choices.sort_by_key(|(id, _)| *id);
        let choices: Vec<JsonValue> = choices.into_iter().map(|(_, choice)| choice).collect();
        assert_eq!(
            multiple_choice_proposal_vote_json(&vote).expect("writes"),
            json!({
                "choices": choices,
                "abstainVoteTally": 3,
                "noneOfTheChoicesVoteTally": 1,
                "result": null,
            })
        );
    }

    #[test]
    fn should_write_how_a_finished_vote_ended() {
        let kept_id = Identifier::from([2; 32]);
        let finished = |winner| MultipleChoiceProposalVote {
            choices: BTreeMap::from([(
                kept_id,
                ChoiceInVote {
                    choice: None,
                    vote_tally: Some(9),
                },
            )]),
            abstain_vote_tally: None,
            none_of_the_choices_vote_tally: Some(4),
            winner: Some((winner, block(9))),
        };
        let finalization_block =
            json!({"height": 9, "coreHeight": 42, "timeMs": 9_000, "epoch": 3});
        for (winner, outcome, kept_choice_id) in [
            (
                ContestedDocumentVotePollWinnerInfo::WonByIdentity(kept_id),
                "keptChoice",
                json!(base58(kept_id)),
            ),
            (
                ContestedDocumentVotePollWinnerInfo::Locked,
                "noneOfTheChoices",
                JsonValue::Null,
            ),
            (
                ContestedDocumentVotePollWinnerInfo::NoWinner,
                "noWinner",
                JsonValue::Null,
            ),
        ] {
            assert_eq!(
                multiple_choice_proposal_vote_json(&finished(winner)).expect("writes"),
                json!({
                    "choices": [{"id": base58(kept_id), "choice": null, "voteTally": 9}],
                    "abstainVoteTally": null,
                    "noneOfTheChoicesVoteTally": 4,
                    "result": {
                        "outcome": outcome,
                        "keptChoiceId": kept_choice_id,
                        "finalizationBlock": finalization_block,
                    },
                })
            );
        }
    }

    #[test]
    fn should_write_a_vote_with_no_choice_as_empty() {
        assert_eq!(
            multiple_choice_proposal_vote_json(&MultipleChoiceProposalVote::default())
                .expect("writes"),
            json!({
                "choices": [],
                "abstainVoteTally": null,
                "noneOfTheChoicesVoteTally": null,
                "result": null,
            })
        );
    }

    #[test]
    fn should_write_a_masternode_answer() {
        let choice_id = Identifier::from([9; 32]);
        assert_eq!(
            masternode_vote_json(Some(MultipleChoiceVoteChoice::Choice(choice_id))),
            Some(json!({"choice": "choice", "choiceId": base58(choice_id)}))
        );
        assert_eq!(
            masternode_vote_json(Some(MultipleChoiceVoteChoice::NoneOfTheChoices)),
            Some(json!({"choice": "noneOfTheChoices", "choiceId": null}))
        );
        assert_eq!(
            masternode_vote_json(Some(MultipleChoiceVoteChoice::Abstain)),
            Some(json!({"choice": "abstain", "choiceId": null}))
        );
        assert_eq!(masternode_vote_json(None), None);
    }
}
