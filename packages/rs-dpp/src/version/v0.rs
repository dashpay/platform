use crate::version::drive_versions::DriveStructureVersion;
use crate::version::protocol_version::{
    FeatureVersionBounds, PlatformVersion,
};
use crate::version::{
    AbciStructureVersion, DataContractFactoryVersion,
    PlatformArchitectureVersion,
};
use std::collections::BTreeMap;
use crate::version::dpp_versions::{DocumentFeatureVersionBounds, DPPVersion, StateTransitionVersions};

pub(super) const PLATFORM_V1: PlatformVersion = PlatformVersion {
    protocol_version: 0,
    document: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    extended_document: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    contract: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    identity: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    proofs: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    costs: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    state_transition_signing: Default::default(),
    drive: Default::default(),
    abci_structure: AbciStructureVersion {
        extended_block_info: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
    },
    platform_architecture: PlatformArchitectureVersion {
        data_contract_factory: DataContractFactoryVersion {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            // The factory can only support data contract version 0
            allowed_contract_bounds_mapping: BTreeMap::from([(
                0,
                FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            )]),
        },
    },
    drive_abci: Default::default(),
    dpp: DPPVersion {
        state_transition_versions: StateTransitionVersions {
            identity_create_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            identity_update_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            identity_top_up_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            identity_credit_withdrawal_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            identity_credit_transfer_state_transition: Default::default(),
            contract_create_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            contract_update_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            documents_batch_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            document_base_state_transition: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            document_create_state_transition: DocumentFeatureVersionBounds {
                bounds: Default::default(),
                base_version_mapping: Default::default(),
            },
            document_replace_state_transition: DocumentFeatureVersionBounds {
                bounds: Default::default(),
                base_version_mapping: Default::default(),
            },
            document_delete_state_transition: DocumentFeatureVersionBounds {
                bounds: Default::default(),
                base_version_mapping: Default::default(),
            },
        },
        contract_versions: Default::default(),
        document_versions: Default::default(),
    }
};
