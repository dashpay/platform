//! The one mapping between asset-lock funding families and the accounts
//! whose pools fund them. Restore, reconstruction and the lock builder all
//! classify funding accounts through it.

use key_wallet::account::AccountType;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

/// Map a record's owning account to the asset-lock funding family it
/// implies. `None` for every non-funding account (standard, CoinJoin,
/// provider keys, DashPay, …).
///
/// For [`AccountType::IdentityTopUp`] the account key *is* the
/// registration index, so the destination identity index is recovered
/// exactly. The singleton families (registration, invitation, unbound
/// top-up, address top-ups) don't encode their destination index in the
/// account — the credit-output address is just "next unused" there — so
/// the index is reported as 0.
pub(crate) fn funding_family(account_type: &AccountType) -> Option<(AssetLockFundingType, u32)> {
    match account_type {
        AccountType::IdentityRegistration => Some((AssetLockFundingType::IdentityRegistration, 0)),
        AccountType::IdentityTopUp { registration_index } => {
            Some((AssetLockFundingType::IdentityTopUp, *registration_index))
        }
        AccountType::IdentityTopUpNotBoundToIdentity => {
            Some((AssetLockFundingType::IdentityTopUpNotBound, 0))
        }
        AccountType::IdentityInvitation => Some((AssetLockFundingType::IdentityInvitation, 0)),
        AccountType::AssetLockAddressTopUp => {
            Some((AssetLockFundingType::AssetLockAddressTopUp, 0))
        }
        AccountType::AssetLockShieldedAddressTopUp => {
            Some((AssetLockFundingType::AssetLockShieldedAddressTopUp, 0))
        }
        _ => None,
    }
}

/// The account whose pool funds a lock of `funding_type`; the inverse of
/// [`funding_family`].
pub(crate) fn funding_account_type(
    funding_type: AssetLockFundingType,
    identity_index: u32,
) -> AccountType {
    match funding_type {
        AssetLockFundingType::IdentityRegistration => AccountType::IdentityRegistration,
        AssetLockFundingType::IdentityTopUp => AccountType::IdentityTopUp {
            registration_index: identity_index,
        },
        AssetLockFundingType::IdentityTopUpNotBound => AccountType::IdentityTopUpNotBoundToIdentity,
        AssetLockFundingType::IdentityInvitation => AccountType::IdentityInvitation,
        AssetLockFundingType::AssetLockAddressTopUp => AccountType::AssetLockAddressTopUp,
        AssetLockFundingType::AssetLockShieldedAddressTopUp => {
            AccountType::AssetLockShieldedAddressTopUp
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every funding family's account classifies back as that family, so a
    /// family the builder funds cannot be missed by restore or
    /// reconstruction.
    #[test]
    fn should_classify_every_funding_familys_account_as_that_family() {
        let families = [
            AssetLockFundingType::IdentityRegistration,
            AssetLockFundingType::IdentityTopUp,
            AssetLockFundingType::IdentityTopUpNotBound,
            AssetLockFundingType::IdentityInvitation,
            AssetLockFundingType::AssetLockAddressTopUp,
            AssetLockFundingType::AssetLockShieldedAddressTopUp,
        ];
        for family in families {
            // Fails to compile when key-wallet adds a family missing above.
            match family {
                AssetLockFundingType::IdentityRegistration
                | AssetLockFundingType::IdentityTopUp
                | AssetLockFundingType::IdentityTopUpNotBound
                | AssetLockFundingType::IdentityInvitation
                | AssetLockFundingType::AssetLockAddressTopUp
                | AssetLockFundingType::AssetLockShieldedAddressTopUp => {}
            }
            let identity_index = match family {
                AssetLockFundingType::IdentityTopUp => 7,
                _ => 0,
            };

            assert_eq!(
                funding_family(&funding_account_type(family, identity_index)),
                Some((family, identity_index)),
                "{family:?}"
            );
        }
    }
}
