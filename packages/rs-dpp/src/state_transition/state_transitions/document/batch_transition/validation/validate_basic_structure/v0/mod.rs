use crate::consensus::basic::document::{
    DocumentTransitionsAreAbsentError, DuplicateDocumentTransitionsWithIdsError,
    MaxDocumentsTransitionsExceededError, NonceOutOfBoundsError,
};
use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::consensus::basic::BasicError;
use crate::state_transition::batch_transition::batched_transition::DocumentIndexOnlyDeleteTransition;

use crate::identity::identity_nonce::MISSING_IDENTITY_REVISIONS_FILTER;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::validation::find_duplicates_by_id::find_duplicates_by_id;
use crate::state_transition::batch_transition::BatchTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use crate::consensus::basic::group::GroupActionNotAllowedOnTransitionError;
use crate::consensus::basic::token::{InvalidActionIdError, InvalidTokenIdError};
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use crate::state_transition::batch_transition::batched_transition::token_transition::{TokenTransition, TokenTransitionV0Methods};
use crate::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionTypeGetter;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_config_update_transition::validate_structure::TokenConfigUpdateTransitionStructureValidation;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::validate_structure::TokenDestroyFrozenFundsTransitionStructureValidation;
use crate::state_transition::batch_transition::token_emergency_action_transition::validate_structure::TokenEmergencyActionTransitionStructureValidation;
use crate::state_transition::batch_transition::token_freeze_transition::validate_structure::TokenFreezeTransitionStructureValidation;
use crate::state_transition::batch_transition::token_mint_transition::validate_structure::TokenMintTransitionStructureValidation;
use crate::state_transition::batch_transition::token_claim_transition::validate_structure::TokenClaimTransitionStructureValidation;
use crate::state_transition::batch_transition::token_direct_purchase_transition::validate_structure::TokenDirectPurchaseTransitionStructureValidation;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::validate_structure::TokenSetPriceForDirectPurchaseTransitionStructureValidation;
use crate::state_transition::batch_transition::token_transfer_transition::validate_structure::TokenTransferTransitionStructureValidation;
use crate::state_transition::batch_transition::token_unfreeze_transition::validate_structure::TokenUnfreezeTransitionStructureValidation;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::{DocumentTransition, DocumentTransitionV0Methods};
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_burn_transition::validate_structure::TokenBurnTransitionStructureValidation;
use crate::state_transition::StateTransitionOwned;

impl BatchTransition {
    #[inline(always)]
    pub(super) fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.transitions_are_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTransitionsAreAbsentError::new().into(),
            ));
        }

        let transitions_len = self.transitions_len();

        if transitions_len > u16::MAX as usize
            || transitions_len as u16
                > platform_version
                    .system_limits
                    .max_transitions_in_documents_batch
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MaxDocumentsTransitionsExceededError::new(
                    platform_version
                        .system_limits
                        .max_transitions_in_documents_batch,
                )
                .into(),
            ));
        }

        // Group transitions by contract ID
        let mut document_transitions_by_contracts: BTreeMap<Identifier, Vec<&DocumentTransition>> =
            BTreeMap::new();

        // Group transitions by contract ID
        let mut token_transitions: Vec<&TokenTransition> = vec![];

        self.transitions_iter()
            .for_each(|batch_transition| match batch_transition {
                BatchedTransitionRef::Document(document_transition) => {
                    let contract_identifier = document_transition.data_contract_id();

                    match document_transitions_by_contracts.entry(contract_identifier) {
                        Entry::Vacant(vacant) => {
                            vacant.insert(vec![document_transition]);
                        }
                        Entry::Occupied(mut identifiers) => {
                            identifiers.get_mut().push(document_transition);
                        }
                    };
                }
                BatchedTransitionRef::Token(token_transition) => {
                    token_transitions.push(token_transition)
                }
            });

        let mut result = SimpleConsensusValidationResult::default();

        for transitions in document_transitions_by_contracts.values() {
            for transition in transitions {
                // We need to make sure that the identity contract nonce is within the allowed bounds
                // This means that it is stored on 40 bits
                if transition.identity_contract_nonce() & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
                    result.add_error(BasicError::NonceOutOfBoundsError(
                        NonceOutOfBoundsError::new(transition.identity_contract_nonce()),
                    ));
                }

                // The indexOnlyDelete kind joined the wire at PV14. Old
                // software cannot decode it at all, so no historical block
                // can contain one — this check exists so that NEW software
                // agrees with old software while a pre-PV14 protocol
                // version is still active: without it, an indexOnly delete
                // submitted at PV13 would decode fine here while being
                // undecodable on 4.1 nodes.
                if let DocumentTransition::IndexOnlyDelete(index_only_delete) = transition {
                    let feature_version = match index_only_delete {
                        DocumentIndexOnlyDeleteTransition::V0(_) => 0,
                    };
                    match &platform_version
                        .dpp
                        .state_transition_serialization_versions
                        .document_index_only_delete_state_transition
                    {
                        None => {
                            // The kind does not exist at this protocol
                            // version; the empty supported range (min 1,
                            // max 0) states exactly that.
                            result.add_error(BasicError::UnsupportedVersionError(
                                UnsupportedVersionError::new(feature_version, 1, 0),
                            ));
                        }
                        Some(bounds) if !bounds.bounds.check_version(feature_version) => {
                            result.add_error(BasicError::UnsupportedVersionError(
                                UnsupportedVersionError::new(
                                    feature_version,
                                    bounds.bounds.min_version,
                                    bounds.bounds.max_version,
                                ),
                            ));
                        }
                        Some(_) => {}
                    }
                }
            }

            // Make sure we don't have duplicate transitions
            let duplicate_transitions = find_duplicates_by_id(transitions, platform_version)?;

            if !duplicate_transitions.is_empty() {
                let references: Vec<(String, [u8; 32])> = duplicate_transitions
                    .into_iter()
                    .map(|transition| {
                        Ok((
                            transition.base().document_type_name().clone(),
                            transition.base().id().to_buffer(),
                        ))
                    })
                    .collect::<Result<Vec<(String, [u8; 32])>, anyhow::Error>>()?;

                result.add_error(BasicError::DuplicateDocumentTransitionsWithIdsError(
                    DuplicateDocumentTransitionsWithIdsError::new(references),
                ));
            }
        }

        for transition in token_transitions {
            // We need to make sure that the identity contract nonce is within the allowed bounds
            // This means that it is stored on 40 bits
            if transition.identity_contract_nonce() & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
                result.add_error(BasicError::NonceOutOfBoundsError(
                    NonceOutOfBoundsError::new(transition.identity_contract_nonce()),
                ));
            }

            let transition_token_id = transition.base().token_id();
            let calculated_token_id = transition.base().calculate_token_id();

            // We need to verify that the token id is correct
            if transition_token_id != calculated_token_id {
                result.add_error(BasicError::InvalidTokenIdError(InvalidTokenIdError::new(
                    calculated_token_id,
                    transition_token_id,
                )));
            }

            let consensus_result = match transition {
                TokenTransition::Burn(burn_transition) => {
                    burn_transition.validate_structure(platform_version)?
                }
                TokenTransition::Mint(mint_transition) => {
                    mint_transition.validate_structure(platform_version)?
                }
                TokenTransition::Transfer(transfer_transition) => {
                    transfer_transition.validate_structure(self.owner_id(), platform_version)?
                }
                TokenTransition::Freeze(freeze_transition) => {
                    freeze_transition.validate_structure(platform_version)?
                }
                TokenTransition::Unfreeze(unfreeze_transition) => {
                    unfreeze_transition.validate_structure(platform_version)?
                }
                TokenTransition::DestroyFrozenFunds(destroy_frozen_funds_transition) => {
                    destroy_frozen_funds_transition.validate_structure(platform_version)?
                }
                TokenTransition::EmergencyAction(emergency_action_transition) => {
                    emergency_action_transition.validate_structure(platform_version)?
                }
                TokenTransition::ConfigUpdate(config_update_transition) => {
                    config_update_transition.validate_structure(platform_version)?
                }
                TokenTransition::Claim(release_transition) => {
                    release_transition.validate_structure(platform_version)?
                }
                TokenTransition::DirectPurchase(direct_purchase_transition) => {
                    direct_purchase_transition.validate_structure(platform_version)?
                }
                TokenTransition::SetPriceForDirectPurchase(
                    set_price_for_direct_purchase_transition,
                ) => {
                    set_price_for_direct_purchase_transition.validate_structure(platform_version)?
                }
            };

            if !consensus_result.is_valid() {
                return Ok(consensus_result);
            }

            // We need to verify that the action id given matches the expected action id
            // But only if we are the proposer
            if let Some(group_state_transition_info) = transition.base().using_group_info() {
                if group_state_transition_info.action_is_proposer {
                    if let Some(calculated_action_id) = transition
                        .calculate_action_id(self.owner_id(), platform_version)
                        .transpose()?
                    {
                        if group_state_transition_info.action_id != calculated_action_id {
                            result.add_error(BasicError::InvalidActionIdError(
                                InvalidActionIdError::new(
                                    calculated_action_id,
                                    group_state_transition_info.action_id,
                                ),
                            ));
                        }
                    } else {
                        result.add_error(BasicError::GroupActionNotAllowedOnTransitionError(
                            GroupActionNotAllowedOnTransitionError::new(
                                transition.action_type().to_string(),
                            ),
                        ));
                    }
                } else if !transition.can_calculate_action_id() {
                    result.add_error(BasicError::GroupActionNotAllowedOnTransitionError(
                        GroupActionNotAllowedOnTransitionError::new(
                            transition.action_type().to_string(),
                        ),
                    ));
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;
    use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::{
        BatchTransition, BatchTransitionV0, BatchTransitionV1,
    };
    use platform_value::BinaryData;
    use std::collections::BTreeMap;

    fn make_base(nonce: u64, type_name: &str) -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([1u8; 32]),
            identity_contract_nonce: nonce,
            document_type_name: type_name.to_string(),
            data_contract_id: Identifier::new([0xAA; 32]),
        })
    }

    fn make_create(nonce: u64) -> DocumentTransition {
        DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base: make_base(nonce, "test_doc"),
            entropy: [0u8; 32],
            data: BTreeMap::new(),
            prefunded_voting_balance: None,
        }))
    }

    fn make_delete(nonce: u64, id_byte: u8) -> DocumentTransition {
        DocumentTransition::Delete(DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([id_byte; 32]),
                identity_contract_nonce: nonce,
                document_type_name: "test_doc".to_string(),
                data_contract_id: Identifier::new([0xAA; 32]),
            }),
        }))
    }

    fn make_batch_v0(transitions: Vec<DocumentTransition>) -> BatchTransition {
        BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::new([0x01; 32]),
            transitions,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        })
    }

    fn make_batch_v1_empty() -> BatchTransition {
        BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::new([0x02; 32]),
            transitions: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        })
    }

    // -----------------------------------------------------------------------
    // indexOnlyDelete (delete-by-values kind) wire gate
    // -----------------------------------------------------------------------

    /// An indexOnlyDelete transition cannot decode at all on pre-4.2
    /// software, so blocks never contain one below PV14 — this check is
    /// what keeps NEW software agreeing with old software at check_tx
    /// while an earlier protocol version is still active. Admitted at PV14
    /// (`document_index_only_delete_state_transition` is `Some` in
    /// STATE_TRANSITION_SERIALIZATION_VERSIONS_V3), rejected below (where
    /// the kind's table entry is `None`).
    #[test]
    fn validate_base_structure_v0_gates_index_only_delete_by_protocol_version() {
        use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::DocumentIndexOnlyDeleteTransitionV0;

        let index_only_delete = DocumentTransition::IndexOnlyDelete(
            DocumentIndexOnlyDeleteTransition::V0(DocumentIndexOnlyDeleteTransitionV0 {
                base: make_base(1, "like"),
                data: Default::default(),
            }),
        );

        let batch = make_batch_v0(vec![index_only_delete]);

        let pv13 = PlatformVersion::get(13).expect("PV13 exists");
        let result = batch
            .validate_base_structure_v0(pv13)
            .expect("no protocol err");
        assert!(
            result.errors.iter().any(|error| matches!(
                error,
                ConsensusError::BasicError(BasicError::UnsupportedVersionError(_))
            )),
            "PV13 must reject an indexOnly delete as an unsupported version, got {:?}",
            result.errors
        );

        let pv14 = PlatformVersion::get(14).expect("PV14 exists");
        let result = batch
            .validate_base_structure_v0(pv14)
            .expect("no protocol err");
        assert!(
            !result.errors.iter().any(|error| matches!(
                error,
                ConsensusError::BasicError(BasicError::UnsupportedVersionError(_))
            )),
            "PV14 must admit an indexOnly delete, got {:?}",
            result.errors
        );
    }

    // -----------------------------------------------------------------------
    // empty batch — DocumentTransitionsAreAbsentError
    // -----------------------------------------------------------------------

    #[test]
    fn validate_base_structure_v0_errors_when_v0_empty() {
        let pv = PlatformVersion::latest();
        let batch = make_batch_v0(vec![]);
        let result = batch
            .validate_base_structure_v0(pv)
            .expect("no protocol err");
        assert!(!result.is_valid());
        let errors = result.errors;
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ConsensusError::BasicError(BasicError::DocumentTransitionsAreAbsentError(_)) => {}
            other => panic!(
                "expected DocumentTransitionsAreAbsentError, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn validate_base_structure_v0_errors_when_v1_empty() {
        let pv = PlatformVersion::latest();
        let batch = make_batch_v1_empty();
        let result = batch
            .validate_base_structure_v0(pv)
            .expect("no protocol err");
        assert!(!result.is_valid());
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::DocumentTransitionsAreAbsentError(_)) => {}
            other => panic!(
                "expected DocumentTransitionsAreAbsentError, got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // valid single transition with no group / nonce / duplicate issues
    // -----------------------------------------------------------------------

    #[test]
    fn validate_base_structure_v0_passes_with_single_valid_transition() {
        let pv = PlatformVersion::latest();
        let batch = make_batch_v0(vec![make_create(1)]);
        let result = batch
            .validate_base_structure_v0(pv)
            .expect("no protocol err");
        assert!(result.is_valid(), "expected valid, got {:?}", result.errors);
    }

    // -----------------------------------------------------------------------
    // nonce out of bounds — high bits set above 40-bit cap
    // -----------------------------------------------------------------------

    #[test]
    fn validate_base_structure_v0_errors_on_nonce_with_high_bits_set() {
        let pv = PlatformVersion::latest();
        // MISSING_IDENTITY_REVISIONS_FILTER masks the top bits used to mark
        // a missing revision. Setting a value above 40 bits triggers the
        // NonceOutOfBoundsError path.
        let bad_nonce: u64 = u64::MAX;
        let batch = make_batch_v0(vec![make_delete(bad_nonce, 7)]);
        let result = batch
            .validate_base_structure_v0(pv)
            .expect("no protocol err");
        assert!(!result.is_valid());
        let has_nonce_err = result.errors.iter().any(|e| {
            matches!(
                e,
                ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(_))
            )
        });
        assert!(
            has_nonce_err,
            "expected NonceOutOfBoundsError, got {:?}",
            result.errors
        );
    }

    // -----------------------------------------------------------------------
    // max-transitions-exceeded — early-return before any other checks
    // -----------------------------------------------------------------------

    #[test]
    fn validate_base_structure_v0_errors_when_transitions_exceed_max() {
        // The latest platform version caps the batch at a small number of
        // transitions. Going over should produce a single
        // MaxDocumentsTransitionsExceededError and NO other errors (the
        // function early-returns).
        let pv = PlatformVersion::latest();
        let max = pv.system_limits.max_transitions_in_documents_batch as usize;
        let mut transitions = Vec::with_capacity(max + 1);
        // Use distinct ids to avoid duplicate noise — but we never reach the
        // duplicate-detection path anyway because we early-return.
        for i in 0..(max + 1) {
            transitions.push(make_delete(i as u64 + 1, i as u8));
        }
        let batch = make_batch_v0(transitions);
        let result = batch
            .validate_base_structure_v0(pv)
            .expect("no protocol err");
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1, "should early-return with one error");
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::MaxDocumentsTransitionsExceededError(_)) => {}
            other => panic!(
                "expected MaxDocumentsTransitionsExceededError, got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // V1 batch with only document transitions — same logic, takes the
    // BatchedTransitionRef::Document arm of the iterator instead.
    // -----------------------------------------------------------------------

    #[test]
    fn validate_base_structure_v0_passes_for_v1_with_documents_only() {
        let pv = PlatformVersion::latest();
        let batch = BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::new([0x02; 32]),
            transitions: vec![BatchedTransition::Document(make_create(1))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        });
        let result = batch
            .validate_base_structure_v0(pv)
            .expect("no protocol err");
        assert!(result.is_valid(), "expected valid, got {:?}", result.errors);
    }
}
