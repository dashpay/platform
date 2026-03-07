/// Marker trait indicating all u64/i64 fields use `#[serde(with)]` for JS-safe serialization.
///
/// Applied automatically by the `#[json_safe_fields]` attribute macro from
/// `dpp-json-convertible-derive`. If you see a compile error about this trait
/// not being satisfied, add `#[json_safe_fields]` to your struct/enum.
///
/// u64 and i64 implement this trait to support generic containers (e.g., `Vec<u64>`),
/// but bare u64/i64 struct fields are still annotated with `#[serde(with = "json_safe_u64")]`
/// by the macro for JS-safe serialization.
pub trait JsonSafeFields {}

// --- Primitive types (no u64/i64 risk) ---

impl JsonSafeFields for bool {}
impl JsonSafeFields for u8 {}
impl JsonSafeFields for u16 {}
impl JsonSafeFields for u32 {}
impl JsonSafeFields for u64 {}
impl JsonSafeFields for i8 {}
impl JsonSafeFields for i16 {}
impl JsonSafeFields for i32 {}
impl JsonSafeFields for i64 {}
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
// These are simple enums/structs used as fields in json_safe_fields-annotated types.

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
impl JsonSafeFields for crate::tokens::token_event::TokenEvent {}
