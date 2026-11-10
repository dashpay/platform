use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// The action of a delta data contract update (version 1 of the transition, from protocol
/// version 15).
///
/// The delta was merged onto the stored contract, so `data_contract` is the whole contract to
/// store, as in the action of an update that carries it. What differs is the fee: an update
/// that carries the whole contract pays the registration cost of the whole contract again,
/// while a delta registers only what it adds, so its action carries that cost, which only the
/// transition knows.
#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV1 {
    /// The updated contract, as it will be stored.
    pub data_contract: DataContract,
    /// The identity contract nonce of the transition.
    pub identity_contract_nonce: IdentityNonce,
    /// The user fee increase of the transition.
    pub user_fee_increase: UserFeeIncrease,
    /// The registration cost of what the delta registers.
    pub registration_cost: Credits,
}
