use crate::address_funds::PlatformAddress;
use crate::asset_lock::StoredAssetLockInfo;
use crate::balances::credits::TokenAmount;
use crate::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractModerationListStatuses,
};
use crate::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
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
    // `TokenAmount`/`Credits` (u64) live in tuple variants / nested containers
    // that `#[json_safe_fields]` can't reach; apply the JS-safe helpers directly
    // so values above `MAX_SAFE_INTEGER` serialize as strings in human-readable
    // JSON. (`AddressNonce`/`GroupSumPower` are `u32` → already JS-safe; the
    // `Vec<u8>` nullifiers are arrays of bytes < 256 → no precision concern.)
    VerifiedTokenBalance(
        Identifier,
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json_safe_u64")
        )]
        TokenAmount,
    ),
    VerifiedTokenIdentityInfo(Identifier, IdentityTokenInfo),
    VerifiedTokenPricingSchedule(Identifier, Option<TokenPricingSchedule>),
    VerifiedTokenStatus(TokenStatus),
    VerifiedTokenIdentitiesBalances(
        #[cfg_attr(
            feature = "json-conversion",
            serde(
                with = "crate::serialization::json::safe_integer_map::json_safe_identifier_u64_map"
            )
        )]
        BTreeMap<Identifier, TokenAmount>,
    ),
    VerifiedPartialIdentity(PartialIdentity),
    VerifiedBalanceTransfer(PartialIdentity, PartialIdentity), //from/to
    VerifiedDocuments(BTreeMap<Identifier, Option<Document>>),
    VerifiedTokenActionWithDocument(Document),
    VerifiedTokenGroupActionWithDocument(GroupSumPower, Option<Document>),
    VerifiedTokenGroupActionWithTokenBalance(
        GroupSumPower,
        GroupActionStatus,
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json_safe_option_u64")
        )]
        Option<TokenAmount>,
    ),
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
    VerifiedAddressInfos(
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "json_safe_address_info_map")
        )]
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedIdentityFullWithAddressInfos(
        Identity,
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "json_safe_address_info_map")
        )]
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedIdentityWithAddressInfos(
        PartialIdentity,
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "json_safe_address_info_map")
        )]
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedAssetLockConsumed(StoredAssetLockInfo),
    VerifiedShieldedNullifiers(Vec<(Vec<u8>, bool)>),
    VerifiedShieldedNullifiersWithAddressInfos(
        Vec<(Vec<u8>, bool)>,
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "json_safe_address_info_map")
        )]
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    VerifiedShieldedNullifiersWithWithdrawalDocument(
        Vec<(Vec<u8>, bool)>,
        BTreeMap<Identifier, Option<Document>>,
    ),
    /// Returned by `ShieldFromAssetLock` when a `surplus_output` is set. Carries the consumed
    /// asset-lock info AND the proven balance of the surplus-output address, so a light/SDK
    /// client can cryptographically confirm the asset-lock surplus credit landed at the signed
    /// `surplus_output` address. The plain [`VerifiedAssetLockConsumed`] is still returned when
    /// no `surplus_output` is set.
    ///
    /// [`VerifiedAssetLockConsumed`]: StateTransitionProofResult::VerifiedAssetLockConsumed
    VerifiedAssetLockConsumedWithAddressInfos(
        StoredAssetLockInfo,
        BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    ),
    /// Returned by `IdentityCreateFromShieldedPool`. Carries the newly-created [`Identity`] AND the
    /// presence of each spent nullifier (`(nullifier_bytes, present)`), proven together in a single
    /// STRICT merged multi-root GroveDB proof. A light/SDK client can cryptographically confirm both
    /// that the identity was created and that the funding nullifiers were consumed.
    VerifiedIdentityWithShieldedNullifiers(Identity, Vec<(Vec<u8>, bool)>),
    /// Returned by `ContractUserModeration`: the target identity's status on the lists the
    /// moderation touched (contract id, identity id, one status per list proved) after the
    /// moderation. A ban proves both lists the contract keeps, since it also removes a
    /// suspension; an unban, a suspend and an unsuspend prove the one list they edit, and say
    /// nothing about the other.
    VerifiedContractModerationListStatuses(Identifier, Identifier, ContractModerationListStatuses),
    /// A contract fee claim's execution proof shows the pot it paid out (contract id, pot, the
    /// pot's last claim, the credits left in it) and the balance of every identity it paid,
    /// after the claim. A pot is paid out at most once per epoch, so within its epoch the last
    /// claim is this claim, and its claimant and block time say so.
    VerifiedContractFeeClaim(
        Identifier,
        ContractFeePot,
        ContractFeePotLastClaim,
        Credits,
        #[cfg_attr(
            feature = "json-conversion",
            serde(
                with = "crate::serialization::json::safe_integer_map::json_safe_identifier_u64_map"
            )
        )]
        BTreeMap<Identifier, Credits>,
    ),
    /// Returned by a `ContractUserModeration` that deletes a document: the record the removal
    /// left under the contract (contract id, document type name, document id, record). The
    /// proof shows the record, and the verifier checks that it names the transition's signer
    /// and carries the transition's reason. A document id is produced at most once, so the
    /// record is of this document and of no other.
    VerifiedContractDocumentRemoval(Identifier, String, Identifier, ContractDocumentRemoval),
}

/// The guarantee a verified state-transition proof establishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StateTransitionProofGuarantee {
    /// The proof binds the execution of this specific state transition:
    /// the verified values could only exist if the transition was applied.
    ExecutionProved,
    /// The proof authenticates a snapshot of the state the transition
    /// affects — keys derived from the transition, values as of the proof's
    /// block — but cannot bind them to the execution of this transition.
    /// Treat as a height-pinned snapshot, not as evidence of execution.
    AffectedState,
}

/// A verified state-transition proof: the result, the guarantee the proof
/// establishes for it, and, when the proof carries it, the credit balance of
/// the identity that owns the transition after it executed.
///
/// Some transition families (balance top-ups, credit transfers and
/// withdrawals, address funds movements, shields, no-history token
/// operations) produce proofs whose values cannot be bound to the execution
/// of one specific transition: the proof only authenticates the affected
/// keys' state at the committed block. The guarantee makes that distinction
/// part of the type so a snapshot cannot be mistaken for execution evidence.
///
/// From protocol version 14 the proof of an owned, fee-paying transition
/// (document and token batches, contract creates and updates, identity
/// updates and key limit updates, contract moderation) also carries the
/// owner's credit balance, read from the same state as the result. It is a
/// snapshot at the proof's block whatever the guarantee, and `None` for a
/// proof made at an earlier version or for a transition without an owner.
#[derive(Debug, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct StateTransitionProofOutcome {
    guarantee: StateTransitionProofGuarantee,
    result: StateTransitionProofResult,
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::serialization::json_safe_option_u64")
    )]
    owner_balance: Option<Credits>,
}

impl StateTransitionProofOutcome {
    /// An outcome whose proof binds the execution of the transition.
    pub fn execution_proved(result: StateTransitionProofResult) -> Self {
        Self {
            guarantee: StateTransitionProofGuarantee::ExecutionProved,
            result,
            owner_balance: None,
        }
    }

    /// An outcome whose proof only authenticates the affected state.
    pub fn affected_state(result: StateTransitionProofResult) -> Self {
        Self {
            guarantee: StateTransitionProofGuarantee::AffectedState,
            result,
            owner_balance: None,
        }
    }

    /// The same outcome carrying the owner's credit balance the proof showed.
    pub fn with_owner_balance(mut self, owner_balance: Option<Credits>) -> Self {
        self.owner_balance = owner_balance;
        self
    }

    /// The guarantee the proof establishes.
    pub fn guarantee(&self) -> StateTransitionProofGuarantee {
        self.guarantee
    }

    /// Whether the proof established that this specific transition executed.
    pub fn is_execution_proved(&self) -> bool {
        self.guarantee == StateTransitionProofGuarantee::ExecutionProved
    }

    /// The verified result, regardless of the guarantee.
    pub fn result(&self) -> &StateTransitionProofResult {
        &self.result
    }

    /// Consume the outcome, discarding the guarantee and the balance.
    pub fn into_result(self) -> StateTransitionProofResult {
        self.result
    }

    /// The credit balance of the transition's owner after it executed, when
    /// the proof carried it: a snapshot at the proof's block.
    pub fn owner_balance(&self) -> Option<Credits> {
        self.owner_balance
    }

    /// Consume the outcome into its guarantee, result and owner balance.
    pub fn into_parts(
        self,
    ) -> (
        StateTransitionProofGuarantee,
        StateTransitionProofResult,
        Option<Credits>,
    ) {
        (self.guarantee, self.result, self.owner_balance)
    }
}

/// Serde `with` module for `BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>`.
///
/// `AddressNonce` is `u32` (JS-safe); `Credits` is `u64` and must serialize as a
/// string in human-readable JSON above `MAX_SAFE_INTEGER`. A small wrapper tuple
/// carries `#[serde(with = "json_safe_u64")]` on the credits, preserving the
/// `[nonce, credits]` array wire-shape while making the value JS-safe. Binary /
/// `Value` paths stay native (the helper checks `is_human_readable`).
#[cfg(feature = "json-conversion")]
mod json_safe_address_info_map {
    use super::{AddressNonce, Credits, PlatformAddress};
    use serde::de::Deserializer;
    use serde::ser::{SerializeMap, Serializer};
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    /// The address-info map shape shared by several `StateTransitionProofResult`
    /// variants. Aliased to keep the helper signatures below under clippy's
    /// `type_complexity` threshold.
    type AddressInfoMap = BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>;

    #[derive(Serialize, Deserialize)]
    struct Entry(
        AddressNonce,
        #[serde(with = "crate::serialization::json_safe_u64")] Credits,
    );

    pub fn serialize<S: Serializer>(
        map: &AddressInfoMap,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            let wrapped = v.map(|(nonce, credits)| Entry(nonce, credits));
            s.serialize_entry(k, &wrapped)?;
        }
        s.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<AddressInfoMap, D::Error> {
        let raw: BTreeMap<PlatformAddress, Option<Entry>> = BTreeMap::deserialize(deserializer)?;
        Ok(raw
            .into_iter()
            .map(|(k, v)| (k, v.map(|Entry(nonce, credits)| (nonce, credits))))
            .collect())
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for StateTransitionProofResult {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for StateTransitionProofResult {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::{Identifier, Value};
    use serde_json::json;

    /// Non-default variant `VerifiedTokenBalance(id, amount)` with both
    /// tuple fields set so the wire-shape assertion catches silent variant
    /// flip / inner-zero on round-trip.
    fn fixture() -> StateTransitionProofResult {
        StateTransitionProofResult::VerifiedTokenBalance(
            Identifier::new([0xab; 32]),
            123_456_789u64,
        )
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `StateTransitionProofResult` uses serde external tagging (default,
        // no `#[serde(tag = ...)]`). Tuple variants serialize as
        // `{ "VariantName": [field0, field1, ...] }`. `Identifier` -> base58
        // string in JSON; `TokenAmount` is `u64` and JSON erases the size —
        // see the value-path assertion which uses `123_456_789u64`.
        assert_eq!(
            json,
            json!({
                "VerifiedTokenBalance": [
                    "CZ8YUVdk7znjrUmnb5n7kgySk9yRAsQDYmyCxzfSky9t",
                    123_456_789u64,
                ],
            })
        );
        let recovered = StateTransitionProofResult::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed `Identifier` and `U64` variants. We
        // construct the expected `Value::Map` by hand: `platform_value!{...}`
        // would convert the `Identifier` interpolation through Serialize
        // (correct) but the outer shape has only one (Text-keyed) entry whose
        // value is an Array of mixed-typed Values, so it's clearer to write
        // the literal Map.
        let expected = Value::Map(vec![(
            Value::Text("VerifiedTokenBalance".to_string()),
            Value::Array(vec![Value::Identifier([0xab; 32]), Value::U64(123_456_789)]),
        )]);
        assert_eq!(value, expected);
        let recovered = StateTransitionProofResult::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn verified_token_balance_large_amount_serializes_as_string() {
        use crate::serialization::JsonConvertible;
        // `TokenAmount` above `Number.MAX_SAFE_INTEGER` must serialize as a JSON
        // string (it sits in a tuple variant the macro can't reach).
        let original = StateTransitionProofResult::VerifiedTokenBalance(
            Identifier::new([0xab; 32]),
            9_007_199_254_740_993, // 2^53 + 1
        );
        let json = original.to_json().expect("to_json");
        assert_eq!(json["VerifiedTokenBalance"][1], json!("9007199254740993"));
        let recovered = StateTransitionProofResult::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    /// The outcome carries the owner's balance next to the result; past
    /// `Number.MAX_SAFE_INTEGER` it serializes as a string, and a proof made
    /// before protocol version 14 carries none.
    #[test]
    fn outcome_owner_balance_serializes_as_string_past_safe_integer() {
        let outcome = StateTransitionProofOutcome::execution_proved(fixture())
            .with_owner_balance(Some(9_007_199_254_740_993));
        let json = serde_json::to_value(&outcome).expect("to json");
        assert_eq!(json["owner_balance"], json!("9007199254740993"));
        assert_eq!(json["guarantee"], json!("ExecutionProved"));
        let recovered: StateTransitionProofOutcome =
            serde_json::from_value(json).expect("from json");
        assert_eq!(outcome, recovered);

        let without_balance = StateTransitionProofOutcome::affected_state(fixture());
        let json = serde_json::to_value(&without_balance).expect("to json");
        assert_eq!(json["owner_balance"], serde_json::Value::Null);
        let recovered: StateTransitionProofOutcome =
            serde_json::from_value(json).expect("from json");
        assert_eq!(without_balance, recovered);
    }

    #[test]
    fn verified_address_infos_large_credits_serialize_as_string() {
        use crate::serialization::{JsonConvertible, ValueConvertible};
        use std::collections::BTreeMap;
        // `Credits` (u64) nested in `BTreeMap<PlatformAddress, Option<(AddressNonce,
        // Credits)>>` must be JS-safe via the bespoke `json_safe_address_info_map`
        // helper, preserving the `[nonce, credits]` array wire-shape.
        let big_credits: Credits = 9_007_199_254_740_993; // 2^53 + 1
        let mut infos: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>> = BTreeMap::new();
        infos.insert(PlatformAddress::P2pkh([0x11; 20]), Some((7, big_credits)));
        infos.insert(PlatformAddress::P2sh([0x22; 20]), None);
        let original = StateTransitionProofResult::VerifiedAddressInfos(infos);

        // Human-readable JSON: credits string, nonce number, null preserved.
        let json = original.to_json().expect("to_json");
        let map = json["VerifiedAddressInfos"].as_object().expect("object");
        let non_null = map
            .values()
            .find(|v| !v.is_null())
            .expect("one populated entry");
        assert_eq!(non_null[0], json!(7));
        assert_eq!(non_null[1], json!("9007199254740993"));
        assert!(
            map.values().any(|v| v.is_null()),
            "the None entry must survive"
        );
        let recovered = StateTransitionProofResult::from_json(json).expect("from_json");
        assert_eq!(original, recovered);

        // Non-human-readable (platform_value): native u64, round-trips intact.
        let value = original.to_object().expect("to_object");
        let recovered = StateTransitionProofResult::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
