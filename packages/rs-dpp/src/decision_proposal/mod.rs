//! Decision proposals (protocol version 15).
//!
//! A decision proposal is a yes/no question any identity puts to the masternodes: a `proposal`
//! document of the decision proposals system contract. Creating one opens a yes/no vote poll
//! ([`YesNoVotePoll`]) whose purpose is the proposal ([`YesNoVotePollPurpose::DecisionProposal`])
//! and whose rule is the one the proposal states, and funds the poll's votes with the decision
//! proposal fund, which the proposer pays. Proposals are signaling only: nothing acts on a
//! result, which stays in the poll's stored info.
//!
//! [`DecisionProposalVotePoll::from_proposal`] reads the poll a proposal opens from its id and
//! properties. Consensus uses it to open the poll, and clients to name the poll they query.

use crate::balances::credits::Credits;
use crate::consensus::basic::decision_proposal::InvalidDecisionProposalRuleError;
use crate::prelude::TimestampMillis;
use crate::validation::ConsensusValidationResult;
use crate::voting::vote_polls::yes_no_vote_poll::{
    VotingPowerRounding, YesNoMinimumVotingPower, YesNoVotePoll, YesNoVotePollPurpose,
};
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::{Identifier, IdentifierBytes32, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// The id of the decision proposals system contract, `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`.
///
/// Spelled here so that consensus code can name the contract without the optional contract
/// crates; the crate's own constant is pinned to this one by a test.
pub const DECISION_PROPOSALS_CONTRACT_ID: Identifier = Identifier(IdentifierBytes32([
    40, 81, 10, 2, 166, 107, 62, 218, 6, 184, 111, 202, 230, 218, 170, 157, 181, 119, 102, 211,
    195, 114, 92, 200, 90, 88, 205, 30, 174, 49, 203, 200,
]));

/// The name of the proposal document type.
pub const PROPOSAL_DOCUMENT_TYPE_NAME: &str = "proposal";

/// The name a proposal's create states its fund under, in its prefunded voting balance: the
/// proposal's own type. A contested create names its contested index there instead.
pub const DECISION_PROPOSAL_FUND_NAME: &str = PROPOSAL_DOCUMENT_TYPE_NAME;

/// The properties of a proposal the poll is read from.
pub mod properties {
    /// The proposal passes when yes is strictly more than this percent of yes plus no.
    pub const PASS_PERCENT: &str = "passPercent";
    /// The least yes plus no voting power the proposal needs, as a percent of the masternode
    /// list's total voting power when the poll closes, rounded up.
    pub const MINIMUM_TURNOUT_PERCENT: &str = "minimumTurnoutPercent";
    pub const END_DATE: &str = "endDate";
}

/// Whether a document of `document_type_name` in the contract `contract_id` is a decision
/// proposal: a `proposal` of the decision proposals contract. Its create opens a yes/no poll and
/// states the decision proposal fund instead of a contest's.
pub fn is_decision_proposal(contract_id: &Identifier, document_type_name: &str) -> bool {
    *contract_id == DECISION_PROPOSALS_CONTRACT_ID
        && document_type_name == PROPOSAL_DOCUMENT_TYPE_NAME
}

/// The fund a decision proposal's create must state, exactly: the fee version's
/// `decision_proposal_fund_required_amount`. It prefunds the votes on the poll; what is left
/// when the poll ends goes to processing fees.
pub fn decision_proposal_required_fund(platform_version: &PlatformVersion) -> Credits {
    platform_version
        .fee_version
        .vote_resolution_fund_fees
        .decision_proposal_fund_required_amount
}

/// The yes/no poll a decision proposal opens and when it closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionProposalVotePoll {
    /// The poll: the proposal as its purpose, the proposal's rule.
    pub vote_poll: YesNoVotePoll,
    /// The proposal's `endDate`: when the poll closes.
    pub end_date: TimestampMillis,
}

impl DecisionProposalVotePoll {
    /// The poll the proposal with `document_id` and `properties` opens, by the rule the
    /// platform version selects (`dpp.voting_versions.decision_proposal_vote_poll`).
    ///
    /// The properties are expected to have passed the proposal's schema. A rule that makes no
    /// poll the system limits allow is refused with [`InvalidDecisionProposalRuleError`]; the
    /// end date is judged against the opening block by the caller
    /// ([`YesNoVotePoll::end_date_window`]).
    ///
    /// # Parameters
    /// - `document_id`: the proposal's id, the poll's purpose.
    /// - `properties`: the proposal's properties.
    /// - `platform_version`: the platform version, whose system limits bound the rule.
    ///
    /// # Returns
    /// The poll and its end date, or the consensus error refusing the rule.
    pub fn from_proposal(
        document_id: Identifier,
        properties: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .decision_proposal_vote_poll
        {
            Some(0) => Ok(Self::from_proposal_v0(
                document_id,
                properties,
                platform_version,
            )),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "DecisionProposalVotePoll::from_proposal".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "DecisionProposalVotePoll::from_proposal is not active before protocol version 15"
                    .to_string(),
            )),
        }
    }

    /// Version 0: the poll passes when yes is strictly more than `passPercent` percent of yes
    /// plus no (a supermajority of `passPercent / 100`), with yes plus no at least
    /// `minimumTurnoutPercent` percent of the masternode list's total voting power when it
    /// closes, rounded up. The rule is held to [`YesNoVotePoll::validate_parameters`]; the
    /// contract's schema holds the percentages to 50 to 80 and 0 to 50.
    fn from_proposal_v0(
        document_id: Identifier,
        properties: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<Self> {
        let read = || -> Result<Self, String> {
            let percent = |path: &str| -> Result<u8, String> {
                let value = properties
                    .get_optional_integer_at_path::<u64>(path)
                    .map_err(|error| format!("`{path}` is not a whole number: {error}"))?
                    .ok_or_else(|| format!("`{path}` is required"))?;
                u8::try_from(value).map_err(|_| format!("`{path}` is at most 255, got {value}"))
            };
            let pass_percent = percent(properties::PASS_PERCENT)?;
            let minimum_turnout_percent = percent(properties::MINIMUM_TURNOUT_PERCENT)?;
            let end_date = properties
                .get_optional_integer_at_path::<u64>(properties::END_DATE)
                .map_err(|error| {
                    format!("`{}` is not a whole number: {error}", properties::END_DATE)
                })?
                .ok_or_else(|| format!("`{}` is required", properties::END_DATE))?;
            let vote_poll = YesNoVotePoll {
                purpose: YesNoVotePollPurpose::DecisionProposal { document_id },
                supermajority_numerator: pass_percent,
                supermajority_denominator: 100,
                minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                    numerator: minimum_turnout_percent,
                    denominator: 100,
                    rounding: VotingPowerRounding::Up,
                },
            };
            vote_poll
                .validate_parameters(platform_version)
                .map_err(|error| match error {
                    ProtocolError::VoteError(reason) => reason,
                    other => other.to_string(),
                })?;
            Ok(Self {
                vote_poll,
                end_date,
            })
        };
        match read() {
            Ok(decision_proposal_vote_poll) => {
                ConsensusValidationResult::new_with_data(decision_proposal_vote_poll)
            }
            Err(reason) => ConsensusValidationResult::new_with_error(
                InvalidDecisionProposalRuleError::new(document_id, reason).into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    fn proposal(pass_percent: u8, minimum_turnout_percent: u8) -> BTreeMap<String, Value> {
        BTreeMap::from([
            (
                "title".to_string(),
                Value::Text("Raise the limit?".to_string()),
            ),
            (
                properties::PASS_PERCENT.to_string(),
                Value::U8(pass_percent),
            ),
            (
                properties::MINIMUM_TURNOUT_PERCENT.to_string(),
                Value::U8(minimum_turnout_percent),
            ),
            (properties::END_DATE.to_string(), Value::U64(5_000)),
        ])
    }

    fn refusal(result: ConsensusValidationResult<DecisionProposalVotePoll>) -> String {
        match result.errors.as_slice() {
            [ConsensusError::BasicError(BasicError::InvalidDecisionProposalRuleError(error))] => {
                error.reason().to_string()
            }
            other => panic!("expected one invalid rule error, got {other:?}"),
        }
    }

    #[test]
    fn should_read_the_poll_a_proposal_opens() {
        let platform_version = PlatformVersion::latest();
        let document_id = Identifier::new([7; 32]);
        let result = DecisionProposalVotePoll::from_proposal(
            document_id,
            &proposal(60, 10),
            platform_version,
        )
        .expect("active at 15");
        assert_eq!(
            result.into_data().expect("a valid rule"),
            DecisionProposalVotePoll {
                vote_poll: YesNoVotePoll {
                    purpose: YesNoVotePollPurpose::DecisionProposal { document_id },
                    supermajority_numerator: 60,
                    supermajority_denominator: 100,
                    minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                        numerator: 10,
                        denominator: 100,
                        rounding: VotingPowerRounding::Up,
                    },
                },
                end_date: 5_000,
            }
        );
    }

    /// Yes must be strictly more than the percent: at 50, a tie fails and one more yes passes.
    #[test]
    fn should_pass_only_strictly_above_the_pass_percent() {
        let platform_version = PlatformVersion::latest();
        let vote_poll = DecisionProposalVotePoll::from_proposal(
            Identifier::new([7; 32]),
            &proposal(50, 0),
            platform_version,
        )
        .expect("active at 15")
        .into_data()
        .expect("a valid rule")
        .vote_poll;
        let passes = |yes, no| {
            vote_poll
                .passes(yes, no, 1_000, platform_version)
                .expect("15")
        };
        assert!(!passes(5, 5));
        assert!(passes(6, 5));
        // A turnout minimum of 10% of 95 rounds up to 10
        let vote_poll = DecisionProposalVotePoll::from_proposal(
            Identifier::new([7; 32]),
            &proposal(50, 10),
            platform_version,
        )
        .expect("active at 15")
        .into_data()
        .expect("a valid rule")
        .vote_poll;
        assert_eq!(
            vote_poll
                .required_voting_power(95, platform_version)
                .expect("15"),
            10
        );
    }

    #[test]
    fn should_refuse_a_rule_outside_the_limits() {
        let platform_version = PlatformVersion::latest();
        let document_id = Identifier::new([7; 32]);
        let read = |properties: BTreeMap<String, Value>| {
            DecisionProposalVotePoll::from_proposal(document_id, &properties, platform_version)
                .expect("active at 15")
        };
        // The schema holds a proposal to 50-80 and 0-50; the poll's own rules still refuse what
        // no poll may run with, whatever a schema allows.
        assert!(refusal(read(proposal(49, 0))).contains("at least 50%"));
        assert!(refusal(read(proposal(100, 0))).contains("below its denominator"));
        assert!(refusal(read(proposal(60, 51))).contains("at most 50%"));
        assert!(read(proposal(50, 50)).is_valid());
        let mut no_turnout = proposal(60, 0);
        no_turnout.remove(properties::MINIMUM_TURNOUT_PERCENT);
        assert!(refusal(read(no_turnout)).contains("is required"));
    }

    #[test]
    fn should_not_be_active_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        assert!(matches!(
            DecisionProposalVotePoll::from_proposal(
                Identifier::new([7; 32]),
                &proposal(60, 10),
                platform_version,
            ),
            Err(ProtocolError::UnknownVersionError(_))
        ));
    }

    #[test]
    fn should_recognize_only_the_proposal_of_the_decision_proposals_contract() {
        assert!(is_decision_proposal(
            &DECISION_PROPOSALS_CONTRACT_ID,
            PROPOSAL_DOCUMENT_TYPE_NAME
        ));
        assert!(!is_decision_proposal(
            &DECISION_PROPOSALS_CONTRACT_ID,
            "other"
        ));
        assert!(!is_decision_proposal(
            &Identifier::new([1; 32]),
            PROPOSAL_DOCUMENT_TYPE_NAME
        ));
    }

    #[cfg(feature = "decision-proposals-contract")]
    #[test]
    fn should_name_the_contract_as_its_crate_does() {
        use data_contracts::decision_proposals_contract;
        use decision_proposals_contract::v1::document_types::proposal;

        assert_eq!(
            DECISION_PROPOSALS_CONTRACT_ID,
            decision_proposals_contract::ID
        );
        assert_eq!(PROPOSAL_DOCUMENT_TYPE_NAME, proposal::NAME);
        assert_eq!(properties::PASS_PERCENT, proposal::properties::PASS_PERCENT);
        assert_eq!(
            properties::MINIMUM_TURNOUT_PERCENT,
            proposal::properties::MINIMUM_TURNOUT_PERCENT
        );
        assert_eq!(properties::END_DATE, proposal::properties::END_DATE);
    }

    /// A proposal's title, description and url freeze a day after it is created, or after a
    /// fifth of the time it runs if that is shorter (which the one-week minimum makes it never
    /// is today); its rule never changes.
    #[cfg(feature = "decision-proposals-contract")]
    #[test]
    fn should_freeze_the_text_of_a_proposal_after_a_day_or_a_fifth_of_its_run() {
        use crate::data_contract::accessors::v0::DataContractV0Getters;
        use crate::data_contract::document_type::accessors::{
            DocumentTypeV0Getters, DocumentTypeV2Getters,
        };
        use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
        use crate::system_data_contracts::{load_system_data_contract, SystemDataContract};

        const DAY_MS: u64 = 86_400_000;
        let platform_version = PlatformVersion::latest();
        let contract =
            load_system_data_contract(SystemDataContract::DecisionProposals, platform_version)
                .expect("the contract loads at 15");
        let document_type = contract
            .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("the proposal type");
        let created_at = 1_000;
        let frozen = |property: &str, run: u64, edited_after: u64| {
            document_type.immutable_field_conditions()[property]
                .holds(
                    &Value::Map(vec![(
                        Value::Text(properties::END_DATE.to_string()),
                        Value::U64(created_at + run),
                    )]),
                    &DocumentSystemValues {
                        created_at: Some(created_at),
                        updated_at: Some(created_at + edited_after),
                        ..Default::default()
                    },
                )
                .expect("the condition evaluates")
        };
        for property in ["title", "description", "url"] {
            // Two weeks: a fifth is 2.8 days, so a day comes first
            assert!(
                !frozen(property, 14 * DAY_MS, DAY_MS),
                "{property} at a day"
            );
            assert!(
                frozen(property, 14 * DAY_MS, DAY_MS + 1),
                "{property} after a day"
            );
            // Two days: a fifth, 9.6 hours, comes first
            let fifth = 2 * DAY_MS / 5;
            assert!(
                !frozen(property, 2 * DAY_MS, fifth),
                "{property} at a fifth"
            );
            assert!(
                frozen(property, 2 * DAY_MS, fifth + 1),
                "{property} after a fifth"
            );
        }
        for property in [
            properties::PASS_PERCENT,
            properties::MINIMUM_TURNOUT_PERCENT,
            properties::END_DATE,
        ] {
            assert!(
                document_type.immutable_fields().contains(property)
                    && !document_type
                        .immutable_field_conditions()
                        .contains_key(property),
                "{property} never changes"
            );
        }
        assert!(document_type.documents_mutable());
        assert!(document_type.documents_keep_history());
    }

    /// A create built from a proposal document, as the SDKs build one, states the decision
    /// proposal fund under the proposal's name.
    #[cfg(feature = "decision-proposals-contract")]
    #[test]
    fn should_state_the_decision_proposal_fund_on_a_proposal_create() {
        use crate::data_contract::accessors::v0::DataContractV0Getters;
        use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
        use crate::document::{Document, DocumentV0};
        use crate::system_data_contracts::{load_system_data_contract, SystemDataContract};

        let platform_version = PlatformVersion::latest();
        let contract =
            load_system_data_contract(SystemDataContract::DecisionProposals, platform_version)
                .expect("the contract loads at 15");
        let document_type = contract
            .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("the proposal type");
        let document: Document = DocumentV0 {
            properties: proposal(60, 10),
            ..Default::default()
        }
        .into();
        assert_eq!(
            document_type
                .prefunded_voting_balance_for_document(&document, platform_version)
                .expect("active"),
            Some((DECISION_PROPOSAL_FUND_NAME.to_string(), 50_000_000_000))
        );
    }
}
