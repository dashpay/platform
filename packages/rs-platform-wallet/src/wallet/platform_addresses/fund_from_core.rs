use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::PlatformWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};

#[derive(Debug)]
pub struct FundPlatformAddressFromCoreResult {
    pub asset_lock_txid: dashcore::Txid,
    pub asset_lock_vout: u32,
    pub address_changeset: PlatformAddressChangeSet,
}

impl PlatformWallet {
    /// Mint platform credits on `target_address` by locking
    /// `amount_duffs` of Core L1 funds.
    pub async fn fund_platform_address_from_core<AS, S>(
        &self,
        amount_duffs: u64,
        core_account_index: u32,
        platform_account_index: u32,
        target_address: PlatformAddress,
        asset_lock_signer: &AS,
        address_signer: &S,
    ) -> Result<FundPlatformAddressFromCoreResult, PlatformWalletError>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
        S: Signer<PlatformAddress> + Send + Sync,
    {
        let (proof, asset_lock_proof_path, out_point) = self
            .asset_locks
            .create_funded_asset_lock_proof(
                amount_duffs,
                core_account_index,
                AssetLockFundingType::AssetLockAddressTopUp,
                0,
                asset_lock_signer,
            )
            .await?;

        // With zero inputs the fee MUST come from the only output;
        // `DeductFromInput(0)` would surface as "index 0 out of bounds
        // (count: 0)" from the DPP layer.
        let mut addresses: BTreeMap<PlatformAddress, Option<Credits>> = BTreeMap::new();
        addresses.insert(target_address, None);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let address_changeset = self
            .platform
            .fund_from_asset_lock_with_signer(
                platform_account_index,
                addresses,
                proof,
                &asset_lock_proof_path,
                asset_lock_signer,
                fee_strategy,
                address_signer,
            )
            .await?;

        Ok(FundPlatformAddressFromCoreResult {
            asset_lock_txid: out_point.txid,
            asset_lock_vout: out_point.vout,
            address_changeset,
        })
    }
}
