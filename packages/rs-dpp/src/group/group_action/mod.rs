pub mod v0;

use crate::data_contract::TokenContractPosition;
use crate::group::action_event::GroupActionEvent;
use crate::group::group_action::v0::GroupActionV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    PartialEq,
    PartialOrd,
    Clone,
    Eq,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
// Group actions reach two decoders. Drive reads the actions it stored itself
// with `deserialize_from_bytes_trusted_no_limit`, as v4.1 did: every stored
// action was valid when it was written, so a budget there could only refuse
// one of them. A client decodes them from GroveDB proof elements before the
// quorum signature is checked, and that untrusted decode runs under this
// budget.
//
// bincode's limit counts memory claimed, not bytes read. A map, a set or a
// vector of anything but bytes claims `len * size_of::<T>()` from its length
// prefix before reading an element; a string or a byte vector claims its
// length. Of the events a group can store, the conventions localizations map
// claims the most per encoded byte: 80 bytes per entry
// (`size_of::<(String, TokenConfigurationLocalization)>()` on a 64-bit target)
// against at least 13 encoded (a two-letter language code, two three-letter
// forms, their three length prefixes, the variant tag and the capitalization
// flag). The `SetPrices` and `Stepwise` maps claim 16 bytes per entry against
// at least 2 encoded for their first 251 keys and 4 after; strings, notes and
// identifiers claim what they encode. A stored action copies its containers
// from the transition that proposed it and is smaller than that transition
// (which also carries its signature and the token id), and a transition is at
// most `max_state_transition_size` bytes (20,480 at every protocol version),
// so no stored action claims more than 20,480 / 13 * 80, about 126,000 bytes.
// 262,144 (256 KiB) leaves more than twice that.
//
// The derive takes the budget as a literal, so it cannot live in
// `SystemLimits`. No consensus path reads it. It has to grow if
// `max_state_transition_size` ever does, and
// `should_decode_the_largest_storable_action_of_each_container_shape` fails
// until it does.
#[platform_serialize(limit = 262144, unversioned)] //versioned directly, no need to use platform_version
pub enum GroupAction {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(GroupActionV0),
}

pub trait GroupActionAccessors {
    fn contract_id(&self) -> Identifier;

    fn proposer_id(&self) -> Identifier;
    fn token_contract_position(&self) -> TokenContractPosition;
    fn event(&self) -> &GroupActionEvent;
}
impl GroupActionAccessors for GroupAction {
    fn contract_id(&self) -> Identifier {
        match self {
            GroupAction::V0(inner) => inner.contract_id(),
        }
    }

    fn proposer_id(&self) -> Identifier {
        match self {
            GroupAction::V0(inner) => inner.proposer_id(),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            GroupAction::V0(inner) => inner.token_contract_position(),
        }
    }

    fn event(&self) -> &GroupActionEvent {
        match self {
            GroupAction::V0(inner) => inner.event(),
        }
    }
}

// TODO(unification pass 2): add round-trip tests for GroupAction once we have an
// explicit fixture (GroupActionV0 has no Default — its `event: GroupActionEvent`
// field is itself a versioned enum without Default).

#[cfg(test)]
mod deserialize_limit_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use crate::serialization::{
        PlatformDeserializableTrusted, PlatformDeserializableUntrusted, PlatformSerializable,
    };
    use crate::tests::fixtures::get_token_conventions_with_localizations_fixture;
    use crate::tokens::token_event::TokenEvent;
    use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
    use platform_version::version::{PlatformVersion, PLATFORM_VERSIONS};
    use std::collections::BTreeMap;

    /// The budget `GroupAction` decoded under before it was sized from
    /// memory claims.
    const PREVIOUS_BUDGET: usize = 100_000;

    /// Builds an action holding the given number of entries in one container.
    type ActionBuilder = fn(usize) -> GroupAction;

    fn action_with_event(event: TokenEvent) -> GroupAction {
        GroupAction::V0(GroupActionV0 {
            contract_id: Identifier::new([1; 32]),
            proposer_id: Identifier::new([2; 32]),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(event),
        })
    }

    fn conventions_change(conventions: TokenConfigurationConvention) -> GroupAction {
        action_with_event(TokenEvent::ConfigUpdate(
            TokenConfigurationChangeItem::Conventions(conventions),
            None,
        ))
    }

    fn localizations_action(entries: usize) -> GroupAction {
        conventions_change(get_token_conventions_with_localizations_fixture(entries))
    }

    /// `entries` keys counted up from 0, each mapping to 0: the shortest
    /// encoding a `u64` to `u64` map with that many entries can have.
    fn shortest_u64_map(entries: usize) -> BTreeMap<u64, u64> {
        (0..entries as u64).map(|key| (key, 0)).collect()
    }

    fn set_prices_action(entries: usize) -> GroupAction {
        action_with_event(TokenEvent::ChangePriceForDirectPurchase(
            Some(TokenPricingSchedule::SetPrices(shortest_u64_map(entries))),
            None,
        ))
    }

    fn stepwise_action(entries: usize) -> GroupAction {
        action_with_event(TokenEvent::ConfigUpdate(
            TokenConfigurationChangeItem::PerpetualDistribution(Some(
                TokenPerpetualDistribution::V0(TokenPerpetualDistributionV0 {
                    distribution_type: RewardDistributionType::BlockBasedDistribution {
                        interval: 1,
                        function: DistributionFunction::Stepwise(shortest_u64_map(entries)),
                    },
                    distribution_recipient: TokenDistributionRecipient::ContractOwner,
                }),
            )),
            None,
        ))
    }

    fn note_action(length: usize) -> GroupAction {
        action_with_event(TokenEvent::Mint(
            1,
            Identifier::new([3; 32]),
            Some("a".repeat(length)),
        ))
    }

    fn encode(action: &GroupAction) -> Vec<u8> {
        action
            .serialize_to_bytes()
            .expect("expected to encode the action")
    }

    /// The action `build` makes with the most entries whose encoding still
    /// fits in `max_bytes`.
    fn largest_fitting(build: ActionBuilder, max_bytes: usize) -> GroupAction {
        // Every entry takes at least one byte, so `max_bytes` entries never fit.
        let (mut fits, mut too_many) = (0, max_bytes);
        while too_many - fits > 1 {
            let entries = (fits + too_many) / 2;
            if encode(&build(entries)).len() <= max_bytes {
                fits = entries;
            } else {
                too_many = entries;
            }
        }
        build(fits)
    }

    /// Whether `bytes` decode untrusted under the budget `GroupAction` had
    /// before.
    fn decodes_under_previous_budget(bytes: &[u8]) -> bool {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<PREVIOUS_BUDGET>();
        bincode::decode_from_slice_untrusted::<GroupAction, _>(bytes, config).is_ok()
    }

    /// `bytes` decode back to `action` untrusted under the budget, as a client
    /// reads a proof, and trusted with and without it.
    fn assert_decodes_with_every_decoder(bytes: &[u8], action: &GroupAction, shape: &str) {
        let untrusted = GroupAction::deserialize_from_bytes_untrusted(bytes)
            .unwrap_or_else(|e| panic!("{shape}: untrusted decode failed: {e}"));
        assert_eq!(&untrusted, action, "{shape}: untrusted decode");
        let trusted = GroupAction::deserialize_from_bytes_trusted(bytes)
            .unwrap_or_else(|e| panic!("{shape}: trusted decode failed: {e}"));
        assert_eq!(&trusted, action, "{shape}: trusted decode");
        let trusted_no_limit = GroupAction::deserialize_from_bytes_trusted_no_limit(bytes)
            .unwrap_or_else(|e| panic!("{shape}: trusted decode without a limit failed: {e}"));
        assert_eq!(
            &trusted_no_limit, action,
            "{shape}: trusted decode without a limit"
        );
    }

    /// A conventions change with 1,250 valid localizations encodes in 16,325
    /// bytes, but its map claims 1,250 * 80 = 100,000 bytes at its length
    /// prefix. The previous budget refused it, on Drive's reads as well as on
    /// proofs.
    #[test]
    fn should_decode_a_conventions_change_with_1250_localizations() {
        let conventions = get_token_conventions_with_localizations_fixture(1_250);
        assert!(conventions
            .validate_localizations(PlatformVersion::latest())
            .expect("expected to validate the localizations")
            .is_valid());
        let action = conventions_change(conventions);
        let bytes = encode(&action);
        assert_eq!(bytes.len(), 16_325);

        assert!(!decodes_under_previous_budget(&bytes));
        assert_decodes_with_every_decoder(&bytes, &action, "1,250 localizations");
    }

    /// A stored action copies its containers from the transition that
    /// proposed it and is smaller than that transition, which also carries its
    /// signature and the token id, so no stored action encodes in more than
    /// `max_state_transition_size` bytes.
    /// For each container a group action can hold, the one packing the most
    /// entries into that size decodes with every decoder. Only the
    /// localizations map claimed more than the previous budget.
    #[test]
    fn should_decode_the_largest_storable_action_of_each_container_shape() {
        let max_bytes = PLATFORM_VERSIONS
            .iter()
            .map(|platform_version| platform_version.system_limits.max_state_transition_size)
            .max()
            .expect("expected platform versions") as usize;
        let shapes: [(&str, ActionBuilder, bool); 4] = [
            ("localizations", localizations_action, false),
            ("SetPrices", set_prices_action, true),
            ("Stepwise", stepwise_action, true),
            ("note", note_action, true),
        ];
        for (shape, build, decoded_under_previous_budget) in shapes {
            let action = largest_fitting(build, max_bytes);
            let bytes = encode(&action);
            assert!(
                bytes.len() > max_bytes - 16,
                "{shape}: {} bytes",
                bytes.len()
            );
            assert_eq!(
                decodes_under_previous_budget(&bytes),
                decoded_under_previous_budget,
                "{shape}: previous budget"
            );
            assert_decodes_with_every_decoder(&bytes, &action, shape);
        }
    }

    /// A proof element is untrusted input: a localizations length prefix
    /// claiming a million entries is refused against the budget before any
    /// entry is read.
    #[test]
    fn should_refuse_a_localizations_length_prefix_beyond_the_budget() {
        // An empty map ends the encoding with its length (0), the decimals
        // and the absent note; keep everything before them.
        let empty = encode(&localizations_action(0));
        let mut buf = empty[..empty.len() - 3].to_vec();
        buf.extend_from_slice(
            &bincode::encode_to_vec(1_000_000u64, bincode::config::standard().with_big_endian())
                .unwrap(),
        );

        let err = GroupAction::deserialize_from_bytes_untrusted(&buf)
            .expect_err("oversized length prefix must be rejected");
        assert!(
            matches!(err, ProtocolError::MaxEncodedBytesReachedError { .. }),
            "unexpected error: {err}"
        );
    }

    /// A proof element is untrusted input: a note length prefix must be
    /// rejected against the byte budget before it sizes an allocation.
    #[test]
    fn rejects_note_length_prefix_beyond_budget_without_allocating() {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let mut buf = Vec::new();
        // GroupAction::V0, then GroupActionV0 { contract_id, proposer_id, position, event }
        buf.extend_from_slice(&bincode::encode_to_vec(0u32, config).unwrap());
        buf.extend_from_slice(&[0u8; 32]);
        buf.extend_from_slice(&[0u8; 32]);
        buf.extend_from_slice(&bincode::encode_to_vec(0u16, config).unwrap());
        // GroupActionEvent::TokenEvent(TokenEvent::Freeze(id, Some(note)))
        buf.extend_from_slice(&bincode::encode_to_vec(0u32, config).unwrap());
        buf.extend_from_slice(&bincode::encode_to_vec(2u32, config).unwrap());
        buf.extend_from_slice(&[0u8; 32]);
        buf.push(1);
        // note length prefix claiming 8 GB, with no bytes following it
        buf.extend_from_slice(&bincode::encode_to_vec(8_000_000_000u64, config).unwrap());

        let err = GroupAction::deserialize_from_bytes_untrusted(&buf)
            .expect_err("oversized length prefix must be rejected");
        assert!(
            matches!(err, ProtocolError::MaxEncodedBytesReachedError { .. }),
            "unexpected error: {err}"
        );
    }
}
