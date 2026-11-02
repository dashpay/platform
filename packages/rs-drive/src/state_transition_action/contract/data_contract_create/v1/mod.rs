/// transformer
pub mod transformer;

use dpp::contract_group::{ContractGroupInfo, ContractGroupMembership};
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// Data contract create transition action v1: v0 plus contract groups.
#[derive(Debug, Clone)]
pub struct DataContractCreateTransitionActionV1 {
    /// data contract
    pub data_contract: DataContract,
    /// identity nonce
    pub identity_nonce: IdentityNonce,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// The contract group the transition registers, with its derived id, if any.
    pub contract_group: Option<(Identifier, ContractGroupInfo)>,
    /// The contract group memberships the created contract declares.
    pub contract_group_memberships: Vec<ContractGroupMembership>,
}
