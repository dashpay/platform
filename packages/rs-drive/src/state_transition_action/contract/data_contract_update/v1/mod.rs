use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// The action a delta-based (V1) data contract update transition becomes.
///
/// The delta was merged onto the stored contract, so `data_contract` is the
/// full contract to store, like the V0 action. What differs is the fee: a
/// full-contract update pays the registration cost of the whole contract it
/// re-sends, while a delta only registers what it adds, so the action carries
/// that cost, computed from the transition before it was merged away.
#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV1 {
    /// The updated contract, as it will be stored.
    pub data_contract: DataContract,
    /// The identity contract nonce of the transition.
    pub identity_contract_nonce: IdentityNonce,
    /// The user fee increase of the transition.
    pub user_fee_increase: UserFeeIncrease,
    /// The registration cost of what the delta adds to the contract.
    pub registration_cost: Credits,
}
