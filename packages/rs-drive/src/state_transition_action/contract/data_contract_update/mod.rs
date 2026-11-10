/// transformer
pub mod transformer;
/// v0
pub mod v0;
/// v1
pub mod v1;

use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::v1::DataContractUpdateTransitionActionV1;
use derive_more::From;
use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// data contract update transition action
#[derive(Debug, Clone, From)]
pub enum DataContractUpdateTransitionAction {
    /// v0
    V0(DataContractUpdateTransitionActionV0),
    /// v1: the action of a delta update, which carries the delta's registration cost
    V1(DataContractUpdateTransitionActionV1),
}

impl DataContractUpdateTransitionAction {
    /// data contract
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.data_contract,
            DataContractUpdateTransitionAction::V1(transition) => transition.data_contract,
        }
    }
    /// data contract ref
    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &transition.data_contract,
            DataContractUpdateTransitionAction::V1(transition) => &transition.data_contract,
        }
    }

    /// data contract mut
    pub fn data_contract_mut(&mut self) -> &mut DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &mut transition.data_contract,
            DataContractUpdateTransitionAction::V1(transition) => &mut transition.data_contract,
        }
    }

    /// identity contract nonce
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => {
                transition.identity_contract_nonce
            }
            DataContractUpdateTransitionAction::V1(transition) => {
                transition.identity_contract_nonce
            }
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.user_fee_increase,
            DataContractUpdateTransitionAction::V1(transition) => transition.user_fee_increase,
        }
    }

    /// The action of a delta update, from the action of the update carrying the whole
    /// contract that the delta was merged into, which is `self`, and the registration cost
    /// of what the delta registers.
    pub fn into_delta_action(self, registration_cost: Credits) -> Self {
        DataContractUpdateTransitionActionV1 {
            identity_contract_nonce: self.identity_contract_nonce(),
            user_fee_increase: self.user_fee_increase(),
            data_contract: self.data_contract(),
            registration_cost,
        }
        .into()
    }

    /// The registration cost this update pays on top of its storage and processing fees:
    /// the whole contract's for an update that carries the whole contract, and that of
    /// what it registers alone for a delta.
    pub fn registration_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => {
                transition.data_contract.registration_cost(platform_version)
            }
            DataContractUpdateTransitionAction::V1(transition) => Ok(transition.registration_cost),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;

    fn make_action() -> DataContractUpdateTransitionAction {
        let fixture =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version);
        let dc = fixture.data_contract_owned();
        let v0 = DataContractUpdateTransitionActionV0 {
            data_contract: dc,
            identity_contract_nonce: 99,
            user_fee_increase: 3,
        };
        DataContractUpdateTransitionAction::from(v0)
    }

    /// The expected document type names produced by `get_data_contract_fixture`.
    const EXPECTED_DOC_TYPES: [&str; 7] = [
        "indexedDocument",
        "niceDocument",
        "noTimeDocument",
        "optionalUniqueIndexedDocument",
        "prettyDocument",
        "uniqueDates",
        "withByteArrays",
    ];

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(action, DataContractUpdateTransitionAction::V0(_)));
        // Verify the fixture values are accessible through the enum wrapper.
        assert_eq!(action.identity_contract_nonce(), 99);
        assert_eq!(action.user_fee_increase(), 3);
        assert_eq!(action.data_contract_ref().version(), 1);
    }

    #[test]
    fn test_data_contract_ref() {
        let action = make_action();
        let dc_ref = action.data_contract_ref();
        // Verify the contract has the expected version and document types.
        assert_eq!(dc_ref.version(), 1);
        let doc_types = dc_ref.document_types();
        assert_eq!(doc_types.len(), EXPECTED_DOC_TYPES.len());
        for name in &EXPECTED_DOC_TYPES {
            assert!(
                dc_ref.has_document_type_for_name(name),
                "expected document type '{}' not found",
                name
            );
        }
    }

    #[test]
    fn test_data_contract_owned() {
        let action = make_action();
        let original_id = action.data_contract_ref().id();
        let original_owner = action.data_contract_ref().owner_id();
        let dc = action.data_contract();
        assert_eq!(dc.id(), original_id);
        assert_eq!(dc.owner_id(), original_owner);
        assert_eq!(dc.version(), 1);
        assert_eq!(dc.document_types().len(), EXPECTED_DOC_TYPES.len());
    }

    #[test]
    fn test_data_contract_mut() {
        let mut action = make_action();
        let original_id = action.data_contract_ref().id();
        let dc_mut = action.data_contract_mut();
        // Verify the mutable reference provides access to the same contract.
        assert_eq!(dc_mut.id(), original_id);
        assert_eq!(dc_mut.version(), 1);
        assert_eq!(dc_mut.document_types().len(), EXPECTED_DOC_TYPES.len());
    }

    #[test]
    fn test_identity_contract_nonce() {
        let action = make_action();
        assert_eq!(action.identity_contract_nonce(), 99);
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 3);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.identity_contract_nonce(), 99);
        assert_eq!(cloned.user_fee_increase(), 3);
        // Verify the cloned contract preserves identity and structure.
        assert_eq!(
            cloned.data_contract_ref().id(),
            action.data_contract_ref().id()
        );
        assert_eq!(
            cloned.data_contract_ref().owner_id(),
            action.data_contract_ref().owner_id()
        );
        assert_eq!(cloned.data_contract_ref().version(), 1);
        assert_eq!(
            cloned.data_contract_ref().document_types().len(),
            EXPECTED_DOC_TYPES.len()
        );
    }

    #[test]
    fn should_pay_the_registration_cost_of_what_a_delta_registers() {
        let platform_version = PlatformVersion::latest();
        let action = make_action();
        let whole_contract_cost = action
            .data_contract_ref()
            .registration_cost(platform_version)
            .expect("expected a registration cost");
        assert_eq!(
            action
                .registration_cost(platform_version)
                .expect("expected a registration cost"),
            whole_contract_cost
        );

        let contract_id = action.data_contract_ref().id();
        let delta_action = action.into_delta_action(7);

        assert!(matches!(
            delta_action,
            DataContractUpdateTransitionAction::V1(_)
        ));
        assert_eq!(delta_action.identity_contract_nonce(), 99);
        assert_eq!(delta_action.user_fee_increase(), 3);
        assert_eq!(delta_action.data_contract_ref().id(), contract_id);
        assert_ne!(whole_contract_cost, 7);
        assert_eq!(
            delta_action
                .registration_cost(platform_version)
                .expect("expected a registration cost"),
            7
        );
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
