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
    pub const SUPERMAJORITY_NUMERATOR: &str = "supermajorityNumerator";
    pub const SUPERMAJORITY_DENOMINATOR: &str = "supermajorityDenominator";
    pub const MINIMUM_VOTING_POWER: &str = "minimumVotingPower";
    pub const MINIMUM_SHARE: &str = "minimumShare";
    pub const MINIMUM_SHARE_NUMERATOR: &str = "minimumShare.numerator";
    pub const MINIMUM_SHARE_DENOMINATOR: &str = "minimumShare.denominator";
    pub const MINIMUM_SHARE_ROUNDING: &str = "minimumShare.rounding";
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

    /// Version 0: the threshold is `supermajorityNumerator / supermajorityDenominator`, and the
    /// minimum is `minimumVotingPower` or `minimumShare`, exactly one of them, each held to
    /// [`YesNoVotePoll::validate_parameters`].
    fn from_proposal_v0(
        document_id: Identifier,
        properties: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<Self> {
        let refused = |reason: String| {
            ConsensusValidationResult::new_with_error(
                InvalidDecisionProposalRuleError::new(document_id, reason).into(),
            )
        };
        let read = || -> Result<Self, String> {
            let integer = |path: &str| {
                properties
                    .get_optional_integer_at_path::<u64>(path)
                    .map_err(|error| format!("`{path}` is not a whole number: {error}"))
            };
            let narrow = |path: &str, value: Option<u64>| -> Result<Option<u8>, String> {
                value
                    .map(|value| {
                        u8::try_from(value)
                            .map_err(|_| format!("`{path}` is at most 255, got {value}"))
                    })
                    .transpose()
            };
            let required = |path: &str, value: Option<u8>| {
                value.ok_or_else(|| format!("`{path}` is required"))
            };
            let supermajority_numerator = required(
                properties::SUPERMAJORITY_NUMERATOR,
                narrow(
                    properties::SUPERMAJORITY_NUMERATOR,
                    integer(properties::SUPERMAJORITY_NUMERATOR)?,
                )?,
            )?;
            let supermajority_denominator = required(
                properties::SUPERMAJORITY_DENOMINATOR,
                narrow(
                    properties::SUPERMAJORITY_DENOMINATOR,
                    integer(properties::SUPERMAJORITY_DENOMINATOR)?,
                )?,
            )?;
            let minimum_voting_power = integer(properties::MINIMUM_VOTING_POWER)?;
            let minimum_share = properties
                .get(properties::MINIMUM_SHARE)
                .filter(|share| !share.is_null());
            let minimum_voting_power = match (minimum_voting_power, minimum_share) {
                (Some(voting_power), None) => YesNoMinimumVotingPower::Absolute {
                    voting_power: u32::try_from(voting_power).map_err(|_| {
                        format!(
                            "`{}` is at most {}, got {voting_power}",
                            properties::MINIMUM_VOTING_POWER,
                            u32::MAX
                        )
                    })?,
                },
                (None, Some(_)) => YesNoMinimumVotingPower::FractionOfTotal {
                    numerator: required(
                        properties::MINIMUM_SHARE_NUMERATOR,
                        narrow(
                            properties::MINIMUM_SHARE_NUMERATOR,
                            integer(properties::MINIMUM_SHARE_NUMERATOR)?,
                        )?,
                    )?,
                    denominator: required(
                        properties::MINIMUM_SHARE_DENOMINATOR,
                        narrow(
                            properties::MINIMUM_SHARE_DENOMINATOR,
                            integer(properties::MINIMUM_SHARE_DENOMINATOR)?,
                        )?,
                    )?,
                    rounding: rounding_from_name(
                        properties
                            .get_str_at_path(properties::MINIMUM_SHARE_ROUNDING)
                            .map_err(|_| {
                                format!("`{}` is required", properties::MINIMUM_SHARE_ROUNDING)
                            })?,
                    )?,
                },
                (Some(_), Some(_)) => {
                    return Err(format!(
                        "set `{}` or `{}`, not both",
                        properties::MINIMUM_VOTING_POWER,
                        properties::MINIMUM_SHARE
                    ))
                }
                (None, None) => {
                    return Err(format!(
                        "set `{}` or `{}`",
                        properties::MINIMUM_VOTING_POWER,
                        properties::MINIMUM_SHARE
                    ))
                }
            };
            let end_date = integer(properties::END_DATE)?
                .ok_or_else(|| format!("`{}` is required", properties::END_DATE))?;
            let vote_poll = YesNoVotePoll {
                purpose: YesNoVotePollPurpose::DecisionProposal { document_id },
                supermajority_numerator,
                supermajority_denominator,
                minimum_voting_power,
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
            Err(reason) => refused(reason),
        }
    }
}

/// The rounding a proposal's `minimumShare.rounding` names, as `VotingPowerRounding` names it in
/// JSON.
fn rounding_from_name(name: &str) -> Result<VotingPowerRounding, String> {
    match name {
        "down" => Ok(VotingPowerRounding::Down),
        "up" => Ok(VotingPowerRounding::Up),
        "downPlusOne" => Ok(VotingPowerRounding::DownPlusOne),
        "upMinusOne" => Ok(VotingPowerRounding::UpMinusOne),
        other => Err(format!(
            "`{}` is one of down, up, downPlusOne and upMinusOne, got {other}",
            properties::MINIMUM_SHARE_ROUNDING
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    fn proposal(minimum: (&str, Value)) -> BTreeMap<String, Value> {
        BTreeMap::from([
            (
                "title".to_string(),
                Value::Text("Raise the limit?".to_string()),
            ),
            (
                properties::SUPERMAJORITY_NUMERATOR.to_string(),
                Value::U8(2),
            ),
            (
                properties::SUPERMAJORITY_DENOMINATOR.to_string(),
                Value::U8(3),
            ),
            (minimum.0.to_string(), minimum.1),
            (properties::END_DATE.to_string(), Value::U64(5_000)),
        ])
    }

    fn share(numerator: u8, denominator: u8, rounding: &str) -> Value {
        Value::Map(vec![
            (Value::Text("numerator".to_string()), Value::U8(numerator)),
            (
                Value::Text("denominator".to_string()),
                Value::U8(denominator),
            ),
            (
                Value::Text("rounding".to_string()),
                Value::Text(rounding.to_string()),
            ),
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
            &proposal((properties::MINIMUM_VOTING_POWER, Value::U32(400))),
            platform_version,
        )
        .expect("active at 15");
        assert_eq!(
            result.into_data().expect("a valid rule"),
            DecisionProposalVotePoll {
                vote_poll: YesNoVotePoll {
                    purpose: YesNoVotePollPurpose::DecisionProposal { document_id },
                    supermajority_numerator: 2,
                    supermajority_denominator: 3,
                    minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 400 },
                },
                end_date: 5_000,
            }
        );
        let result = DecisionProposalVotePoll::from_proposal(
            document_id,
            &proposal((properties::MINIMUM_SHARE, share(1, 3, "downPlusOne"))),
            platform_version,
        )
        .expect("active at 15");
        assert_eq!(
            result
                .into_data()
                .expect("a valid rule")
                .vote_poll
                .minimum_voting_power,
            YesNoMinimumVotingPower::FractionOfTotal {
                numerator: 1,
                denominator: 3,
                rounding: VotingPowerRounding::DownPlusOne,
            }
        );
    }

    #[test]
    fn should_refuse_a_rule_outside_the_limits() {
        let platform_version = PlatformVersion::latest();
        let document_id = Identifier::new([7; 32]);
        let refused = |properties: BTreeMap<String, Value>| {
            refusal(
                DecisionProposalVotePoll::from_proposal(document_id, &properties, platform_version)
                    .expect("active at 15"),
            )
        };

        // Neither minimum, and both.
        let mut neither = proposal((properties::MINIMUM_VOTING_POWER, Value::U32(1)));
        neither.remove(properties::MINIMUM_VOTING_POWER);
        assert!(refused(neither).contains("set `minimumVotingPower` or `minimumShare`"));
        let mut both = proposal((properties::MINIMUM_VOTING_POWER, Value::U32(1)));
        both.insert(properties::MINIMUM_SHARE.to_string(), share(1, 3, "down"));
        assert!(refused(both).contains("not both"));

        // A threshold below a simple majority.
        let mut below_majority = proposal((properties::MINIMUM_VOTING_POWER, Value::U32(1)));
        below_majority.insert(
            properties::SUPERMAJORITY_NUMERATOR.to_string(),
            Value::U8(127),
        );
        below_majority.insert(
            properties::SUPERMAJORITY_DENOMINATOR.to_string(),
            Value::U8(255),
        );
        assert!(refused(below_majority).contains("at least 50%"));

        // A minimum above its cap: 2,000 voting power, half the network.
        let max_voting_power = platform_version
            .system_limits
            .max_yes_no_vote_poll_minimum_voting_power;
        assert!(DecisionProposalVotePoll::from_proposal(
            document_id,
            &proposal((
                properties::MINIMUM_VOTING_POWER,
                Value::U32(max_voting_power)
            )),
            platform_version,
        )
        .expect("active at 15")
        .is_valid());
        refused(proposal((
            properties::MINIMUM_VOTING_POWER,
            Value::U32(max_voting_power + 1),
        )));
        assert!(DecisionProposalVotePoll::from_proposal(
            document_id,
            &proposal((properties::MINIMUM_SHARE, share(1, 2, "up"))),
            platform_version,
        )
        .expect("active at 15")
        .is_valid());
        refused(proposal((properties::MINIMUM_SHARE, share(51, 100, "up"))));
    }

    #[test]
    fn should_not_be_active_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        assert!(matches!(
            DecisionProposalVotePoll::from_proposal(
                Identifier::new([7; 32]),
                &proposal((properties::MINIMUM_VOTING_POWER, Value::U32(1))),
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
        assert_eq!(
            properties::SUPERMAJORITY_NUMERATOR,
            proposal::properties::SUPERMAJORITY_NUMERATOR
        );
        assert_eq!(
            properties::SUPERMAJORITY_DENOMINATOR,
            proposal::properties::SUPERMAJORITY_DENOMINATOR
        );
        assert_eq!(
            properties::MINIMUM_VOTING_POWER,
            proposal::properties::MINIMUM_VOTING_POWER
        );
        assert_eq!(
            properties::MINIMUM_SHARE,
            proposal::properties::MINIMUM_SHARE
        );
        assert_eq!(properties::END_DATE, proposal::properties::END_DATE);
        for name in proposal::properties::minimum_share::ROUNDINGS {
            assert!(rounding_from_name(name).is_ok(), "{name} is a rounding");
        }
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
            properties: proposal((properties::MINIMUM_VOTING_POWER, Value::U32(1))),
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
