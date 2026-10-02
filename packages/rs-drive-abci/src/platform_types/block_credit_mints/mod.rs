use dpp::fee::Credits;
use drive::util::batch::DriveOperation;
use std::collections::BTreeMap;

/// The credits a block's applied state transitions minted into Platform, in total and per asset
/// lock transaction they came from. The daily withdrawal limit leaves out the mints of an asset
/// lock Core mined a whole credit pool window ago, and counts every other mint.
///
/// Mirrors applied state: the block loop rewinds it with the state a dropped transition rolls
/// back, or the block would record an inflow for a transition the proposal omits.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlockCreditMints {
    total: Credits,
    by_asset_lock: BTreeMap<[u8; 32], Credits>,
}

impl BlockCreditMints {
    /// The mints of one batch of operations (one executed state transition): their
    /// `AddToSystemCredits` total, attributed to the asset lock they spent when they name one.
    pub fn of_operations(operations: &[DriveOperation]) -> Self {
        let mut mints = BlockCreditMints {
            total: DriveOperation::credit_mints(operations),
            by_asset_lock: BTreeMap::new(),
        };
        if let Some((asset_lock_txid, amount)) = DriveOperation::asset_lock_credit_mints(operations)
        {
            mints.by_asset_lock.insert(asset_lock_txid, amount);
        }
        mints
    }

    /// Adds the mints of another batch, saturating like the total always has.
    pub fn add(&mut self, other: BlockCreditMints) {
        self.total = self.total.saturating_add(other.total);
        for (asset_lock_txid, amount) in other.by_asset_lock {
            let minted = self.by_asset_lock.entry(asset_lock_txid).or_default();
            *minted = minted.saturating_add(amount);
        }
    }

    /// Every credit the block's state transitions minted.
    pub fn total(&self) -> Credits {
        self.total
    }

    /// The credits minted per asset lock transaction id (in the byte order `Txid` holds it).
    pub fn by_asset_lock(&self) -> &BTreeMap<[u8; 32], Credits> {
        &self.by_asset_lock
    }

    /// The credits minted without naming one asset lock: none in practice, as every state
    /// transition that mints spends one.
    pub fn not_attributed_to_an_asset_lock(&self) -> Credits {
        let attributed = self
            .by_asset_lock
            .values()
            .fold(0u64, |sum, amount| sum.saturating_add(*amount));
        self.total.saturating_sub(attributed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
    use dpp::platform_value::Bytes36;
    use dpp::version::PlatformVersion;
    use drive::util::batch::SystemOperationType;

    fn spend(asset_lock: u8, amount: Credits) -> Vec<DriveOperation<'static>> {
        let mut outpoint = [asset_lock; 36];
        outpoint[32..].copy_from_slice(&0u32.to_le_bytes());
        vec![
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { amount }),
            DriveOperation::SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint: Bytes36::new(outpoint),
                asset_lock_value: AssetLockValue::new(
                    amount,
                    vec![],
                    0,
                    vec![],
                    PlatformVersion::latest(),
                )
                .expect("expected an asset lock value"),
            }),
        ]
    }

    #[test]
    fn should_sum_mints_per_asset_lock_and_in_total() {
        let mut mints = BlockCreditMints::default();
        mints.add(BlockCreditMints::of_operations(&spend(1, 300)));
        mints.add(BlockCreditMints::of_operations(&spend(2, 500)));
        mints.add(BlockCreditMints::of_operations(&spend(1, 200)));

        assert_eq!(mints.total(), 1_000);
        assert_eq!(
            mints.by_asset_lock(),
            &BTreeMap::from([([1; 32], 500), ([2; 32], 500)])
        );
        assert_eq!(mints.not_attributed_to_an_asset_lock(), 0);
    }

    #[test]
    fn should_count_a_mint_naming_no_asset_lock_in_the_total_only() {
        let mut mints = BlockCreditMints::default();
        mints.add(BlockCreditMints::of_operations(&[
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { amount: 70 }),
        ]));
        mints.add(BlockCreditMints::of_operations(&spend(3, 30)));

        assert_eq!(mints.total(), 100);
        assert_eq!(mints.not_attributed_to_an_asset_lock(), 70);
    }
}
