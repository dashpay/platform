use crate::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use crate::state_transition::batch_transition::document_base_transition::v2::v2_methods::DocumentBaseTransitionV2Methods;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::ProtocolError;

impl DocumentBaseTransitionV2Methods for DocumentBaseTransition {
    fn action_fee_agreement(&self) -> Option<DocumentActionFeeAgreement> {
        match self {
            DocumentBaseTransition::V0(_) | DocumentBaseTransition::V1(_) => None,
            DocumentBaseTransition::V2(v2) => v2.action_fee_agreement,
        }
    }

    fn set_action_fee_agreement(&mut self, action_fee_agreement: DocumentActionFeeAgreement) {
        match self {
            DocumentBaseTransition::V0(_) | DocumentBaseTransition::V1(_) => {}
            DocumentBaseTransition::V2(v2) => v2.set_action_fee_agreement(action_fee_agreement),
        }
    }

    fn clear_action_fee_agreement(&mut self) {
        match self {
            DocumentBaseTransition::V0(_) | DocumentBaseTransition::V1(_) => {}
            DocumentBaseTransition::V2(v2) => v2.clear_action_fee_agreement(),
        }
    }
}

impl DocumentBaseTransition {
    /// The version of the base, as the `document_base_state_transition` bounds count it
    pub fn feature_version(&self) -> u16 {
        match self {
            DocumentBaseTransition::V0(_) => 0,
            DocumentBaseTransition::V1(_) => 1,
            DocumentBaseTransition::V2(_) => 2,
        }
    }

    /// Sets the action fees the transition agrees to pay, refusing a base that cannot carry
    /// them: a transition built with an agreement must not go out without it.
    pub fn try_set_action_fee_agreement(
        &mut self,
        action_fee_agreement: DocumentActionFeeAgreement,
    ) -> Result<(), ProtocolError> {
        match self {
            DocumentBaseTransition::V0(_) | DocumentBaseTransition::V1(_) => {
                Err(ProtocolError::UnknownVersionMismatch {
                    method: "DocumentBaseTransition::try_set_action_fee_agreement".to_string(),
                    known_versions: vec![2],
                    received: self.feature_version(),
                })
            }
            DocumentBaseTransition::V2(v2) => {
                v2.set_action_fee_agreement(action_fee_agreement);
                Ok(())
            }
        }
    }
}
