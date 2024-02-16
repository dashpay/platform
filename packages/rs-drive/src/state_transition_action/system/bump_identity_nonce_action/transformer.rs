use dpp::ProtocolError;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use crate::state_transition_action::system::bump_identity_nonce_action::{BumpIdentityNonceAction, BumpIdentityNonceActionV0};

impl BumpIdentityNonceAction {
    /// from identity update
    pub fn from_identity_update_transition(
        value: IdentityUpdateTransition,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityUpdateTransition::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_identity_update(v0)?.into(),
            ),
        }
    }

    /// from borrowed identity update
    pub fn from_borrowed_identity_update_transition(
        value: &IdentityUpdateTransition,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityUpdateTransition::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_borrowed_identity_update(v0)?
                    .into(),
            ),
        }
    }

    /// from identity update action
    pub fn from_identity_update_transition_action(
        value: IdentityUpdateTransitionAction,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityUpdateTransitionAction::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_identity_update_action(v0)?
                    .into(),
            ),
        }
    }

    /// from borrowed identity update action
    pub fn from_borrowed_identity_update_transition_action(
        value: &IdentityUpdateTransitionAction,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityUpdateTransitionAction::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_borrowed_identity_update_action(
                    v0,
                )?
                    .into(),
            ),
        }
    }

    /// from identity transfer
    pub fn from_identity_credit_transfer_transition(
        value: IdentityCreditTransferTransition,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditTransferTransition::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_identity_credit_transfer(v0)?.into(),
            ),
        }
    }

    /// from borrowed identity transfer
    pub fn from_borrowed_identity_credit_transfer_transition(
        value: &IdentityCreditTransferTransition,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditTransferTransition::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_borrowed_identity_credit_transfer(v0)?
                    .into(),
            ),
        }
    }

    /// from identity transfer action
    pub fn from_identity_credit_transfer_transition_action(
        value: IdentityCreditTransferTransitionAction,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditTransferTransitionAction::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_identity_credit_transfer_action(v0)?
                    .into(),
            ),
        }
    }

    /// from borrowed identity transfer action
    pub fn from_borrowed_identity_credit_transfer_transition_action(
        value: &IdentityCreditTransferTransitionAction,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditTransferTransitionAction::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_borrowed_identity_credit_transfer_action(
                    v0,
                )?
                    .into(),
            ),
        }
    }


    /// from identity withdrawal
    pub fn from_identity_credit_withdrawal_transition(
        value: IdentityCreditWithdrawalTransition,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditWithdrawalTransition::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_identity_credit_withdrawal(v0)?.into(),
            ),
        }
    }

    /// from borrowed identity withdrawal
    pub fn from_borrowed_identity_credit_withdrawal_transition(
        value: &IdentityCreditWithdrawalTransition,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditWithdrawalTransition::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_borrowed_identity_credit_withdrawal(v0)?
                    .into(),
            ),
        }
    }

    /// from identity withdrawal action
    pub fn from_identity_credit_withdrawal_transition_action(
        value: IdentityCreditWithdrawalTransitionAction,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditWithdrawalTransitionAction::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_identity_credit_withdrawal_action(v0)?
                    .into(),
            ),
        }
    }

    /// from borrowed identity withdrawal action
    pub fn from_borrowed_identity_credit_withdrawal_transition_action(
        value: &IdentityCreditWithdrawalTransitionAction,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreditWithdrawalTransitionAction::V0(v0) => Ok(
                BumpIdentityNonceActionV0::try_from_borrowed_identity_credit_withdrawal_action(
                    v0,
                )?
                    .into(),
            ),
        }
    }
}
