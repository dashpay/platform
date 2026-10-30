use crate::error::Error;
use dpp::consensus::signature::ContractBoundedKeyOutOfBoundsError;
use dpp::contract_group::{ContractGroupMember, ContractGroupMembership};
use dpp::identity::contract_bounds::{BatchedTransitionBoundsCheck, ContractBounds};
use dpp::identity::Purpose;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::ProtocolError;
use std::collections::BTreeSet;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTransitionIdError;
use dpp::consensus::signature::{InvalidSignaturePublicKeySecurityLevelError, SignatureError};
use dpp::dashcore::Network;
use dpp::document::Document;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::{StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned, StateTransitionOwned};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::validation::ConsensusValidationResult;

use dpp::version::PlatformVersion;

use drive::state_transition_action::batch::{BatchTransitionAction, ResolvedContractGroupMemberships};
use dpp::data_contract::document_type::DocumentTypeRef;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_index_only_delete_transition_action::DocumentIndexOnlyDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_create_transition_action::DocumentCreateTransitionActionValidation;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use dpp::consensus::state::token::TokenNotTransferableError;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::common::validate_document_not_expired::validate_document_action_not_expired;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::document::document_purchase_transition_action::DocumentPurchaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_transfer_transition_action::DocumentTransferTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_update_price_transition_action::DocumentUpdatePriceTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStructureValidationV1
{
    fn validate_advanced_structure_from_state_v1(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &BatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV1 for BatchTransition {
    fn validate_advanced_structure_from_state_v1(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &BatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let security_levels = action.combined_security_level_requirement()?;

        let signing_key = identity.loaded_public_keys.get(&self.signature_public_key_id()).ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution("the key must exist for advanced structure validation as we already fetched it during signature validation")))?;

        if !security_levels.contains(&signing_key.security_level()) {
            // We only need to bump the first identity data contract nonce as that will make a replay
            // attack not possible

            let first_transition = self.first_transition().ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution("There must be at least one state transition as this is already verified in basic validation")))?;

            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_batched_transition_ref(
                    first_transition,
                    self.owner_id(),
                    self.user_fee_increase(),
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![SignatureError::InvalidSignaturePublicKeySecurityLevelError(
                    InvalidSignaturePublicKeySecurityLevelError::new(
                        signing_key.security_level(),
                        security_levels,
                    ),
                )
                .into()],
            ));
        }

        // Every transition's request to have the contract owner pay its gas must be one its
        // document type's token cost offers, and a batch names one payer for all of them. Both
        // sides travel on the action, so this reads no state.
        if let Err(error) = action.resolve_gas_payer() {
            let first_transition = self.first_transition().ok_or(Error::Execution(
                ExecutionError::CorruptedCodeExecution("empty validated batch"),
            ))?;
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_batched_transition_ref(
                    first_transition,
                    self.owner_id(),
                    self.user_fee_increase(),
                ),
            );
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![error],
            ));
        }

        // Every transition that owes an action fee must have agreed to it: it names the amounts
        // and the pricing its document type declares, and accepts the fee multiplier of the
        // epoch the batch executes in. The contract is read when the action executes, so
        // without this its owner could change what a signed transition pays. The declaration,
        // the agreement and the multiplier all travel on the action, so this reads no state.
        if let Err(error) = action.validate_action_fee_agreements()? {
            let first_transition = self.first_transition().ok_or(Error::Execution(
                ExecutionError::CorruptedCodeExecution("empty validated batch"),
            ))?;
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_batched_transition_ref(
                    first_transition,
                    self.owner_id(),
                    self.user_fee_increase(),
                ),
            );
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![error],
            ));
        }

        // A contract-bound AUTHENTICATION key may only act inside its contract (and document
        // type), or inside the members of its contract group. The signer is authenticated, so
        // an out-of-bounds member is a paid failure.
        if signing_key.purpose() == Purpose::AUTHENTICATION {
            if let Some(bounds) = signing_key.contract_bounds() {
                let mut out_of_bounds = false;
                let mut billed_contracts = BTreeSet::new();
                for member in self.transitions_iter() {
                    match bounds.check_batched_transition(member) {
                        BatchedTransitionBoundsCheck::Allowed => {}
                        BatchedTransitionBoundsCheck::Denied => {
                            out_of_bounds = true;
                            break;
                        }
                        BatchedTransitionBoundsCheck::RequiresContractGroupMembership {
                            contract_group_id,
                            contract_id,
                            member,
                        } => {
                            // The transformer resolved the member contract's group
                            // memberships into the action, with the fee of that read. The
                            // fee is billed here, once per contract, where the answer is
                            // used. The whole contract or the exact document type or token
                            // must belong to the key's group.
                            let resolved = action.contract_group_memberships(&contract_id).ok_or(
                                Error::Execution(ExecutionError::CorruptedCodeExecution(
                                    "the batch transformer must resolve contract group memberships for a batch signed by a group-bound key",
                                )),
                            )?;
                            if billed_contracts.insert(contract_id) {
                                execution_context.add_operation(
                                    ValidationOperation::PrecalculatedOperation(
                                        resolved.fee.clone(),
                                    ),
                                );
                            }
                            let memberships = &resolved.memberships;
                            let whole_contract = ContractGroupMembership {
                                contract_group_id,
                                member: ContractGroupMember::Contract,
                            };
                            let exact = ContractGroupMembership {
                                contract_group_id,
                                member,
                            };
                            if !memberships.contains(&whole_contract)
                                && !memberships.contains(&exact)
                            {
                                out_of_bounds = true;
                                break;
                            }
                        }
                    }
                }
                // A create may consume documents of other types of its own contract
                // (`consume`), deleting them with the key that signs it: the key must be
                // allowed to act on those types as well
                if !out_of_bounds {
                    'creates: for transition in action.transitions() {
                        let BatchedTransitionAction::DocumentAction(
                            DocumentTransitionAction::CreateAction(create),
                        ) = transition
                        else {
                            continue;
                        };
                        let contract = &create.base().data_contract_fetch_info().contract;
                        let document_type = contract
                            .document_type_for_name(create.base().document_type_name())
                            .map_err(ProtocolError::from)?;
                        // Shipped v1 (PV14; PV15 via v2) edited in place, output-identical:
                        // see `consumption_in_bounds_matches_the_rule_it_replaced`.
                        let allowed = create_consumption_in_bounds(
                            bounds,
                            document_type,
                            action.contract_group_memberships(&contract.id()),
                        )?;
                        if !allowed {
                            out_of_bounds = true;
                            break 'creates;
                        }
                    }
                }
                if out_of_bounds {
                    let first = self.first_transition().ok_or(Error::Execution(
                        ExecutionError::CorruptedCodeExecution("empty validated batch"),
                    ))?;
                    let bump = BumpIdentityDataContractNonceAction::from_batched_transition_ref(
                        first,
                        self.owner_id(),
                        self.user_fee_increase(),
                    );
                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        StateTransitionAction::BumpIdentityDataContractNonceAction(bump),
                        vec![ContractBoundedKeyOutOfBoundsError::new(signing_key.id()).into()],
                    ));
                }
            }
        }

        // We should validate that all newly created documents have valid ids
        for transition in self.transitions_iter() {
            if let BatchedTransitionRef::Document(DocumentTransition::Create(create_transition)) =
                transition
            {
                // Validate the ID. It commits to the identity contract nonce
                // of this transition, which is consumed at most once, so an id
                // can be produced at most once: a deleted document can not be
                // created again under the id it had.
                let generated_document_id = Document::generate_document_id(
                    create_transition.base().data_contract_id_ref(),
                    &self.owner_id(),
                    create_transition.base().document_type_name(),
                    &create_transition.entropy(),
                    create_transition.base().identity_contract_nonce(),
                    platform_version,
                )?;

                // The nonce derived id is billed by what the double SHA-256
                // really hashes, both passes (4 blocks for most document type
                // names), instead of the flat 2 blocks up to v13.
                execution_context.add_operation(ValidationOperation::DoubleSha256(
                    Document::generate_document_id_sha256_blocks(
                        create_transition.base().document_type_name(),
                        platform_version,
                    )?,
                ));

                let id = create_transition.base().id();
                if generated_document_id != id {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                            create_transition.base(),
                            self.owner_id(),
                            self.user_fee_increase(),
                        ),
                    );

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![
                            InvalidDocumentTransitionIdError::new(generated_document_id, id).into(),
                        ],
                    ));
                }
            }
        }

        // A document whose type declares a `ttl` is no longer replaced, transferred, bought or
        // repriced once that has passed (`DocumentExpiredError`), judged from the document the
        // action carries and the block time. Here rather than in the state validation, so
        // check_tx refuses the change too.
        for transition in action.transitions() {
            let BatchedTransitionAction::DocumentAction(document_action) = transition else {
                continue;
            };
            let result = validate_document_action_not_expired(document_action, block_info)?;
            if !result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(
                        document_action.base(),
                        self.owner_id(),
                        self.user_fee_increase(),
                    ),
                );
                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    result.errors,
                ));
            }
        }

        // Next we need to validate the structure of all actions (this means with the data contract)
        for transition in action.transitions() {
            match transition {
                BatchedTransitionAction::DocumentAction(document_action) => match document_action {
                    DocumentTransitionAction::CreateAction(create_action) => {
                        let result = create_action.validate_structure(
                            identity.id,
                            block_info,
                            network,
                            platform_version,
                        )?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(document_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                    DocumentTransitionAction::ReplaceAction(replace_action) => {
                        let result =
                            replace_action.validate_structure(identity.id, platform_version)?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(replace_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                    DocumentTransitionAction::DeleteAction(delete_action) => {
                        let result = delete_action.validate_structure(platform_version)?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(delete_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                    DocumentTransitionAction::TransferAction(transfer_action) => {
                        let result = transfer_action.validate_structure(platform_version)?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transfer_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                    DocumentTransitionAction::UpdatePriceAction(update_price_action) => {
                        let result = update_price_action.validate_structure(platform_version)?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(update_price_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                    DocumentTransitionAction::PurchaseAction(purchase_action) => {
                        let result = purchase_action.validate_structure(platform_version)?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(purchase_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                    DocumentTransitionAction::IndexOnlyDeleteAction(index_only_delete_action) => {
                        let result =
                            index_only_delete_action.validate_structure(platform_version)?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(index_only_delete_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                },
                BatchedTransitionAction::TokenAction(token_transition_action) => {
                    // token actions do advanced structure validation on the base action, and a
                    // transfer is refused for a token that is not transferable
                    let result = token_transition_action
                        .base()
                        .validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(token_transition_action.base(), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                    // The token's configuration comes with the contract the action already
                    // carries, so the check needs no read and the mempool refuses the transfer
                    // as well. The base validation above established the token exists.
                    if matches!(
                        token_transition_action,
                        TokenTransitionAction::TransferAction(_)
                    ) && !token_transition_action
                        .base()
                        .token_configuration()?
                        .is_transferable()
                    {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(token_transition_action.base(), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            vec![TokenNotTransferableError::new(
                                token_transition_action.base().token_id(),
                                "a token transfer".to_string(),
                            )
                            .into()],
                        ));
                    }
                }
                BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we should not have a bump identity contract nonce at this stage",
                    )));
                }
            }
        }
        Ok(ConsensusValidationResult::new())
    }
}

/// Whether `bounds`, already found to cover a create of `document_type`, also cover every type
/// the create may consume ([`ContractBounds::check_document_consumption`]). A contract group
/// bound is answered from `memberships`, those the transformer resolved for the create's
/// contract; the batch billed that read where the create's own check used it.
fn create_consumption_in_bounds(
    bounds: &ContractBounds,
    document_type: DocumentTypeRef<'_>,
    memberships: Option<&ResolvedContractGroupMemberships>,
) -> Result<bool, Error> {
    for check in bounds.check_document_consumption(document_type) {
        let allowed = match check {
            BatchedTransitionBoundsCheck::Allowed => true,
            BatchedTransitionBoundsCheck::Denied => false,
            BatchedTransitionBoundsCheck::RequiresContractGroupMembership {
                contract_group_id,
                member,
                ..
            } => {
                let resolved =
                    memberships.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "the create's own bounds check resolved its contract's group memberships",
                    )))?;
                [ContractGroupMember::Contract, member]
                    .into_iter()
                    .any(|member| {
                        resolved.memberships.contains(&ContractGroupMembership {
                            contract_group_id,
                            member,
                        })
                    })
            }
        };
        if !allowed {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::create_consumption_in_bounds;
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use dpp::contract_group::{ContractGroupMember, ContractGroupMembership};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::DataContract;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::platform_value::Identifier;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::json_document::json_document_to_json_value;
    use dpp::version::PlatformVersion;
    use drive::drive::contract_groups::types::ContractGroupMembershipsForContract;
    use drive::state_transition_action::batch::ResolvedContractGroupMemberships;
    use std::collections::BTreeSet;

    /// The consumed-type check of `validate_advanced_structure_from_state_v1` as it shipped in
    /// PV14 (v5.0.1), copied verbatim but for the memberships, which it read from the action
    /// for the create's contract.
    fn shipped_rule(
        bounds: &ContractBounds,
        document_type: DocumentTypeRef<'_>,
        memberships: Option<&ResolvedContractGroupMemberships>,
    ) -> Result<bool, Error> {
        for consumed_type in document_type.consumable_document_type_names() {
            let allowed = match bounds {
                // The create itself is inside the contract
                ContractBounds::SingleContract { .. } => true,
                ContractBounds::SingleContractDocumentType {
                    document_type_name, ..
                } => document_type_name == consumed_type,
                ContractBounds::ContractGroup { id } => {
                    let resolved = memberships.ok_or(Error::Execution(
                        ExecutionError::CorruptedCodeExecution(
                            "the create's own bounds check resolved its contract's group memberships",
                        ),
                    ))?;
                    [
                        ContractGroupMember::Contract,
                        ContractGroupMember::DocumentType(consumed_type.to_string()),
                    ]
                    .into_iter()
                    .any(|member| {
                        resolved.memberships.contains(&ContractGroupMembership {
                            contract_group_id: *id,
                            member,
                        })
                    })
                }
            };
            if !allowed {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Contracts whose document types consume others: DPNS (`domain` consumes `preorder`) and
    /// the commit-reveal reference-validation fixture, parsed at `platform_version`.
    fn consuming_contracts(platform_version: &PlatformVersion) -> Vec<DataContract> {
        let fixture = json_document_to_json_value(
            "tests/supporting_files/contract/reference-validation/reference-validation-contract-commit-reveal.json",
        )
        .expect("expected the commit-reveal contract json");
        vec![
            load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .expect("expected the DPNS contract"),
            DataContract::from_json(fixture, true, platform_version)
                .expect("expected to parse the commit-reveal contract"),
        ]
    }

    fn resolved(
        memberships: ContractGroupMembershipsForContract,
    ) -> ResolvedContractGroupMemberships {
        ResolvedContractGroupMemberships {
            memberships,
            fee: Default::default(),
        }
    }

    /// The memberships a group-bound create may find resolved for its contract: none resolved
    /// at all, none held, the whole contract, or one document type or token, in the key's
    /// group or another.
    fn membership_cases(
        type_names: &[String],
        group: Identifier,
        other_group: Identifier,
    ) -> Vec<Option<ResolvedContractGroupMemberships>> {
        let mut cases = vec![
            None,
            Some(resolved(Default::default())),
            Some(resolved(ContractGroupMembershipsForContract {
                contract: BTreeSet::from([group]),
                ..Default::default()
            })),
            Some(resolved(ContractGroupMembershipsForContract {
                contract: BTreeSet::from([other_group]),
                ..Default::default()
            })),
            Some(resolved(ContractGroupMembershipsForContract {
                tokens: [(0, BTreeSet::from([group]))].into(),
                ..Default::default()
            })),
        ];
        for name in type_names {
            for held_by in [group, other_group] {
                cases.push(Some(resolved(ContractGroupMembershipsForContract {
                    document_types: [(name.clone(), BTreeSet::from([held_by]))].into(),
                    ..Default::default()
                })));
            }
        }
        // Every type in the key's group at once
        cases.push(Some(resolved(ContractGroupMembershipsForContract {
            document_types: type_names
                .iter()
                .map(|name| (name.clone(), BTreeSet::from([group])))
                .collect(),
            ..Default::default()
        })));
        cases
    }

    /// `create_consumption_in_bounds`, which asks dpp, answers exactly as the rule shipped in
    /// v1 at PV14 did, on every bound kind, consuming type and membership state, error included.
    #[test]
    fn consumption_in_bounds_matches_the_rule_it_replaced() {
        let group = Identifier::from([7; 32]);
        let other_group = Identifier::from([8; 32]);
        let foreign_contract = Identifier::from([9; 32]);
        let mut consuming_types = 0;
        for platform_version in [
            PlatformVersion::get(14).expect("expected protocol version 14"),
            PlatformVersion::latest(),
        ] {
            for contract in consuming_contracts(platform_version) {
                let type_names: Vec<String> = contract.document_types().keys().cloned().collect();
                let mut bounds_cases = vec![
                    ContractBounds::SingleContract { id: contract.id() },
                    ContractBounds::SingleContract {
                        id: foreign_contract,
                    },
                    ContractBounds::SingleContractDocumentType {
                        id: contract.id(),
                        document_type_name: "absent".to_string(),
                    },
                    ContractBounds::ContractGroup { id: group },
                ];
                for name in &type_names {
                    for id in [contract.id(), foreign_contract] {
                        bounds_cases.push(ContractBounds::SingleContractDocumentType {
                            id,
                            document_type_name: name.clone(),
                        });
                    }
                }
                let memberships = membership_cases(&type_names, group, other_group);
                for name in &type_names {
                    let document_type = contract
                        .document_type_for_name(name)
                        .expect("expected the document type");
                    if !document_type.consumable_document_type_names().is_empty() {
                        consuming_types += 1;
                    }
                    for bounds in &bounds_cases {
                        for resolved in &memberships {
                            let expected = shipped_rule(bounds, document_type, resolved.as_ref());
                            let actual = create_consumption_in_bounds(
                                bounds,
                                document_type,
                                resolved.as_ref(),
                            );
                            assert_eq!(
                                format!("{actual:?}"),
                                format!("{expected:?}"),
                                "{} {name} {bounds:?} {resolved:?} at protocol version {}",
                                contract.id(),
                                platform_version.protocol_version
                            );
                        }
                    }
                }
            }
        }
        assert!(
            consuming_types >= 4,
            "expected the grid to cover consuming types at both protocol versions, saw {consuming_types}"
        );
    }
}
