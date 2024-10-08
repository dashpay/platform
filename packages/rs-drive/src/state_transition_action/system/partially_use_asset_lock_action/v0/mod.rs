use dpp::fee::Credits;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::prelude::UserFeeIncrease;
mod transformer;
#[derive(Default, Debug, Clone)]
pub struct PartiallyUseAssetLockActionV0 {
    /// asset lock outpoint
    pub asset_lock_outpoint: Bytes36,
    /// initial credit value
    pub initial_credit_value: Credits,
    /// the previous transaction signable bytes hashes that tried to used this asset lock, but failed
    pub previous_transaction_hashes: Vec<Bytes32>,
    /// asset lock script
    pub asset_lock_script: Vec<u8>,
    /// remaining credit value AFTER used credits are deducted
    pub remaining_credit_value: Credits,
    /// the used credits for processing, this is what will go to Evonodes for processing
    /// this is after applying the user fee increase
    pub used_credits: Credits,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait PartiallyUseAssetLockActionAccessorsV0 {
    /// asset lock outpoint
    fn asset_lock_outpoint(&self) -> Bytes36;
    /// initial credit value
    fn initial_credit_value(&self) -> Credits;
    /// asset lock script used to very that the asset lock can be used
    fn asset_lock_script(&self) -> &Vec<u8>;
    /// asset lock script used to very that the asset lock can be used, this consumes the action
    fn asset_lock_script_owned(self) -> Vec<u8>;
    /// remaining credit value AFTER used credits are deducted
    fn remaining_credit_value(&self) -> Credits;
    /// the used credits for processing, this is what will go to Evonodes for processing
    fn used_credits(&self) -> Credits;
    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;

    /// the previous transaction signable bytes hashes that tried to used this asset lock, but failed
    fn previous_transaction_hashes_ref(&self) -> &Vec<Bytes32>;
}
