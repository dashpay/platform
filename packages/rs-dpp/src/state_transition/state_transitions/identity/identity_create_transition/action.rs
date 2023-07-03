use crate::prelude::Identity;
use derive_more::From;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::identity_create_transition::v0_action::IdentityCreateTransitionActionV0;

#[derive(Debug, Clone, From)]
pub enum IdentityCreateTransitionAction {
    V0(IdentityCreateTransitionActionV0),
}

impl IdentityCreateTransitionAction {
    pub fn public_keys(self) -> Identity {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.data_contract,
        }
    }

    pub fn data_contract_ref(&self) -> &Identity {
        match self {
            IdentityCreateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }
}

impl From<IdentityCreateTransition> for IdentityCreateTransitionAction {
    fn from(value: IdentityCreateTransition) -> Self {
        match value {
            IdentityCreateTransition::V0(v0) => {
                IdentityCreateTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&IdentityCreateTransition> for IdentityCreateTransitionAction {
    fn from(value: &IdentityCreateTransition) -> Self {
        match value {
            IdentityCreateTransition::V0(v0) => {
                IdentityCreateTransitionActionV0::from(v0).into()
            }
        }
    }
}
