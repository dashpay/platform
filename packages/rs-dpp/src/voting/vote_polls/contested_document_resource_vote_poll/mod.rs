use crate::fee::Credits;
use crate::moderation_charter::is_charter_election;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::PlatformSerializable;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
pub struct ContestedDocumentResourceVotePoll {
    pub contract_id: Identifier,
    pub document_type_name: String,
    pub index_name: String,
    pub index_values: Vec<Value>,
}

impl fmt::Display for ContestedDocumentResourceVotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format the index_values as a comma-separated list
        let index_values_str: Vec<String> =
            self.index_values.iter().map(|v| v.to_string()).collect();
        write!(
            f,
            "ContestedDocumentResourceVotePoll {{ contract_id: {}, document_type_name: {}, index_name: {}, index_values: [{}] }}",
            self.contract_id,
            self.document_type_name,
            self.index_name,
            index_values_str.join(", ")
        )
    }
}

impl Default for ContestedDocumentResourceVotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll {
            contract_id: Default::default(),
            document_type_name: "".to_string(),
            index_name: "".to_string(),
            index_values: vec![],
        }
    }
}

impl ContestedDocumentResourceVotePoll {
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = self.serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.unique_id()
    }

    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
    }

    /// The prefunded voting balance a contender pays into this contest, see
    /// [`required_vote_resolution_fund`].
    pub fn required_vote_resolution_fund(&self, platform_version: &PlatformVersion) -> Credits {
        required_vote_resolution_fund(
            &self.contract_id,
            &self.document_type_name,
            platform_version,
        )
    }

    /// The prefunded voting balance a contender pays to join this contest while it holds
    /// `contenders` contenders, see [`required_vote_resolution_fund_to_join`].
    pub fn required_vote_resolution_fund_to_join(
        &self,
        contenders: u16,
        platform_version: &PlatformVersion,
    ) -> Credits {
        required_vote_resolution_fund_to_join(
            &self.contract_id,
            &self.document_type_name,
            contenders,
            platform_version,
        )
    }
}

/// The prefunded voting balance a contender pays into a contest on the contested index of
/// `document_type_name` in the contract `contract_id`: the moderation fund for a moderation
/// election (an `electedCharter` of the moderation charters contract), the contested document
/// fund for every other contest. Whatever the votes leave of it is released as processing fees
/// when the contest is cleaned up.
pub fn required_vote_resolution_fund(
    contract_id: &Identifier,
    document_type_name: &str,
    platform_version: &PlatformVersion,
) -> Credits {
    let fund_fees = &platform_version.fee_version.vote_resolution_fund_fees;
    if is_charter_election(contract_id, document_type_name) {
        fund_fees.moderation_vote_resolution_fund_required_amount
    } else {
        fund_fees.contested_document_vote_resolution_fund_required_amount
    }
}

/// The prefunded voting balance a contender pays to join a contest on the contested index of
/// `document_type_name` in the contract `contract_id` while the contest holds `contenders`
/// contenders: the contest's fund ([`required_vote_resolution_fund`]), doubled for every
/// `contested_document_contenders_per_fund_doubling` contenders the contest holds. From
/// protocol version 14 that is every 100, so the first 100 contenders pay the fund, the next 100
/// twice it, and the 901st to the 1,000th, the last a contest accepts, 512 times it. Before 14
/// every contender pays the fund.
///
/// This is the least a contender may pay; everything it pays goes to the contest's fund.
pub fn required_vote_resolution_fund_to_join(
    contract_id: &Identifier,
    document_type_name: &str,
    contenders: u16,
    platform_version: &PlatformVersion,
) -> Credits {
    let fund = required_vote_resolution_fund(contract_id, document_type_name, platform_version);
    let contenders_per_doubling = platform_version
        .fee_version
        .vote_resolution_fund_fees
        .contested_document_contenders_per_fund_doubling;
    if contenders_per_doubling == 0 {
        return fund;
    }
    let doublings = u32::from(contenders / contenders_per_doubling);
    2u64.checked_pow(doublings)
        .map_or(Credits::MAX, |multiplier| fund.saturating_mul(multiplier))
}

#[cfg(test)]
mod fund_to_join_tests {
    use super::*;
    use crate::moderation_charter::{
        ELECTED_CHARTER_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
    };

    const DASH: Credits = 100_000_000_000;

    fn dpns_fund_to_join(contenders: u16, platform_version: &PlatformVersion) -> Credits {
        required_vote_resolution_fund_to_join(
            &Identifier::new([0xC1; 32]),
            "domain",
            contenders,
            platform_version,
        )
    }

    /// From protocol version 14 the fund a contender pays doubles for every 100 contenders the
    /// contest holds, so filling a contest to its 1,000 contenders costs 10,230 Dash
    #[test]
    fn should_double_the_fund_for_every_100_contenders_a_contest_holds() {
        let platform_version = PlatformVersion::latest();

        for (contenders, fund) in [
            (0, DASH / 10),
            (99, DASH / 10),
            (100, DASH / 5),
            (199, DASH / 5),
            (200, 2 * DASH / 5),
            (899, 256 * DASH / 10),
            (900, 512 * DASH / 10),
            (999, 512 * DASH / 10),
        ] {
            assert_eq!(
                dpns_fund_to_join(contenders, platform_version),
                fund,
                "joining a contest holding {contenders} contenders"
            );
        }

        let fill = (0..1_000u16)
            .map(|contenders| dpns_fund_to_join(contenders, platform_version))
            .sum::<Credits>();
        assert_eq!(fill, 10_230 * DASH);

        // A moderation election doubles its own fund
        assert_eq!(
            required_vote_resolution_fund_to_join(
                &MODERATION_CHARTERS_CONTRACT_ID,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                999,
                platform_version,
            ),
            256 * DASH
        );

        // Past what 64 bits hold the fund saturates instead of overflowing
        assert_eq!(dpns_fund_to_join(u16::MAX, platform_version), Credits::MAX);
    }

    /// PROTOCOL_VERSION_13: every contender pays the same fund
    #[test]
    fn should_double_the_fund_for_every_100_contenders_a_contest_holds_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");

        for contenders in [0, 100, 999, u16::MAX] {
            assert_eq!(
                dpns_fund_to_join(contenders, platform_version),
                DASH / 5,
                "joining a contest holding {contenders} contenders"
            );
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default values per field (real contract id, named type/index, two
    /// index values) so the wire-shape assertion catches silent zero-out /
    /// vec-truncate on round-trip.
    fn fixture() -> ContestedDocumentResourceVotePoll {
        ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([0xc1; 32]),
            document_type_name: "preorder".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("alice".to_string()),
            ],
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // This is a plain struct (no `#[serde(tag)]`), so there is no
        // `$formatVersion` on the wire. `Identifier` -> base58 string.
        // `Value::Text` inside the array -> JSON string.
        assert_eq!(
            json,
            json!({
                "contractId": "E3M3d7sy8ZKivUGxBexL9wxE7ebqzGWFqkdeFMedCJFS",
                "documentTypeName": "preorder",
                "indexName": "parentNameAndLabel",
                "indexValues": ["dash", "alice"],
            })
        );
        let recovered = ContestedDocumentResourceVotePoll::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Interpolate the `Identifier` via `platform_value!` so Serialize emits
        // `Value::Identifier` (NOT `Value::Bytes32`). `index_values` is a
        // `Vec<Value>` round-tripped element-wise.
        let id = Identifier::new([0xc1; 32]);
        assert_eq!(
            value,
            platform_value!({
                "contractId": id,
                "documentTypeName": "preorder",
                "indexName": "parentNameAndLabel",
                "indexValues": ["dash", "alice"],
            })
        );
        let recovered = ContestedDocumentResourceVotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
