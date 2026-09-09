//! Public changeset traits must remain available with or without `shielded`.

use platform_wallet::changeset::ShieldedChangeSet;

static_assertions::assert_not_impl_any!(ShieldedChangeSet: Copy, PartialEq, Eq);

#[test]
fn should_support_common_traits() {
    fn assert_traits<T: std::fmt::Debug + Clone + Default>() {}
    assert_traits::<ShieldedChangeSet>();
}

#[cfg(feature = "serde")]
#[test]
fn should_round_trip_empty_shielded_changeset() {
    fn assert_serde<T: serde::Serialize + serde::de::DeserializeOwned>() {}
    assert_serde::<ShieldedChangeSet>();

    let empty: ShieldedChangeSet = Default::default();
    let encoded = serde_json::to_string(&empty).unwrap();
    let decoded: ShieldedChangeSet = serde_json::from_str(&encoded).unwrap();
    assert!(decoded.is_empty());
}
