/// transformer
pub mod transformer;

use dpp::data_contract::DataContract;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// Data contract update transition action v1.
/// This version is used for V1 state transitions that contain delta-based updates.
/// The old contract is fetched from state during validation, and the new contract
/// is produced by applying the deltas.
#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV1 {
    /// The existing data contract before the update (fetched from state).
    pub old_data_contract: DataContract,
    /// The new data contract after applying the update.
    pub data_contract: DataContract,
    /// Identity contract nonce.
    pub identity_contract_nonce: IdentityNonce,
    /// Fee multiplier.
    pub user_fee_increase: UserFeeIncrease,
}
