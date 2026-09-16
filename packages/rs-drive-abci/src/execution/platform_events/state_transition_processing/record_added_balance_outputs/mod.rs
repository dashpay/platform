mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// Which event family produced the `added_to_balance_outputs` being recorded. The recorded SET of
/// transparent-address credits changed at protocol v13, and this discriminator is what the versioned
/// `record_added_balance_outputs` method filters on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::execution) enum AddedBalanceOutputsOrigin {
    /// Long-standing transparent credits (e.g. `IdentityCreditTransferToAddresses`). Recorded in
    /// every version.
    Transparent,
    /// Shielded-spend transparent credits: the `Unshield` net output, the `ShieldFromAssetLock`
    /// surplus, and the `IdentityCreateFromShieldedPool` duplicate-key fallback. Recorded only from
    /// v1 (protocol v13); dropped by v0 so pre-v13 blocks byte-match.
    ShieldedSpend,
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fold the transparent-address credits an applied event produced into the per-block
    /// address-balance update map that feeds the recent-address-balance-changes tree.
    ///
    /// Versioned because the recorded SET changed at protocol v13: v0 records only
    /// `Transparent`-origin credits (the historical behavior), v1 additionally records
    /// `ShieldedSpend`-origin credits. Events always carry their outputs regardless of version — the
    /// version decides only whether shielded-origin outputs are folded — and construction and the
    /// downstream storage stay unconditional.
    ///
    /// This fold is NOT the only part of the v13 recorded-set expansion: `process_raw_state_transitions`
    /// has its own v0/v1 (a SIBLING versioned method) where v1 additionally records the balance
    /// effects of paid-invalid / unsuccessful-paid transitions that v0 drops. That method reads its
    /// OWN version field (`process_raw_state_transitions`), not this one — but both bump to v1
    /// together in the v13 method set, so the expansion activates atomically. Neither site carries a
    /// version conditional inside a _v0 function; the version is chosen by dispatch.
    pub(in crate::execution) fn record_added_balance_outputs(
        &self,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
        origin: AddedBalanceOutputsOrigin,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .record_added_balance_outputs
        {
            0 => self.record_added_balance_outputs_v0(
                address_balances_in_update,
                added_to_balance_outputs,
                origin,
            ),
            1 => self.record_added_balance_outputs_v1(
                address_balances_in_update,
                added_to_balance_outputs,
                origin,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "record_added_balance_outputs".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

/// Merge `added_to_balance_outputs` into `address_balances_in_update` (when both are present) with
/// the standard Set+Add merge semantics: Set + Add = Set(sum), Add + Add = Add(sum), both saturating.
/// Shared by v0 and v1 — the only difference between the two versions is the origin filter in the
/// caller, never the merge itself.
fn fold_added_balance_outputs(
    address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
    added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
) {
    let (Some(balance_updates), Some(outputs)) =
        (address_balances_in_update, added_to_balance_outputs)
    else {
        return;
    };
    for (address, credits) in outputs {
        balance_updates
            .entry(address)
            .and_modify(|existing| {
                *existing = match existing {
                    // Set + Add = Set to combined value
                    CreditOperation::SetCredits(set_val) => {
                        CreditOperation::SetCredits(set_val.saturating_add(credits))
                    }
                    // Add + Add = saturating add
                    CreditOperation::AddToCredits(add_val) => {
                        CreditOperation::AddToCredits(add_val.saturating_add(credits))
                    }
                };
            })
            .or_insert(CreditOperation::AddToCredits(credits));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;

    /// v0 (protocol < v13): `Transparent`-origin credits are folded (historical behavior);
    /// `ShieldedSpend`-origin credits are dropped even though the event carried them.
    #[test]
    fn v0_folds_transparent_and_drops_shielded_spend() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let v12 = PlatformVersion::get(12).expect("v12 must exist");

        let transparent = PlatformAddress::P2pkh([0xA1; 20]);
        let shielded = PlatformAddress::P2pkh([0xB2; 20]);

        // Transparent-origin: recorded by v0.
        let mut map: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        platform
            .record_added_balance_outputs(
                Some(&mut map),
                Some(BTreeMap::from([(transparent, 100)])),
                AddedBalanceOutputsOrigin::Transparent,
                v12,
            )
            .expect("record must not error");
        assert_eq!(
            map.get(&transparent),
            Some(&CreditOperation::AddToCredits(100)),
            "v0 must record transparent-origin credits"
        );

        // ShieldedSpend-origin: dropped by v0.
        let mut map2: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        platform
            .record_added_balance_outputs(
                Some(&mut map2),
                Some(BTreeMap::from([(shielded, 200)])),
                AddedBalanceOutputsOrigin::ShieldedSpend,
                v12,
            )
            .expect("record must not error");
        assert!(
            map2.is_empty(),
            "v0 must DROP shielded-spend-origin credits (pre-v13 byte-match)"
        );
    }

    /// v1 (protocol v13+): both origins are folded.
    #[test]
    fn v1_records_both_origins() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let v13 = PlatformVersion::get(13).expect("v13 must exist");

        let transparent = PlatformAddress::P2pkh([0xA1; 20]);
        let shielded = PlatformAddress::P2pkh([0xB2; 20]);

        let mut map: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        platform
            .record_added_balance_outputs(
                Some(&mut map),
                Some(BTreeMap::from([(transparent, 100)])),
                AddedBalanceOutputsOrigin::Transparent,
                v13,
            )
            .expect("record must not error");
        platform
            .record_added_balance_outputs(
                Some(&mut map),
                Some(BTreeMap::from([(shielded, 200)])),
                AddedBalanceOutputsOrigin::ShieldedSpend,
                v13,
            )
            .expect("record must not error");

        assert_eq!(
            map.get(&transparent),
            Some(&CreditOperation::AddToCredits(100)),
            "v1 must record transparent-origin credits"
        );
        assert_eq!(
            map.get(&shielded),
            Some(&CreditOperation::AddToCredits(200)),
            "v1 must record shielded-spend-origin credits"
        );
    }
}
