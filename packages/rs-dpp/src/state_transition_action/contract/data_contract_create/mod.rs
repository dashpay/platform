pub mod v0;
#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::prelude::DataContract;
use derive_more::From;
use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;

#[derive(Debug, Clone, From)]
pub enum DataContractCreateTransitionAction {
    V0(DataContractCreateTransitionActionV0),
}

impl DataContractCreateTransitionAction {
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.data_contract,
        }
    }

    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }
}

