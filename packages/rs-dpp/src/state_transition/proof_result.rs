use crate::address_funds::PlatformAddress;
use crate::asset_lock::StoredAssetLockInfo;
use crate::balances::credits::TokenAmount;
use crate::data_contract::group::GroupSumPower;
use crate::data_contract::DataContract;
use crate::document::Document;
use crate::fee::Credits;
use crate::group::group_action_status::GroupActionStatus;
use crate::identity::{Identity, PartialIdentity};
use crate::prelude::AddressNonce;
use crate::tokens::info::IdentityTokenInfo;
use crate::tokens::status::TokenStatus;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::voting::votes::Vote;
use platform_value::Identifier;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, strum::Display, derive_more::TryInto)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StateTransitionProofResult {
    VerifiedDataContract(DataContract),
    VerifiedIdentity(Identity),
    VerifiedTokenBalanceAbsence(Identifier),
    VerifiedTokenBalance(Identifier, TokenAmount),
    VerifiedTokenIdentityInfo(Identifier, IdentityTokenInfo),
    VerifiedTokenPricingSchedule(Identifier, Option<TokenPricingSchedule>),
    VerifiedTokenStatus(TokenStatus),
    VerifiedTokenIdentitiesBalances(BTreeMap<Identifier, TokenAmount>),
    VerifiedPartialIdentity(PartialIdentity),
    VerifiedBalanceTransfer(PartialIdentity, PartialIdentity), //from/to
    VerifiedDocuments(BTreeMap<Identifier, Option<Document>>),
    VerifiedTokenActionWithDocument(Document),
    VerifiedTokenGroupActionWithDocument(GroupSumPower, Option<Document>),
    VerifiedTokenGroupActionWithTokenBalance(GroupSumPower, GroupActionStatus, Option<TokenAmount>),
    VerifiedTokenGroupActionWithTokenIdentityInfo(
        GroupSumPower,
        GroupActionStatus,
        Option<IdentityTokenInfo>,
    ),
    VerifiedTokenGroupActionWithTokenPricingSchedule(
        GroupSumPower,
        GroupActionStatus,
        Option<TokenPricingSchedule>,
    ),
    VerifiedMasternodeVote(Vote),
    VerifiedNextDistribution(Vote),
    VerifiedAddressInfos(BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>),
    VerifiedIdentityFullWithAddressInfos(
        Identity,
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedIdentityWithAddressInfos(
        PartialIdentity,
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedAssetLockConsumed(StoredAssetLockInfo),
    VerifiedShieldedNullifiers(Vec<(Vec<u8>, bool)>),
    VerifiedShieldedNullifiersWithAddressInfos(
        Vec<(Vec<u8>, bool)>,
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedShieldedNullifiersWithWithdrawalDocument(
        Vec<(Vec<u8>, bool)>,
        BTreeMap<Identifier, Option<Document>>,
    ),
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for StateTransitionProofResult {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for StateTransitionProofResult {}


#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use platform_value::Identifier;

    /// Non-default variant `VerifiedTokenBalance(id, amount)` with both
    /// tuple fields set so a per-property assertion catches silent
    /// variant flip / inner-zero on round-trip.
    fn fixture() -> StateTransitionProofResult {
        StateTransitionProofResult::VerifiedTokenBalance(
            Identifier::new([0xab; 32]),
            123_456_789u64,
        )
    }

    fn assert_per_property(actual: &StateTransitionProofResult) {
        match actual {
            StateTransitionProofResult::VerifiedTokenBalance(id, amount) => {
                assert_eq!(*id, Identifier::new([0xab; 32]), "VerifiedTokenBalance.id");
                assert_eq!(*amount, 123_456_789u64, "VerifiedTokenBalance.amount");
            }
            other => panic!("expected VerifiedTokenBalance, got {}", other),
        }
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = StateTransitionProofResult::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_per_property(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = StateTransitionProofResult::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_per_property(&recovered);
    }
}
