//! The JSON the decision proposal functions return, one shape per kind, written from the
//! rs-sdk types. Field names follow the wasm bindings (`packages/wasm-sdk/src/decision_proposals.rs`
//! and `packages/wasm-sdk/src/queries/voting/yes_no.rs`).

use crate::FFIError;
use dash_sdk::dpp::block::block_info::BlockInfo;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::serialization::JsonConvertible;
use dash_sdk::dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dash_sdk::dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::platform::decision_proposals::DecisionProposal;
use dash_sdk::platform::yes_no_vote_polls::{MasternodeYesNoVote, YesNoVotePollState};
use drive_proof_verifier::types::Documents;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};

/// A proposal as every decision proposal read returns it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalJson<'a> {
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
    description: &'a str,
    url: Option<&'a str>,
    pass_percent: u8,
    minimum_turnout_percent: u8,
    /// Milliseconds since the Unix epoch.
    end_date: u64,
}

/// `document`, a `proposal` document, as the proposal JSON object.
pub(super) fn proposal_json(document: Document) -> Result<JsonValue, FFIError> {
    let proposal = DecisionProposal::from_document(document)?;
    let document = &proposal.document;
    Ok(serde_json::to_value(ProposalJson {
        id: document.id().to_string(Encoding::Base58),
        owner_id: document.owner_id().to_string(Encoding::Base58),
        revision: document.revision(),
        created_at: document.created_at(),
        updated_at: document.updated_at(),
        title: &proposal.title,
        description: &proposal.description,
        url: proposal.url.as_deref(),
        pass_percent: proposal.pass_percent,
        minimum_turnout_percent: proposal.minimum_turnout_percent,
        end_date: proposal.end_date,
    })?)
}

/// One page of a listing as a JSON array of proposals, in the listing's order.
pub(super) fn proposals_json(documents: Documents) -> Result<JsonValue, FFIError> {
    documents
        .into_values()
        .flatten()
        .map(proposal_json)
        .collect::<Result<Vec<_>, _>>()
        .map(JsonValue::Array)
}

/// The properties of `document` as the JSON object the platform wallet's document create and
/// replace take (`properties_json`).
pub(super) fn properties_json(document: &Document) -> Result<JsonValue, FFIError> {
    Ok(serde_json::to_value(document.properties())?)
}

fn block_json(block: &BlockInfo) -> JsonValue {
    json!({
        "height": block.height,
        "coreHeight": block.core_height,
        "timeMs": block.time_ms,
        "epoch": block.epoch.index,
    })
}

/// The state of an opened poll; `None` for a poll never opened.
pub(super) fn vote_poll_state_json(
    state: Option<YesNoVotePollState>,
) -> Result<Option<JsonValue>, FFIError> {
    let Some(state) = state else {
        return Ok(None);
    };
    let Some(stored_info) = state.stored_info else {
        return Ok(None);
    };
    let vote_poll = VotePoll::YesNoVotePoll(stored_info.vote_poll().clone())
        .to_json()
        .map_err(|e| FFIError::InternalError(format!("Failed to write the vote poll: {e}")))?;
    let mut object = json!({
        "votePoll": vote_poll,
        "yesVotingPower": state.yes_voting_power,
        "noVotingPower": state.no_voting_power,
        "abstainVotingPower": state.abstain_voting_power,
    });
    match stored_info.status() {
        YesNoVotePollStatus::Started(start_block) => {
            object["status"] = json!("started");
            object["startBlock"] = block_json(start_block);
        }
        YesNoVotePollStatus::Finished(result) => {
            object["status"] = json!("finished");
            object["result"] = json!({
                "passed": result.passed,
                "yesVotingPower": result.yes_voting_power,
                "noVotingPower": result.no_voting_power,
                "abstainVotingPower": result.abstain_voting_power,
                "requiredVotingPower": result.required_voting_power,
                "startBlock": block_json(&result.start_block),
                "finalizationBlock": block_json(&result.finalization_block),
            });
        }
    }
    Ok(Some(object))
}

/// The name a vote choice is written with: `"yes"`, `"no"` or `"abstain"`.
pub(super) fn vote_choice_name(vote_choice: YesNoAbstainVoteChoice) -> &'static str {
    match vote_choice {
        YesNoAbstainVoteChoice::Yes => "yes",
        YesNoAbstainVoteChoice::No => "no",
        YesNoAbstainVoteChoice::Abstain => "abstain",
    }
}

/// A masternode's answer; `None` when it has not voted on the poll.
pub(super) fn masternode_vote_json(vote: Option<MasternodeYesNoVote>) -> Option<JsonValue> {
    vote.map(|vote| {
        json!({
            "choice": vote_choice_name(vote.vote_choice),
            "timesVoted": vote.times_voted,
        })
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use dash_sdk::dpp::block::epoch::Epoch;
    use dash_sdk::dpp::data_contract::DataContract;
    use dash_sdk::dpp::decision_proposal::properties;
    use dash_sdk::dpp::document::DocumentV0Setters;
    use dash_sdk::dpp::platform_value::{Bytes32, Identifier};
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::v0::YesNoVotePollStoredInfoV0;
    use dash_sdk::dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
    use dash_sdk::dpp::voting::vote_polls::yes_no_vote_poll::{
        VotingPowerRounding, YesNoMinimumVotingPower, YesNoVotePoll, YesNoVotePollPurpose,
    };
    use dash_sdk::platform::decision_proposals::{
        build_decision_proposal_document, DecisionProposalInput,
    };

    pub(in crate::decision_proposal) const CREATED_AT: u64 = 1_700_000_000_000;
    pub(in crate::decision_proposal) const END_DATE: u64 = CREATED_AT + 14 * 86_400_000;

    pub(in crate::decision_proposal) fn proposals_contract() -> DataContract {
        load_system_data_contract(
            SystemDataContract::DecisionProposals,
            PlatformVersion::latest(),
        )
        .expect("the decision proposals contract loads at the latest version")
    }

    pub(in crate::decision_proposal) fn input() -> DecisionProposalInput {
        DecisionProposalInput {
            title: "Raise the block size?".to_string(),
            description: "Yes doubles it; no keeps it.".to_string(),
            url: Some("https://forum.dash.org/t/block-size".to_string()),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date: END_DATE,
        }
    }

    /// A proposal of identity `[3; 32]` built by rs-sdk, as Platform stores it.
    pub(in crate::decision_proposal) fn stored_proposal(input: &DecisionProposalInput) -> Document {
        let mut document = build_decision_proposal_document(
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

    #[test]
    fn should_write_a_proposal_with_every_field() {
        let document = stored_proposal(&input());
        let id = document.id();
        assert_eq!(
            proposal_json(document).expect("a proposal"),
            json!({
                "id": id.to_string(Encoding::Base58),
                "ownerId": Identifier::from([3; 32]).to_string(Encoding::Base58),
                "revision": 1,
                "createdAt": CREATED_AT,
                "updatedAt": CREATED_AT,
                "title": "Raise the block size?",
                "description": "Yes doubles it; no keeps it.",
                "url": "https://forum.dash.org/t/block-size",
                "passPercent": 60,
                "minimumTurnoutPercent": 10,
                "endDate": END_DATE,
            })
        );
    }

    #[test]
    fn should_write_an_absent_url_as_null() {
        let document = stored_proposal(&DecisionProposalInput {
            url: None,
            ..input()
        });
        assert_eq!(
            proposal_json(document).expect("a proposal")["url"],
            JsonValue::Null
        );
    }

    #[test]
    fn should_refuse_a_document_that_is_not_a_proposal() {
        let mut document = stored_proposal(&input());
        document.remove(properties::TITLE);
        assert!(proposal_json(document).is_err());
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
        let listing = proposals_json(documents).expect("proposals");
        let ids: Vec<&str> = listing
            .as_array()
            .expect("an array")
            .iter()
            .map(|proposal| proposal["id"].as_str().expect("an id"))
            .collect();
        assert_eq!(
            ids,
            [
                first.id().to_string(Encoding::Base58),
                second.id().to_string(Encoding::Base58)
            ]
        );
        assert_eq!(
            proposals_json(Documents::default()).expect("empty"),
            json!([])
        );
    }

    /// The properties are the document's own, which the wallet's create takes back.
    #[test]
    fn should_write_the_properties_a_create_takes() {
        let document = stored_proposal(&input());
        assert_eq!(
            properties_json(&document).expect("properties"),
            json!({
                "title": "Raise the block size?",
                "description": "Yes doubles it; no keeps it.",
                "url": "https://forum.dash.org/t/block-size",
                "passPercent": 60,
                "minimumTurnoutPercent": 10,
                "endDate": END_DATE,
            })
        );
    }

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            purpose: YesNoVotePollPurpose::DecisionProposal {
                document_id: Identifier::new([0xc1; 32]),
            },
            supermajority_numerator: 60,
            supermajority_denominator: 100,
            minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                numerator: 10,
                denominator: 100,
                rounding: VotingPowerRounding::Up,
            },
        }
    }

    fn poll_json() -> JsonValue {
        json!({
            "$type": "yesNoVotePoll",
            "purpose": {
                "$type": "decisionProposal",
                "documentId": Identifier::new([0xc1; 32]).to_string(Encoding::Base58),
            },
            "supermajorityNumerator": 60,
            "supermajorityDenominator": 100,
            "minimumVotingPower": {
                "$type": "fractionOfTotal",
                "numerator": 10,
                "denominator": 100,
                "rounding": "up",
            },
        })
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
    fn should_write_a_started_poll_with_its_tallies() {
        let state = YesNoVotePollState {
            stored_info: Some(YesNoVotePollStoredInfoV0::new(poll(), block(7)).into()),
            yes_voting_power: 5,
            no_voting_power: 1,
            abstain_voting_power: 4,
        };
        assert_eq!(
            vote_poll_state_json(Some(state)).expect("writes"),
            Some(json!({
                "votePoll": poll_json(),
                "status": "started",
                "startBlock": {"height": 7, "coreHeight": 42, "timeMs": 7_000, "epoch": 3},
                "yesVotingPower": 5,
                "noVotingPower": 1,
                "abstainVotingPower": 4,
            }))
        );
    }

    #[test]
    fn should_write_a_finished_poll_with_its_result() {
        let mut stored_info: YesNoVotePollStoredInfo =
            YesNoVotePollStoredInfoV0::new(poll(), block(7)).into();
        stored_info
            .finalize_vote_poll(30, 10, 8, 20, true, block(9))
            .expect("finalizes");
        let state = YesNoVotePollState {
            stored_info: Some(stored_info),
            ..Default::default()
        };
        assert_eq!(
            vote_poll_state_json(Some(state)).expect("writes"),
            Some(json!({
                "votePoll": poll_json(),
                "status": "finished",
                "result": {
                    "passed": true,
                    "yesVotingPower": 30,
                    "noVotingPower": 10,
                    "abstainVotingPower": 8,
                    "requiredVotingPower": 20,
                    "startBlock": {"height": 7, "coreHeight": 42, "timeMs": 7_000, "epoch": 3},
                    "finalizationBlock": {"height": 9, "coreHeight": 42, "timeMs": 9_000, "epoch": 3},
                },
                "yesVotingPower": 0,
                "noVotingPower": 0,
                "abstainVotingPower": 0,
            }))
        );
    }

    #[test]
    fn should_write_a_poll_never_opened_as_absent() {
        assert_eq!(vote_poll_state_json(None).expect("writes"), None);
        assert_eq!(
            vote_poll_state_json(Some(YesNoVotePollState::default())).expect("writes"),
            None
        );
    }

    #[test]
    fn should_write_a_masternode_answer() {
        for (vote_choice, name) in [
            (YesNoAbstainVoteChoice::Yes, "yes"),
            (YesNoAbstainVoteChoice::No, "no"),
            (YesNoAbstainVoteChoice::Abstain, "abstain"),
        ] {
            assert_eq!(
                masternode_vote_json(Some(MasternodeYesNoVote {
                    vote_choice,
                    times_voted: 2,
                })),
                Some(json!({"choice": name, "timesVoted": 2}))
            );
        }
        assert_eq!(masternode_vote_json(None), None);
    }
}
