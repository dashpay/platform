/// Marker trait proving a type's u64/i64 fields are protected for JS-safe JSON serialization.
///
/// # How it works
///
/// The `#[json_safe_fields]` attribute macro auto-implements this trait for annotated types.
/// The `#[derive(JsonConvertible)]` macro implements it for versioned enums and asserts
/// that all inner variant types also implement it.
///
/// # Compile errors
///
/// If you see `` the trait `JsonSafeFields` is not satisfied ``, it means a field type
/// in an annotated struct doesn't implement this trait. Fix it by one of:
///
/// - **Struct with u64 fields**: add `#[cfg_attr(feature = "json-conversion", json_safe_fields)]`
/// - **Simple enum/struct without u64**: add `impl JsonSafeFields for MyType {}` below
/// - **`BTreeMap<K, u64>` field**: add `#[serde(with = "json_safe_generic_u64_value_map")]`
/// - **New `type Foo = u64` alias**: add it to `U64_ALIASES` in the proc macro crate
///
/// # Why u64/i64 don't implement this trait
///
/// A bare `u64` is NOT JS-safe. Safety comes from `#[serde(with = "json_safe_u64")]`
/// applied at the field level. By excluding u64/i64 from this trait, the compiler
/// catches unprotected type aliases (`type Foo = u64`) and containers (`Vec<u64>`)
/// at compile time.
pub trait JsonSafeFields {}

// --- Primitive types (no u64/i64 risk) ---
// NOTE: u64 and i64 are intentionally excluded — see trait doc above.

impl JsonSafeFields for bool {}
impl JsonSafeFields for u8 {}
impl JsonSafeFields for u16 {}
impl JsonSafeFields for u32 {}
impl JsonSafeFields for i8 {}
impl JsonSafeFields for i16 {}
impl JsonSafeFields for i32 {}
impl JsonSafeFields for f32 {}
impl JsonSafeFields for f64 {}
impl JsonSafeFields for usize {}
impl JsonSafeFields for isize {}
impl JsonSafeFields for char {}
impl JsonSafeFields for String {}
impl JsonSafeFields for () {}
impl<T: JsonSafeFields> JsonSafeFields for &T {}
impl<const N: usize> JsonSafeFields for [u8; N] {}

// --- Standard collections (safe if inner types are safe) ---
// NOTE: Vec<u64>, BTreeMap<K, u64> etc. will NOT satisfy these bounds because
// u64 doesn't implement JsonSafeFields. Fields with such types must use a
// custom `#[serde(with = "...")]` module (e.g., json_safe_u64_u64_map).

impl<T: JsonSafeFields> JsonSafeFields for Vec<T> {}
impl<T: JsonSafeFields> JsonSafeFields for Option<T> {}
impl<T: JsonSafeFields> JsonSafeFields for Box<T> {}
impl<K: JsonSafeFields, V: JsonSafeFields> JsonSafeFields for std::collections::BTreeMap<K, V> {}
impl<T: JsonSafeFields> JsonSafeFields for std::collections::BTreeSet<T> {}
impl<K: JsonSafeFields, V: JsonSafeFields> JsonSafeFields for std::collections::HashMap<K, V> {}
impl<T: JsonSafeFields> JsonSafeFields for std::collections::HashSet<T> {}

// --- Platform types (external, don't contain unprotected u64/i64) ---

impl JsonSafeFields for platform_value::Identifier {}
impl JsonSafeFields for platform_value::BinaryData {}
impl JsonSafeFields for platform_value::Bytes20 {}
impl JsonSafeFields for platform_value::Bytes32 {}
impl JsonSafeFields for platform_value::Bytes36 {}
impl JsonSafeFields for platform_value::Value {}
impl JsonSafeFields for platform_value::string_encoding::Encoding {}

// --- External crate types ---

impl JsonSafeFields for dashcore::OutPoint {}

// --- rs-dpp types that don't contain u64/i64 or have their own safe handling ---
// Add new entries here when a json_safe_fields-annotated struct has a field whose
// type is a simple enum/struct without u64/i64. The compiler will tell you which
// type is missing via a `JsonSafeFields is not satisfied` error.

impl JsonSafeFields
    for crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers
{
}
impl JsonSafeFields
    for crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements
{
}
impl JsonSafeFields
    for crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode
{
}
impl JsonSafeFields for crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient {}
impl JsonSafeFields for crate::identity::Purpose {}
impl JsonSafeFields for crate::identity::SecurityLevel {}
impl JsonSafeFields for crate::identity::KeyType {}
impl JsonSafeFields for crate::block::epoch::Epoch {}
impl JsonSafeFields for crate::identity::identity_public_key::IdentityPublicKey {}
impl JsonSafeFields for crate::identity::state_transition::asset_lock_proof::AssetLockProof {}
impl JsonSafeFields for crate::address_funds::PlatformAddress {}
impl JsonSafeFields for crate::address_funds::AddressFundsFeeStrategy {}
// `AddressWitness` is verified via `#[json_safe_fields]` on the type itself
// (named-field variants of `BinaryData`), so no manual marker is needed here.
impl JsonSafeFields for crate::withdrawal::Pooling {}
impl JsonSafeFields for crate::identity::core_script::CoreScript {}
impl JsonSafeFields for crate::voting::votes::Vote {}
// `DocumentBaseTransition` wraps `DocumentBaseTransitionV0` / `V1`, both of
// which are `#[json_safe_fields]`-annotated, so the wrapper enum is safe by
// induction: every u64 inside is protected by `json_safe_u64`.
impl JsonSafeFields
    for crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition
{
}
// `TokenPaymentInfo` (v0 wrapper) — V0 is `#[json_safe_fields]`-annotated.
impl JsonSafeFields for crate::tokens::token_payment_info::TokenPaymentInfo {}
// `GasFeesPaidBy` is a unit-variant enum (no u64).
impl JsonSafeFields for crate::tokens::gas_fees_paid_by::GasFeesPaidBy {}
// `GroupStateTransitionInfo` is verified via `#[json_safe_fields]` on the type
// itself (named `u16` / `Identifier` / `bool` fields) — no manual marker needed.
// `TokenBaseTransition` wraps `TokenBaseTransitionV0` which is
// `#[json_safe_fields]`-annotated, so the wrapper is safe by induction.
impl JsonSafeFields
    for crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition
{
}
// BatchTransition family wrappers — each variant's outer enum is itself
// safe by induction (every V0 inner is `#[json_safe_fields]`-annotated;
// the outer-enum manual `impl JsonConvertible` doesn't auto-impl
// JsonSafeFields, so we declare it explicitly here).
impl JsonSafeFields
    for crate::state_transition::batch_transition::batched_transition::DocumentTransition
{
}
impl JsonSafeFields
    for crate::state_transition::batch_transition::batched_transition::TokenTransition
{
}
impl JsonSafeFields
    for crate::state_transition::batch_transition::batched_transition::BatchedTransition
{
}
impl JsonSafeFields for crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice {}
impl JsonSafeFields for crate::group::action_event::GroupActionEvent {}
// TokenEvent contains u64 aliases (TokenAmount, Credits) in tuple variants that
// `#[json_safe_fields]` can't auto-annotate. Developer takes responsibility for
// JS-safe serialization of these fields. See token_event.rs for details.
impl JsonSafeFields for crate::tokens::token_event::TokenEvent {}
// `TokenEmergencyAction` is a unit-variant enum (Pause / Resume).
impl JsonSafeFields for crate::tokens::emergency_action::TokenEmergencyAction {}
// `TokenDistributionType` is a unit-variant enum.
impl JsonSafeFields
    for crate::data_contract::associated_token::token_distribution_key::TokenDistributionType
{
}
// `TokenPricingSchedule` has tuple variants holding `Credits` (u64) and
// `BTreeMap<TokenAmount, Credits>`. `#[json_safe_fields]` can't auto-annotate
// variant-internal u64s, so it serializes through an internally-`$type`-tagged
// `Repr` that routes both through `json_safe_u64` / `json_safe_u64_u64_map` —
// this marker is therefore truthful, not a bare escape hatch.
impl JsonSafeFields for crate::tokens::token_pricing_schedule::TokenPricingSchedule {}
// `TokenConfigurationChangeItem` has tuple variants with `Option<TokenAmount>`
// and `Option<GroupContractPosition>` (u64-shaped). Same escape-hatch pattern.
impl JsonSafeFields
    for crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem
{
}
// `RewardDistributionMoment` carries `BlockHeight`/`TimestampMillis` (u64) in
// tuple variants. Unlike the bare escape-hatches above, its u64 fields are
// *actually* JS-safe: `#[serde(with = "json_safe_u64")]` is applied directly on
// the variant fields (see reward_distribution_moment/mod.rs).
impl JsonSafeFields
    for crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment
{
}
// `ContestedIndexFieldMatch::PositiveIntegerMatch(u128)` is made JS-safe via
// `#[serde(with = "json_safe_u128")]` on the variant field (see
// document_type/index/mod.rs); `Regex(LazyRegex)` round-trips as a string.
impl JsonSafeFields for crate::data_contract::document_type::ContestedIndexFieldMatch {}
// `TokenDistributionInfo::PreProgrammed` carries a `TimestampMillis` (u64) made
// JS-safe via `#[serde(with = "json_safe_u64")]`; `Perpetual`'s
// `RewardDistributionMoment` is JS-safe via its own annotation.
impl JsonSafeFields
    for crate::data_contract::associated_token::token_distribution_key::TokenDistributionInfo
{
}
