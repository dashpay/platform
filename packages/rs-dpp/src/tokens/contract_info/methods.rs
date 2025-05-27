use crate::data_contract::TokenContractPosition;
use crate::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use crate::tokens::contract_info::TokenContractInfo;
use platform_value::Identifier;

impl TokenContractInfoV0Accessors for TokenContractInfo {
    fn contract_id(&self) -> Identifier {
        match self {
            TokenContractInfo::V0(v0) => v0.contract_id(),
        }
    }

    fn set_contract_id(&mut self, contract_id: Identifier) {
        match self {
            TokenContractInfo::V0(v0) => v0.set_contract_id(contract_id),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            TokenContractInfo::V0(v0) => v0.token_contract_position(),
        }
    }

    fn set_token_contract_position(&mut self, position: TokenContractPosition) {
        match self {
            TokenContractInfo::V0(v0) => v0.set_token_contract_position(position),
        }
    }
}
