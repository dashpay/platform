use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use crate::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionV0;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use dpp::ProtocolError;

impl BumpIdentityNonceActionV0 {
    /// try from identity update
    pub fn try_from_identity_update(
        value: IdentityUpdateTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityUpdateTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase: fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed identity update
    pub fn try_from_borrowed_identity_update(
        value: &IdentityUpdateTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityUpdateTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase: fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            fee_multiplier: *fee_multiplier,
        })
    }

    /// try from identity update action
    pub fn try_from_identity_update_action(
        value: IdentityUpdateTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityUpdateTransitionActionV0 {
            identity_id,
            nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed identity update action
    pub fn try_from_borrowed_identity_update_action(
        value: &IdentityUpdateTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityUpdateTransitionActionV0 {
            identity_id,
            nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            fee_multiplier: *fee_multiplier,
        })
    }

    /// try from identity credit transfer
    pub fn try_from_identity_credit_transfer(
        value: IdentityCreditTransferTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase: fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed identity credit transfer
    pub fn try_from_borrowed_identity_credit_transfer(
        value: &IdentityCreditTransferTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase: fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            fee_multiplier: *fee_multiplier,
        })
    }

    /// try from identity credit transfer action
    pub fn try_from_identity_credit_transfer_action(
        value: IdentityCreditTransferTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditTransferTransitionActionV0 {
            identity_id,
            nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed identity credit transfer action
    pub fn try_from_borrowed_identity_credit_transfer_action(
        value: &IdentityCreditTransferTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditTransferTransitionActionV0 {
            identity_id,
            nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            fee_multiplier: *fee_multiplier,
        })
    }

    /// try from identity credit withdrawal
    pub fn try_from_identity_credit_withdrawal(
        value: IdentityCreditWithdrawalTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditWithdrawalTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase: fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed identity credit withdrawal
    pub fn try_from_borrowed_identity_credit_withdrawal(
        value: &IdentityCreditWithdrawalTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditWithdrawalTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase: fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            fee_multiplier: *fee_multiplier,
        })
    }

    /// try from identity credit withdrawal action
    pub fn try_from_identity_credit_withdrawal_action(
        value: IdentityCreditWithdrawalTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditWithdrawalTransitionActionV0 {
            identity_id,
            nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed identity credit withdrawal action
    pub fn try_from_borrowed_identity_credit_withdrawal_action(
        value: &IdentityCreditWithdrawalTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreditWithdrawalTransitionActionV0 {
            identity_id,
            nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            fee_multiplier: *fee_multiplier,
        })
    }
}
