use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionTypeWithResolvedRecipient;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
use crate::fee::Credits;
use crate::prelude::{
    DataContract, DerivationEncryptionKeyIndex, IdentityNonce, RootEncryptionKeyIndex,
};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::fmt;

pub type TokenEventPublicNote = Option<String>;
pub type TokenEventSharedEncryptedNote = Option<SharedEncryptedNote>;
pub type TokenEventPersonalEncryptedNote = Option<(
    RootEncryptionKeyIndex,
    DerivationEncryptionKeyIndex,
    Vec<u8>,
)>;
use crate::serialization::PlatformSerializableWithPlatformVersion;
use crate::tokens::emergency_action::TokenEmergencyAction;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::tokens::SharedEncryptedNote;
use crate::ProtocolError;

/// Alias representing the identity that will receive tokens or other effects from a token operation.
pub type RecipientIdentifier = Identifier;

/// Alias representing the identity that will have tokens burned from their account.
pub type BurnFromIdentifier = Identifier;

/// Alias representing the identity performing a token purchase.
pub type PurchaserIdentifier = Identifier;

/// Alias representing the identity whose tokens are subject to freezing or unfreezing.
pub type FrozenIdentifier = Identifier;

/// Represents a recorded token-related operation for use in historical documents and group actions.
///
/// `TokenEvent` is designed to encapsulate a single logical token operation,
/// such as minting, burning, transferring, or freezing tokens. These events are typically:
///
/// - **Persisted as historical records** of state transitions, enabling auditability and tracking.
/// - **Used in group (multisig) actions**, where multiple identities collaborate to authorize complex transitions.
///
/// This enum includes rich metadata for each type of operation, such as optional notes (plaintext or encrypted),
/// involved identities, and amounts. It is **externally versioned** and marked as `unversioned` in platform serialization,
/// meaning each variant is self-contained without requiring version dispatching logic.
#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
// Custom `Serialize` / `Deserialize` below — `TokenEvent` is a flat enum
// with all-tuple variants. Internal tagging requires struct variants or
// newtype-of-named-struct, which doesn't apply to tuple shapes. The custom
// impl maps positional tuple fields to named JSON keys per variant, emits
// an internal `$type` discriminator (no `data` wrapper), and uses the
// `json_safe_u64`
// / `json_safe_option_encrypted_note` helpers for u64 + encrypted-note
// fields. Bincode `Encode` / `Decode` derives above are untouched —
// consensus binary path is unaffected.
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)]
pub enum TokenEvent {
    /// Event representing the minting of tokens to a recipient.
    ///
    /// - `TokenAmount`: The amount of tokens minted.
    /// - `RecipientIdentifier`: The identity receiving the minted tokens.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Mint(TokenAmount, RecipientIdentifier, TokenEventPublicNote),

    /// Event representing the burning of tokens, removing them from circulation.
    ///
    /// - `TokenAmount`: The amount of tokens burned.
    /// - `BurnFromIdentifier`: The account to burn from.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Burn(TokenAmount, BurnFromIdentifier, TokenEventPublicNote),

    /// Event representing freezing of tokens for a specific identity.
    ///
    /// - `FrozenIdentifier`: The identity whose tokens are frozen.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Freeze(FrozenIdentifier, TokenEventPublicNote),

    /// Event representing unfreezing of tokens for a specific identity.
    ///
    /// - `FrozenIdentifier`: The identity whose tokens are unfrozen.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Unfreeze(FrozenIdentifier, TokenEventPublicNote),

    /// Event representing destruction of tokens that were previously frozen.
    ///
    /// - `FrozenIdentifier`: The identity whose frozen tokens are destroyed.
    /// - `TokenAmount`: The amount of frozen tokens destroyed.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    DestroyFrozenFunds(FrozenIdentifier, TokenAmount, TokenEventPublicNote),

    /// Event representing a transfer of tokens from one identity to another.
    ///
    /// - `RecipientIdentifier`: The recipient of the tokens.
    /// - `TokenEventPublicNote`: Optional plaintext note.
    /// - `TokenEventSharedEncryptedNote`: Optional shared encrypted metadata (multi-party).
    /// - `TokenEventPersonalEncryptedNote`: Optional private encrypted metadata (recipient-only).
    /// - `TokenAmount`: The amount of tokens transferred.
    Transfer(
        RecipientIdentifier,
        TokenEventPublicNote,
        TokenEventSharedEncryptedNote,
        TokenEventPersonalEncryptedNote,
        TokenAmount,
    ),

    /// Event representing a claim of tokens from a distribution pool or source.
    ///
    /// - `TokenDistributionTypeWithResolvedRecipient`: Type and resolved recipient of the claim.
    /// - `TokenAmount`: The amount of tokens claimed.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Claim(
        TokenDistributionTypeWithResolvedRecipient,
        TokenAmount,
        TokenEventPublicNote,
    ),

    /// Event representing an emergency action taken on a token or identity.
    ///
    /// - `TokenEmergencyAction`: The type of emergency action performed.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    EmergencyAction(TokenEmergencyAction, TokenEventPublicNote),

    /// Event representing an update to the configuration of a token.
    ///
    /// - `TokenConfigurationChangeItem`: The configuration change that was applied.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    ConfigUpdate(TokenConfigurationChangeItem, TokenEventPublicNote),

    /// Event representing a change in the direct purchase price of a token.
    ///
    /// - `Option<TokenPricingSchedule>`: The new pricing schedule. `None` disables direct purchase.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    ChangePriceForDirectPurchase(Option<TokenPricingSchedule>, TokenEventPublicNote),

    /// Event representing the direct purchase of tokens by a user.
    ///
    /// - `TokenAmount`: The amount of tokens purchased.
    /// - `Credits`: The number of credits paid.
    DirectPurchase(TokenAmount, Credits),
}

// Manual impl because TokenEvent is a flat enum with u64-alias tuple variants
// (TokenAmount, Credits). `#[derive(JsonConvertible)]` would fail: it asserts inner
// variant types implement `JsonSafeFields`, but TokenAmount/Credits are u64 aliases
// which intentionally don't. The `#[json_safe_fields]` macro can't annotate tuple
// variant fields either. Safety is ensured by manual `impl JsonSafeFields` in
// safe_fields.rs — the developer takes responsibility for these fields.
#[cfg(feature = "json-conversion")]
impl JsonConvertible for TokenEvent {}

#[cfg(feature = "serde-conversion")]
impl serde::Serialize for TokenEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        // Wrappers that route through `json_safe_u64` and the encrypted-note
        // helper so large u64s stringify in JSON HR and Vec<u8> inside the
        // tuple becomes base64.
        struct SafeU64<'a>(&'a u64);
        impl<'a> serde::Serialize for SafeU64<'a> {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                crate::serialization::json::safe_integer::json_safe_u64::serialize(self.0, s)
            }
        }
        struct SafeOptEncNote<'a>(&'a Option<(u32, u32, Vec<u8>)>);
        impl<'a> serde::Serialize for SafeOptEncNote<'a> {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                crate::serialization::json::safe_integer::json_safe_option_encrypted_note::serialize(
                    self.0, s,
                )
            }
        }

        match self {
            TokenEvent::Mint(amount, recipient, note) => {
                let mut m = serializer.serialize_map(Some(4))?;
                m.serialize_entry("$type", "mint")?;
                m.serialize_entry("amount", &SafeU64(amount))?;
                m.serialize_entry("recipient", recipient)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::Burn(amount, from, note) => {
                let mut m = serializer.serialize_map(Some(4))?;
                m.serialize_entry("$type", "burn")?;
                m.serialize_entry("amount", &SafeU64(amount))?;
                m.serialize_entry("burnFromIdentifier", from)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::Freeze(frozen, note) => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("$type", "freeze")?;
                m.serialize_entry("frozenIdentifier", frozen)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::Unfreeze(frozen, note) => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("$type", "unfreeze")?;
                m.serialize_entry("frozenIdentifier", frozen)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::DestroyFrozenFunds(frozen, amount, note) => {
                let mut m = serializer.serialize_map(Some(4))?;
                m.serialize_entry("$type", "destroyFrozenFunds")?;
                m.serialize_entry("frozenIdentifier", frozen)?;
                m.serialize_entry("amount", &SafeU64(amount))?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::Transfer(recipient, note, shared, private, amount) => {
                let mut m = serializer.serialize_map(Some(6))?;
                m.serialize_entry("$type", "transfer")?;
                m.serialize_entry("recipient", recipient)?;
                m.serialize_entry("publicNote", note)?;
                m.serialize_entry("sharedEncryptedNote", &SafeOptEncNote(shared))?;
                m.serialize_entry("privateEncryptedNote", &SafeOptEncNote(private))?;
                m.serialize_entry("amount", &SafeU64(amount))?;
                m.end()
            }
            TokenEvent::Claim(distribution_type, amount, note) => {
                let mut m = serializer.serialize_map(Some(4))?;
                m.serialize_entry("$type", "claim")?;
                m.serialize_entry("distributionType", distribution_type)?;
                m.serialize_entry("amount", &SafeU64(amount))?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::EmergencyAction(action, note) => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("$type", "emergencyAction")?;
                m.serialize_entry("action", action)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::ConfigUpdate(change, note) => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("$type", "configUpdate")?;
                m.serialize_entry("configurationChange", change)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::ChangePriceForDirectPurchase(schedule, note) => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("$type", "changePriceForDirectPurchase")?;
                m.serialize_entry("pricingSchedule", schedule)?;
                m.serialize_entry("publicNote", note)?;
                m.end()
            }
            TokenEvent::DirectPurchase(amount, credits) => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("$type", "directPurchase")?;
                m.serialize_entry("amount", &SafeU64(amount))?;
                m.serialize_entry("credits", &SafeU64(credits))?;
                m.end()
            }
        }
    }
}

#[cfg(feature = "serde-conversion")]
impl<'de> serde::Deserialize<'de> for TokenEvent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{Error, IgnoredAny, MapAccess, Visitor};

        // Newtype wrappers that route u64 / encrypted-note deserialization
        // through the json_safe helpers (accept both numeric and string forms
        // for u64; accept either tuple-with-base64 or tuple-with-bytes).
        #[derive(serde::Deserialize)]
        #[serde(transparent)]
        struct U64Safe(
            #[serde(with = "crate::serialization::json::safe_integer::json_safe_u64")] u64,
        );
        #[derive(serde::Deserialize)]
        #[serde(transparent)]
        struct OptEncNote(
            #[serde(
                with = "crate::serialization::json::safe_integer::json_safe_option_encrypted_note"
            )]
            Option<(u32, u32, Vec<u8>)>,
        );

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = TokenEvent;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("TokenEvent as a map with `$type` discriminator + variant fields")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TokenEvent, A::Error> {
                let mut ty: Option<String> = None;
                let mut amount: Option<u64> = None;
                let mut credits: Option<u64> = None;
                let mut recipient: Option<Identifier> = None;
                let mut burn_from: Option<Identifier> = None;
                let mut frozen: Option<Identifier> = None;
                let mut public_note: Option<String> = None;
                let mut shared_note: Option<(u32, u32, Vec<u8>)> = None;
                let mut private_note: Option<(u32, u32, Vec<u8>)> = None;
                let mut distribution_type: Option<TokenDistributionTypeWithResolvedRecipient> =
                    None;
                let mut action: Option<TokenEmergencyAction> = None;
                let mut configuration_change: Option<TokenConfigurationChangeItem> = None;
                let mut pricing_schedule: Option<TokenPricingSchedule> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => ty = Some(map.next_value()?),
                        "amount" => amount = Some(map.next_value::<U64Safe>()?.0),
                        "credits" => credits = Some(map.next_value::<U64Safe>()?.0),
                        "recipient" => recipient = Some(map.next_value()?),
                        "burnFromIdentifier" => burn_from = Some(map.next_value()?),
                        "frozenIdentifier" => frozen = Some(map.next_value()?),
                        "publicNote" => public_note = map.next_value()?,
                        "sharedEncryptedNote" => {
                            shared_note = map.next_value::<OptEncNote>()?.0;
                        }
                        "privateEncryptedNote" => {
                            private_note = map.next_value::<OptEncNote>()?.0;
                        }
                        "distributionType" => distribution_type = Some(map.next_value()?),
                        "action" => action = Some(map.next_value()?),
                        "configurationChange" => configuration_change = Some(map.next_value()?),
                        "pricingSchedule" => pricing_schedule = map.next_value()?,
                        _ => {
                            let _: IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let ty = ty.ok_or_else(|| A::Error::missing_field("$type"))?;
                match ty.as_str() {
                    "mint" => Ok(TokenEvent::Mint(
                        amount.ok_or_else(|| A::Error::missing_field("amount"))?,
                        recipient.ok_or_else(|| A::Error::missing_field("recipient"))?,
                        public_note,
                    )),
                    "burn" => Ok(TokenEvent::Burn(
                        amount.ok_or_else(|| A::Error::missing_field("amount"))?,
                        burn_from.ok_or_else(|| A::Error::missing_field("burnFromIdentifier"))?,
                        public_note,
                    )),
                    "freeze" => Ok(TokenEvent::Freeze(
                        frozen.ok_or_else(|| A::Error::missing_field("frozenIdentifier"))?,
                        public_note,
                    )),
                    "unfreeze" => Ok(TokenEvent::Unfreeze(
                        frozen.ok_or_else(|| A::Error::missing_field("frozenIdentifier"))?,
                        public_note,
                    )),
                    "destroyFrozenFunds" => Ok(TokenEvent::DestroyFrozenFunds(
                        frozen.ok_or_else(|| A::Error::missing_field("frozenIdentifier"))?,
                        amount.ok_or_else(|| A::Error::missing_field("amount"))?,
                        public_note,
                    )),
                    "transfer" => Ok(TokenEvent::Transfer(
                        recipient.ok_or_else(|| A::Error::missing_field("recipient"))?,
                        public_note,
                        shared_note,
                        private_note,
                        amount.ok_or_else(|| A::Error::missing_field("amount"))?,
                    )),
                    "claim" => Ok(TokenEvent::Claim(
                        distribution_type
                            .ok_or_else(|| A::Error::missing_field("distributionType"))?,
                        amount.ok_or_else(|| A::Error::missing_field("amount"))?,
                        public_note,
                    )),
                    "emergencyAction" => Ok(TokenEvent::EmergencyAction(
                        action.ok_or_else(|| A::Error::missing_field("action"))?,
                        public_note,
                    )),
                    "configUpdate" => Ok(TokenEvent::ConfigUpdate(
                        configuration_change
                            .ok_or_else(|| A::Error::missing_field("configurationChange"))?,
                        public_note,
                    )),
                    "changePriceForDirectPurchase" => Ok(TokenEvent::ChangePriceForDirectPurchase(
                        pricing_schedule,
                        public_note,
                    )),
                    "directPurchase" => Ok(TokenEvent::DirectPurchase(
                        amount.ok_or_else(|| A::Error::missing_field("amount"))?,
                        credits.ok_or_else(|| A::Error::missing_field("credits"))?,
                    )),
                    other => Err(A::Error::unknown_variant(
                        other,
                        &[
                            "mint",
                            "burn",
                            "freeze",
                            "unfreeze",
                            "destroyFrozenFunds",
                            "transfer",
                            "claim",
                            "emergencyAction",
                            "configUpdate",
                            "changePriceForDirectPurchase",
                            "directPurchase",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `TokenEvent` has a custom `Serialize` / `Deserialize` impl emitting an
    // internally-tagged flat shape: each variant maps positional tuple fields
    // to named JSON keys (`amount` / `recipient` / `publicNote` / etc.).
    // Round-trip covers a representative sample: `Mint` (3-tuple), `Freeze`
    // (2-tuple including null note), `DirectPurchase` (2-tuple of u64 aliases).

    pub(crate) fn mint_fixture() -> TokenEvent {
        TokenEvent::Mint(
            5_000,
            Identifier::new([0xa1; 32]),
            Some("genesis mint".to_string()),
        )
    }

    #[test]
    fn json_round_trip_mint() {
        use crate::serialization::JsonConvertible;
        let original = mint_fixture();
        let json = original.to_json().expect("to_json");
        // `TokenAmount` (u64) → `json_safe_u64` (number for small values,
        // string above MAX_SAFE_INTEGER). `Identifier` → base58 string in HR.
        assert_eq!(
            json,
            json!({
                "$type": "mint",
                "amount": 5_000,
                "recipient": "Bswb3UyeD1pUTaGiE6WvqwFpJZsQSEY1xhJePCDTHdvp",
                "publicNote": "genesis mint",
            })
        );
        let recovered = TokenEvent::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_freeze_no_note() {
        use crate::serialization::JsonConvertible;
        let original = TokenEvent::Freeze(Identifier::new([0xb2; 32]), None);
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$type": "freeze",
                "frozenIdentifier": "D2ZcUbtpG5sKq7XLeB4YnpNnTGSptKCxTddoNeydzJQq",
                "publicNote": null,
            })
        );
        let recovered = TokenEvent::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_direct_purchase() {
        use crate::serialization::JsonConvertible;
        let original = TokenEvent::DirectPurchase(100, 5_000);
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$type": "directPurchase",
                "amount": 100,
                "credits": 5_000,
            })
        );
        let recovered = TokenEvent::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_mint() {
        use crate::serialization::ValueConvertible;
        let original = mint_fixture();
        let value = original.to_object().expect("to_object");
        // `TokenAmount` is `u64` → `Value::U64`. Identifier → `Value::Identifier`.
        assert_eq!(
            value,
            platform_value!({
                "$type": "mint",
                "amount": 5_000u64,
                "recipient": Identifier::new([0xa1; 32]),
                "publicNote": "genesis mint",
            })
        );
        let recovered = TokenEvent::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_freeze_no_note() {
        use crate::serialization::ValueConvertible;
        let original = TokenEvent::Freeze(Identifier::new([0xb2; 32]), None);
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$type": "freeze",
                "frozenIdentifier": Identifier::new([0xb2; 32]),
                "publicNote": null,
            })
        );
        let recovered = TokenEvent::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_direct_purchase() {
        use crate::serialization::ValueConvertible;
        let original = TokenEvent::DirectPurchase(100, 5_000);
        let value = original.to_object().expect("to_object");
        // `TokenAmount` and `Credits` are both `u64`.
        assert_eq!(
            value,
            platform_value!({
                "$type": "directPurchase",
                "amount": 100u64,
                "credits": 5_000u64,
            })
        );
        let recovered = TokenEvent::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

impl fmt::Display for TokenEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenEvent::Mint(amount, recipient, note) => {
                write!(f, "Mint {} to {}{}", amount, recipient, format_note(note))
            }
            TokenEvent::Burn(amount, burn_from_identifier, note) => {
                write!(
                    f,
                    "Burn {} from {}{}",
                    amount,
                    burn_from_identifier,
                    format_note(note)
                )
            }
            TokenEvent::Freeze(identity, note) => {
                write!(f, "Freeze {}{}", identity, format_note(note))
            }
            TokenEvent::Unfreeze(identity, note) => {
                write!(f, "Unfreeze {}{}", identity, format_note(note))
            }
            TokenEvent::DestroyFrozenFunds(identity, amount, note) => {
                write!(
                    f,
                    "Destroy {} frozen from {}{}",
                    amount,
                    identity,
                    format_note(note)
                )
            }
            TokenEvent::Transfer(to, note, _, _, amount) => {
                write!(f, "Transfer {} to {}{}", amount, to, format_note(note))
            }
            TokenEvent::Claim(recipient, amount, note) => {
                write!(
                    f,
                    "Claim {} by {:?}{}",
                    amount,
                    recipient,
                    format_note(note)
                )
            }
            TokenEvent::EmergencyAction(action, note) => {
                write!(f, "Emergency action {:?}{}", action, format_note(note))
            }
            TokenEvent::ConfigUpdate(change, note) => {
                write!(f, "Configuration update {:?}{}", change, format_note(note))
            }
            TokenEvent::ChangePriceForDirectPurchase(schedule, note) => match schedule {
                Some(s) => write!(f, "Change price schedule to {:?}{}", s, format_note(note)),
                None => write!(f, "Disable direct purchase{}", format_note(note)),
            },
            TokenEvent::DirectPurchase(amount, credits) => {
                write!(f, "Direct purchase of {} for {} credits", amount, credits)
            }
        }
    }
}

fn format_note(note: &Option<String>) -> String {
    match note {
        Some(n) => format!(" (note: {})", n),
        None => String::new(),
    }
}

impl TokenEvent {
    pub fn associated_document_type_name(&self) -> &str {
        match self {
            TokenEvent::Mint(..) => "mint",
            TokenEvent::Burn(..) => "burn",
            TokenEvent::Freeze(..) => "freeze",
            TokenEvent::Unfreeze(..) => "unfreeze",
            TokenEvent::DestroyFrozenFunds(..) => "destroyFrozenFunds",
            TokenEvent::Transfer(..) => "transfer",
            TokenEvent::Claim(..) => "claim",
            TokenEvent::EmergencyAction(..) => "emergencyAction",
            TokenEvent::ConfigUpdate(..) => "configUpdate",
            TokenEvent::DirectPurchase(..) => "directPurchase",
            TokenEvent::ChangePriceForDirectPurchase(..) => "directPricing",
        }
    }

    /// Returns a reference to the public note if the variant includes one.
    pub fn public_note(&self) -> Option<&str> {
        match self {
            TokenEvent::Mint(_, _, Some(note))
            | TokenEvent::Burn(_, _, Some(note))
            | TokenEvent::Freeze(_, Some(note))
            | TokenEvent::Unfreeze(_, Some(note))
            | TokenEvent::DestroyFrozenFunds(_, _, Some(note))
            | TokenEvent::Transfer(_, Some(note), _, _, _)
            | TokenEvent::Claim(_, _, Some(note))
            | TokenEvent::EmergencyAction(_, Some(note))
            | TokenEvent::ConfigUpdate(_, Some(note))
            | TokenEvent::ChangePriceForDirectPurchase(_, Some(note)) => Some(note),
            _ => None,
        }
    }

    pub fn associated_document_type<'a>(
        &self,
        token_history_contract: &'a DataContract,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError> {
        Ok(token_history_contract.document_type_for_name(self.associated_document_type_name())?)
    }

    pub fn build_historical_document_owned(
        self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        let document_id = Document::generate_document_id_v0(
            &token_id,
            &owner_id,
            format!("history_{}", self.associated_document_type_name()).as_str(),
            owner_nonce.to_be_bytes().as_slice(),
        );

        let properties = match self {
            TokenEvent::Mint(mint_amount, recipient_id, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("recipientId".to_string(), recipient_id.into()),
                    ("amount".to_string(), mint_amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Burn(burn_amount, burn_from_identifier, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("burnFromId".to_string(), burn_from_identifier.into()),
                    ("amount".to_string(), burn_amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Transfer(
                to,
                public_note,
                token_event_shared_encrypted_note,
                token_event_personal_encrypted_note,
                amount,
            ) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("amount".to_string(), amount.into()),
                    ("toIdentityId".to_string(), to.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("publicNote".to_string(), note.into());
                }
                if let Some((sender_key_index, recipient_key_index, note)) =
                    token_event_shared_encrypted_note
                {
                    properties.insert("encryptedSharedNote".to_string(), note.into());
                    properties.insert("senderKeyIndex".to_string(), sender_key_index.into());
                    properties.insert("recipientKeyIndex".to_string(), recipient_key_index.into());
                }

                if let Some((root_encryption_key_index, derivation_encryption_key_index, note)) =
                    token_event_personal_encrypted_note
                {
                    properties.insert("encryptedPersonalNote".to_string(), note.into());
                    properties.insert(
                        "rootEncryptionKeyIndex".to_string(),
                        root_encryption_key_index.into(),
                    );
                    properties.insert(
                        "derivationEncryptionKeyIndex".to_string(),
                        derivation_encryption_key_index.into(),
                    );
                }
                properties
            }
            TokenEvent::Freeze(frozen_identity_id, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("frozenIdentityId".to_string(), frozen_identity_id.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Unfreeze(frozen_identity_id, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("frozenIdentityId".to_string(), frozen_identity_id.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::DestroyFrozenFunds(frozen_identity_id, amount, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("frozenIdentityId".to_string(), frozen_identity_id.into()),
                    ("destroyedAmount".to_string(), amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::EmergencyAction(action, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("action".to_string(), (action as u8).into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::ConfigUpdate(configuration_change_item, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    (
                        "changeItemType".to_string(),
                        configuration_change_item.u8_item_index().into(),
                    ),
                    (
                        "changeItem".to_string(),
                        configuration_change_item
                            .serialize_consume_to_bytes_with_platform_version(platform_version)?
                            .into(),
                    ),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Claim(recipient, amount, public_note) => {
                let (recipient_type, recipient_id, distribution_type) = match recipient {
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(identifier) => {
                        (1u8, identifier, 0u8)
                    }
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(identifier),
                    ) => (0, identifier, 1),
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Identity(identifier),
                    ) => (1, identifier, 1),
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Evonode(identifier),
                    ) => (2, identifier, 1),
                };

                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("recipientType".to_string(), recipient_type.into()),
                    ("recipientId".to_string(), recipient_id.into()),
                    ("distributionType".to_string(), distribution_type.into()),
                    ("amount".to_string(), amount.into()),
                ]);

                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::ChangePriceForDirectPurchase(price, note) => {
                let mut properties = BTreeMap::from([("tokenId".to_string(), token_id.into())]);

                if let Some(price_schedule) = price {
                    properties.insert(
                        "priceSchedule".to_string(),
                        price_schedule
                            .serialize_consume_to_bytes_with_platform_version(platform_version)?
                            .into(),
                    );
                }

                if let Some(note) = note {
                    properties.insert("note".to_string(), note.into());
                }

                properties
            }
            TokenEvent::DirectPurchase(amount, total_cost) => BTreeMap::from([
                ("tokenId".to_string(), token_id.into()),
                ("tokenAmount".to_string(), amount.into()),
                ("purchaseCost".to_string(), total_cost.into()),
            ]),
        };

        let document: Document = DocumentV0 {
            contract_version: None,
            id: document_id,
            owner_id,
            properties,
            revision: None,
            created_at: Some(block_info.time_ms),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: Some(block_info.height),
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_id() -> Identifier {
        Identifier::from([1u8; 32])
    }

    fn test_id_2() -> Identifier {
        Identifier::from([2u8; 32])
    }

    // ---- associated_document_type_name tests ----

    #[test]
    fn associated_name_mint() {
        let event = TokenEvent::Mint(0, test_id(), None);
        assert_eq!(event.associated_document_type_name(), "mint");
    }

    #[test]
    fn associated_name_burn() {
        let event = TokenEvent::Burn(0, test_id(), None);
        assert_eq!(event.associated_document_type_name(), "burn");
    }

    #[test]
    fn associated_name_freeze() {
        let event = TokenEvent::Freeze(test_id(), None);
        assert_eq!(event.associated_document_type_name(), "freeze");
    }

    #[test]
    fn associated_name_unfreeze() {
        let event = TokenEvent::Unfreeze(test_id(), None);
        assert_eq!(event.associated_document_type_name(), "unfreeze");
    }

    #[test]
    fn associated_name_destroy_frozen_funds() {
        let event = TokenEvent::DestroyFrozenFunds(test_id(), 0, None);
        assert_eq!(event.associated_document_type_name(), "destroyFrozenFunds");
    }

    #[test]
    fn associated_name_transfer() {
        let event = TokenEvent::Transfer(test_id(), None, None, None, 0);
        assert_eq!(event.associated_document_type_name(), "transfer");
    }

    #[test]
    fn associated_name_claim() {
        let recipient = TokenDistributionTypeWithResolvedRecipient::PreProgrammed(test_id());
        let event = TokenEvent::Claim(recipient, 0, None);
        assert_eq!(event.associated_document_type_name(), "claim");
    }

    #[test]
    fn associated_name_emergency_action() {
        let event = TokenEvent::EmergencyAction(TokenEmergencyAction::Pause, None);
        assert_eq!(event.associated_document_type_name(), "emergencyAction");
    }

    #[test]
    fn associated_name_config_update() {
        let event = TokenEvent::ConfigUpdate(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            None,
        );
        assert_eq!(event.associated_document_type_name(), "configUpdate");
    }

    #[test]
    fn associated_name_direct_purchase() {
        let event = TokenEvent::DirectPurchase(0, 0);
        assert_eq!(event.associated_document_type_name(), "directPurchase");
    }

    #[test]
    fn associated_name_change_price() {
        let event = TokenEvent::ChangePriceForDirectPurchase(None, None);
        assert_eq!(event.associated_document_type_name(), "directPricing");
    }

    // ---- all associated_document_type_name values are distinct ----

    #[test]
    fn all_document_type_names_are_unique() {
        let recipient = TokenDistributionTypeWithResolvedRecipient::PreProgrammed(test_id());
        let events: Vec<TokenEvent> = vec![
            TokenEvent::Mint(0, test_id(), None),
            TokenEvent::Burn(0, test_id(), None),
            TokenEvent::Freeze(test_id(), None),
            TokenEvent::Unfreeze(test_id(), None),
            TokenEvent::DestroyFrozenFunds(test_id(), 0, None),
            TokenEvent::Transfer(test_id(), None, None, None, 0),
            TokenEvent::Claim(recipient, 0, None),
            TokenEvent::EmergencyAction(TokenEmergencyAction::Pause, None),
            TokenEvent::ConfigUpdate(
                TokenConfigurationChangeItem::TokenConfigurationNoChange,
                None,
            ),
            TokenEvent::DirectPurchase(0, 0),
            TokenEvent::ChangePriceForDirectPurchase(None, None),
        ];
        let names: Vec<&str> = events
            .iter()
            .map(|e| e.associated_document_type_name())
            .collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            names.len(),
            unique.len(),
            "Duplicate document type names found"
        );
    }

    // ---- format_note helper ----

    #[test]
    fn format_note_none_returns_empty() {
        assert_eq!(format_note(&None), "");
    }

    #[test]
    fn format_note_some_returns_formatted() {
        assert_eq!(format_note(&Some("hello".to_string())), " (note: hello)");
    }
}
