use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::{DocumentCreationNotAllowedError, InvalidDocumentTypeError};
use dpp::consensus::state::document::document_contest_index_mismatch_error::DocumentContestIndexMismatchError;
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::consensus::state::document::document_contest_not_required_error::DocumentContestNotRequiredError;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::identifier::Identifier;
use dpp::validation::{SimpleConsensusValidationResult};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStructureValidationV1 {
    fn validate_structure_v1(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentCreateTransitionActionStructureValidationV1 for DocumentCreateTransitionAction {
    fn validate_structure_v1(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        // Make sure that the document type is defined in the contract
        let document_type_name = self.base().document_type_name();

        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        // Don't do the following validation on testnet before epoch 2080
        // As state transitions already happened that would break this validation
        // We want to keep both if-s for better readability
        #[allow(clippy::collapsible_if)]
        if !(network == Network::Testnet && block_info.epoch.index < 2080) {
            let expected_vote_poll = document_type
                .contested_vote_poll_for_document_properties(self.data(), platform_version)?;

            match (expected_vote_poll, self.prefunded_voting_balance()) {
                (
                    Some(VotePoll::ContestedDocumentResourceVotePoll(expected)),
                    Some((provided, paid_amount)),
                ) => {
                    let expected_amount = platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_vote_resolution_fund_required_amount;
                    if expected_amount != *paid_amount {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DocumentContestNotPaidForError::new(
                                self.base().id(),
                                expected_amount,
                                *paid_amount,
                            )
                            .into(),
                        ));
                    }

                    // -->> Introduced in V1 <<--
                    // The index name in the prefunded voting balance is chosen by the submitter,
                    // and it is what keys the vote poll, its stored info and its prefunded
                    // specialized balance. Document insertion however always uses the contested
                    // index of the document type. Without this check those two can name different
                    // indexes, so the contest would be funded, resolved and cleaned up under a
                    // vote poll that does not describe the contest that was actually created.
                    let provided: ContestedDocumentResourceVotePoll = provided.into();
                    if provided != expected {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DocumentContestIndexMismatchError::new(
                                self.base().id(),
                                expected.index_name,
                                provided.index_name,
                            )
                            .into(),
                        ));
                    }
                    // -->> End Introduced in V1 <<--
                }
                (Some(_), None) => {
                    let expected_amount = platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_vote_resolution_fund_required_amount;
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentContestNotPaidForError::new(self.base().id(), expected_amount, 0)
                            .into(),
                    ));
                }
                // -->> Introduced in V1 <<--
                // A document that resolves to no contested index must not open a contest.
                // Otherwise a document that is not a contested resource is stored as a
                // contender and only becomes registered if it wins a masternode vote.
                (None, Some((provided, _))) => {
                    let provided: ContestedDocumentResourceVotePoll = provided.into();
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentContestNotRequiredError::new(self.base().id(), provided.index_name)
                            .into(),
                    ));
                }
                // -->> End Introduced in V1 <<--
                (None, None) => {}
            }
        }

        match document_type.creation_restriction_mode() {
            CreationRestrictionMode::NoRestrictions => {}
            CreationRestrictionMode::OwnerOnly => {
                if owner_id != data_contract.owner_id() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentCreationNotAllowedError::new(
                            self.base().data_contract_id(),
                            document_type_name.clone(),
                            document_type.creation_restriction_mode(),
                        )
                        .into(),
                    ));
                }
            }
            CreationRestrictionMode::NoCreationAllowed => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentCreationNotAllowedError::new(
                        self.base().data_contract_id(),
                        document_type_name.clone(),
                        document_type.creation_restriction_mode(),
                    )
                    .into(),
                ));
            }
        }
        // Validate user defined properties

        data_contract
            .validate_document_properties(document_type_name, self.data().into(), platform_version)
            .map_err(Error::Protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::DocumentCreateTransitionActionValidation;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::fee::Credits;
    use dpp::platform_value::Value;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;
    use drive::util::object_size_info::DataContractOwnedResolvedInfo;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    /// The contested index of the DPNS `domain` document type.
    const CONTESTED_INDEX_NAME: &str = "parentNameAndLabel";
    /// A second, non-contested index on the same document type.
    const OTHER_INDEX_NAME: &str = "identityId";

    /// `parentNameAndLabel` is contested for labels matching
    /// `^[a-zA-Z01-]{3,19}$`, so this one opens a contest.
    const CONTESTED_LABEL: &str = "quantum";
    /// Twenty characters: too long to be contested.
    const NON_CONTESTED_LABEL: &str = "quantumcomputingnow1";

    /// The protocol version whose structure validation is v0, i.e. the last one
    /// that accepts a prefunded voting balance naming any index at all.
    const PROTOCOL_VERSION_BEFORE_CROSS_CHECK: u32 = 13;

    fn required_amount(platform_version: &PlatformVersion) -> Credits {
        platform_version
            .fee_version
            .vote_resolution_fund_fees
            .contested_document_vote_resolution_fund_required_amount
    }

    fn domain_properties(label: &str) -> BTreeMap<String, Value> {
        BTreeMap::from([
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedLabel".to_string(),
                Value::Text(label.to_string()),
            ),
            ("label".to_string(), Value::Text(label.to_string())),
        ])
    }

    /// Builds the action the transformer would build for a `domain` document
    /// with `label`, whose prefunded voting balance names `prefunded_index`.
    fn create_action(
        label: &str,
        prefunded_index: Option<&str>,
        platform_version: &PlatformVersion,
    ) -> DocumentCreateTransitionAction {
        let contract_fetch_info = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
            platform_version.protocol_version,
        ));
        let data = domain_properties(label);

        let prefunded_voting_balance = prefunded_index.map(|index_name| {
            // Mirrors the transformer: the index is taken as given and its
            // values are extracted from the document data.
            let index_values = contract_fetch_info
                .contract
                .document_type_for_name("domain")
                .expect("expected the domain document type")
                .indexes()
                .get(index_name)
                .expect("expected the index to exist on the document type")
                .extract_values(&data);

            let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::DataContractFetchInfo(
                    contract_fetch_info.clone(),
                ),
                document_type_name: "domain".to_string(),
                index_name: index_name.to_string(),
                index_values,
            };

            (vote_poll, required_amount(platform_version))
        });

        DocumentCreateTransitionAction::V0(DocumentCreateTransitionActionV0 {
            base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                id: Identifier::from([0xAA; 32]),
                identity_contract_nonce: 1,
                document_type_name: "domain".to_string(),
                data_contract: contract_fetch_info,
                token_cost: None,
                gas_fees_paid_by: GasFeesPaidBy::default(),
            }),
            block_info: BlockInfo::default(),
            data,
            prefunded_voting_balance,
            current_store_contest_info: None,
            should_store_contest_info: None,
        })
    }

    fn validate(
        action: &DocumentCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Vec<ConsensusError> {
        action
            .validate_structure(
                Identifier::from([0xBB; 32]),
                &BlockInfo::default(),
                Network::Mainnet,
                platform_version,
            )
            .expect("expected structure validation to run")
            .errors
    }

    fn contest_errors(errors: &[ConsensusError]) -> Vec<&StateError> {
        errors
            .iter()
            .filter_map(|error| match error {
                ConsensusError::StateError(
                    state_error @ (StateError::DocumentContestIndexMismatchError(_)
                    | StateError::DocumentContestNotRequiredError(_)
                    | StateError::DocumentContestNotPaidForError(_)),
                ) => Some(state_error),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn should_reject_a_prefunded_voting_balance_naming_another_index() {
        let platform_version = PlatformVersion::latest();

        let action = create_action(CONTESTED_LABEL, Some(OTHER_INDEX_NAME), platform_version);

        let errors = validate(&action, platform_version);

        let [ConsensusError::StateError(StateError::DocumentContestIndexMismatchError(error))] =
            errors.as_slice()
        else {
            panic!("expected a single DocumentContestIndexMismatchError, got {errors:?}");
        };
        assert_eq!(error.expected_index_name(), CONTESTED_INDEX_NAME);
        assert_eq!(error.provided_index_name(), OTHER_INDEX_NAME);
    }

    #[test]
    fn should_reject_a_prefunded_voting_balance_on_a_non_contested_document() {
        let platform_version = PlatformVersion::latest();

        let action = create_action(
            NON_CONTESTED_LABEL,
            Some(CONTESTED_INDEX_NAME),
            platform_version,
        );

        let errors = validate(&action, platform_version);

        let [ConsensusError::StateError(StateError::DocumentContestNotRequiredError(error))] =
            errors.as_slice()
        else {
            panic!("expected a single DocumentContestNotRequiredError, got {errors:?}");
        };
        assert_eq!(error.provided_index_name(), CONTESTED_INDEX_NAME);
    }

    #[test]
    fn should_still_reject_a_contested_document_that_was_not_paid_for() {
        let platform_version = PlatformVersion::latest();

        let action = create_action(CONTESTED_LABEL, None, platform_version);

        let errors = validate(&action, platform_version);

        assert!(
            matches!(
                contest_errors(&errors).as_slice(),
                [StateError::DocumentContestNotPaidForError(_)]
            ),
            "expected the contest to still be required to be paid for, got {errors:?}"
        );
    }

    #[test]
    fn should_accept_a_prefunded_voting_balance_naming_the_contested_index() {
        let platform_version = PlatformVersion::latest();

        let action = create_action(
            CONTESTED_LABEL,
            Some(CONTESTED_INDEX_NAME),
            platform_version,
        );

        // The property map above is deliberately minimal, so document property
        // validation is expected to complain; what must not appear is any
        // contest-related error.
        assert!(
            contest_errors(&validate(&action, platform_version)).is_empty(),
            "a prefunded voting balance naming the contested index must not be rejected as a contest error"
        );
    }

    /// The cross-check is consensus-relevant, so it must not apply before its
    /// protocol version: v0 accepted both shapes rejected above.
    #[test]
    fn should_not_cross_check_the_index_before_its_protocol_version() {
        let platform_version = PlatformVersion::get(PROTOCOL_VERSION_BEFORE_CROSS_CHECK)
            .expect("expected a platform version before the cross-check");

        assert_eq!(
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_structure_validation,
            0
        );

        let mismatched = create_action(CONTESTED_LABEL, Some(OTHER_INDEX_NAME), platform_version);
        assert!(
            contest_errors(&validate(&mismatched, platform_version)).is_empty(),
            "the index name was not cross-checked before the cross-check protocol version"
        );

        let not_contested = create_action(
            NON_CONTESTED_LABEL,
            Some(CONTESTED_INDEX_NAME),
            platform_version,
        );
        assert!(
            contest_errors(&validate(&not_contested, platform_version)).is_empty(),
            "an unnecessary prefunded voting balance was not rejected before the cross-check protocol version"
        );
    }
}
