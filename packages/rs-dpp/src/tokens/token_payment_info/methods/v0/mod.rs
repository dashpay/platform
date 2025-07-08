use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::calculate_token_id;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use platform_value::Identifier;

pub trait TokenPaymentInfoMethodsV0: TokenPaymentInfoAccessorsV0 {
    fn is_valid_for_required_cost(&self, required_cost: TokenAmount) -> bool {
        if let Some(min_cost) = self.minimum_token_cost() {
            if required_cost < min_cost {
                return false;
            }
        }

        if let Some(max_cost) = self.maximum_token_cost() {
            if required_cost > max_cost {
                return false;
            }
        }

        true
    }

    fn matches_token_contract(
        &self,
        contract_id: &Option<Identifier>,
        token_contract_position: TokenContractPosition,
    ) -> bool {
        self.payment_token_contract_id_ref() == contract_id
            && self.token_contract_position() == token_contract_position
    }

    fn token_id(&self, current_contract_id: Identifier) -> Identifier {
        calculate_token_id(
            self.payment_token_contract_id()
                .unwrap_or(current_contract_id)
                .as_bytes(),
            self.token_contract_position(),
        )
        .into()
    }
}
