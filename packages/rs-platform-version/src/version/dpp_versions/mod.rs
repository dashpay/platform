pub mod dpp_asset_lock_versions;
pub mod dpp_contract_versions;
pub mod dpp_costs_versions;
pub mod dpp_document_versions;
pub mod dpp_factory_versions;
pub mod dpp_identity_versions;
pub mod dpp_method_versions;
pub mod dpp_state_transition_conversion_versions;
pub mod dpp_state_transition_method_versions;
pub mod dpp_state_transition_serialization_versions;
pub mod dpp_state_transition_versions;
pub mod dpp_token_versions;
pub mod dpp_validation_versions;
pub mod dpp_voting_versions;

use dpp_asset_lock_versions::DPPAssetLockVersions;
use dpp_contract_versions::DPPContractVersions;
use dpp_costs_versions::DPPCostsVersions;
use dpp_document_versions::DPPDocumentVersions;
use dpp_factory_versions::DPPFactoryVersions;
use dpp_identity_versions::DPPIdentityVersions;
use dpp_method_versions::DPPMethodVersions;
use dpp_state_transition_conversion_versions::DPPStateTransitionConversionVersions;
use dpp_state_transition_method_versions::DPPStateTransitionMethodVersions;
use dpp_state_transition_serialization_versions::DPPStateTransitionSerializationVersions;
use dpp_state_transition_versions::DPPStateTransitionVersions;
use dpp_token_versions::DPPTokenVersions;
use dpp_validation_versions::DPPValidationVersions;
use dpp_voting_versions::DPPVotingVersions;

#[derive(Clone, Debug, Default)]
pub struct DPPVersion {
    pub costs: DPPCostsVersions,
    pub validation: DPPValidationVersions,
    // TODO: Should be split by state transition type
    pub state_transition_serialization_versions: DPPStateTransitionSerializationVersions,
    pub state_transition_conversion_versions: DPPStateTransitionConversionVersions,
    pub state_transition_method_versions: DPPStateTransitionMethodVersions,
    pub state_transitions: DPPStateTransitionVersions,
    pub contract_versions: DPPContractVersions,
    pub document_versions: DPPDocumentVersions,
    pub identity_versions: DPPIdentityVersions,
    pub voting_versions: DPPVotingVersions,
    pub token_versions: DPPTokenVersions,
    pub asset_lock_versions: DPPAssetLockVersions,
    pub methods: DPPMethodVersions,
    pub factory_versions: DPPFactoryVersions,
}
