/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use derive_more::From;
use dpp::data_contract::DataContract;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// data contract create transition action
#[derive(Debug, Clone, From)]
pub enum DataContractCreateTransitionAction {
    /// v0
    V0(DataContractCreateTransitionActionV0),
}

impl DataContractCreateTransitionAction {
    /// data contract
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.data_contract,
        }
    }
    /// data contract ref
    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }

    /// identity nonce
    pub fn identity_nonce(&self) -> IdentityNonce {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.identity_nonce,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;

    fn make_action() -> DataContractCreateTransitionAction {
        let fixture =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version);
        let dc = fixture.data_contract_owned();
        let v0 = DataContractCreateTransitionActionV0 {
            data_contract: dc,
            identity_nonce: 42,
            user_fee_increase: 7,
        };
        DataContractCreateTransitionAction::from(v0)
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
        assert!(matches!(action, DataContractCreateTransitionAction::V0(_)));
        // Verify the fixture values are accessible through the enum wrapper.
        assert_eq!(action.identity_nonce(), 42);
        assert_eq!(action.user_fee_increase(), 7);
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
    fn test_identity_nonce() {
        let action = make_action();
        assert_eq!(action.identity_nonce(), 42);
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 7);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.identity_nonce(), 42);
        assert_eq!(cloned.user_fee_increase(), 7);
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
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
