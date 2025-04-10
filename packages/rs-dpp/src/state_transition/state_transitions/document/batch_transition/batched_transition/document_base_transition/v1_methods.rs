use crate::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentBaseTransitionV1Methods for DocumentBaseTransition {
    fn token_payment_info(&self) -> Option<TokenPaymentInfo> {
        match self {
            DocumentBaseTransition::V0(_) => None,
            DocumentBaseTransition::V1(v1) => v1.token_payment_info,
        }
    }

    fn token_payment_info_ref(&self) -> &Option<TokenPaymentInfo> {
        match self {
            DocumentBaseTransition::V0(_) => &None,
            DocumentBaseTransition::V1(v1) => v1.token_payment_info_ref(),
        }
    }

    fn set_token_payment_info(&mut self, token_payment_info: TokenPaymentInfo) {
        match self {
            DocumentBaseTransition::V0(_) => {}
            DocumentBaseTransition::V1(v1) => v1.set_token_payment_info(token_payment_info),
        }
    }

    fn clear_token_payment_info(&mut self) {
        match self {
            DocumentBaseTransition::V0(_) => {}
            DocumentBaseTransition::V1(v1) => v1.clear_token_payment_info(),
        }
    }
}
