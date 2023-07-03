use crate::state_transition::fee::Credits;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditTransferTransitionActionV0 {
    pub transfer_amount: Credits,
    pub recipient_id: Identifier,
    pub identity_id: Identifier,
}

impl From<IdentityCreditTransferTransitionV0> for IdentityCreditTransferTransitionActionV0 {
    fn from(value: IdentityCreditTransferTransitionV0) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id: owner_id,
            recipient_id,
            amount,
            ..
        } = value;
        IdentityCreditTransferTransitionActionV0 {
            identity_id: owner_id,
            recipient_id,
            transfer_amount: amount,
        }
    }
}

impl From<&IdentityCreditTransferTransitionV0> for IdentityCreditTransferTransitionActionV0 {
    fn from(value: &IdentityCreditTransferTransitionV0) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            recipient_id,
            amount,
            ..
        } = value;
        IdentityCreditTransferTransitionActionV0 {
            identity_id: *identity_id,
            recipient_id: *recipient_id,
            transfer_amount: *amount,
        }
    }
}
