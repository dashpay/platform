mod identity_and_signatures;
mod state;
mod structure;

use std::collections::{btree_map::Entry, BTreeMap};

use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::document::document_transition::{DocumentTransitionExt, DocumentTransitionObjectLike};
use dpp::document::validation::basic::validate_documents_batch_transition_basic::DOCUMENTS_BATCH_TRANSITIONS_SCHEMA_VALIDATOR;
use dpp::identity::PartialIdentity;
use dpp::{
    consensus::basic::BasicError,
    document::{
        validation::basic::validate_documents_batch_transition_basic::validate_document_transitions as validate_document_transitions_basic,
        DocumentsBatchTransition,
    },
    platform_value::{Identifier, Value},
    prelude::DocumentTransition,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransitionAction,
    },
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult, ValidationResult},
};

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::document_state_validation::validate_documents_batch_transition_state::validate_document_batch_transition_state;
use crate::validation::state_transition::documents_batch::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::validation::state_transition::documents_batch::state::v0::StateTransitionStateValidationV0;
use crate::validation::state_transition::documents_batch::structure::v0::StateTransitionStructureValidationV0;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature_v0;
use crate::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::validation::state_transition::transformer::StateTransitionActionTransformerV0;

impl StateTransitionActionTransformerV0 for DocumentsBatchTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action(platform, tx)
    }
}

impl StateTransitionValidationV0 for DocumentsBatchTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        protocol_version: u32,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        //todo: use protocol version to determine validation
        self.validate_structure_v0(drive, tx)
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        protocol_version: u32,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        //todo: use protocol version to determine validation
        self.validate_identity_and_signatures_v0(drive, transaction)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.validate_state_v0(platform, tx)
    }
}
