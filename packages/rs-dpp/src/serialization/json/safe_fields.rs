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
impl JsonSafeFields for platform_value::Value {}
impl JsonSafeFields for platform_value::string_encoding::Encoding {}

// --- External crate types ---

impl JsonSafeFields for dashcore::OutPoint {}

// --- rs-dpp types that don't contain u64/i64 or have their own safe handling ---
// Add new entries here when a json_safe_fields-annotated struct has a field whose
// type is a simple enum/struct without u64/i64. The compiler will tell you which
// type is missing via a `JsonSafeFields is not satisfied` error.

impl JsonSafeFields for crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers {}
impl JsonSafeFields for crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements {}
impl JsonSafeFields for crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode {}
impl JsonSafeFields for crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient {}
impl JsonSafeFields for crate::identity::Purpose {}
impl JsonSafeFields for crate::identity::SecurityLevel {}
impl JsonSafeFields for crate::identity::KeyType {}
impl JsonSafeFields for crate::block::epoch::Epoch {}
impl JsonSafeFields for crate::identity::identity_public_key::IdentityPublicKey {}
impl JsonSafeFields for crate::identity::state_transition::asset_lock_proof::AssetLockProof {}
impl JsonSafeFields for crate::address_funds::PlatformAddress {}
impl JsonSafeFields for crate::withdrawal::Pooling {}
impl JsonSafeFields for crate::identity::core_script::CoreScript {}
impl JsonSafeFields for crate::voting::votes::Vote {}
impl JsonSafeFields for crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice {}
impl JsonSafeFields for crate::group::action_event::GroupActionEvent {}
// TokenEvent contains u64 aliases (TokenAmount, Credits) in tuple variants that
// `#[json_safe_fields]` can't auto-annotate. Developer takes responsibility for
// JS-safe serialization of these fields. See token_event.rs for details.
impl JsonSafeFields for crate::tokens::token_event::TokenEvent {}
