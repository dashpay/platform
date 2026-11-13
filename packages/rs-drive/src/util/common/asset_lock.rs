//! The record Drive keeps under `SpentAssetLockTransactions` for each asset
//! lock outpoint a transition used.

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::asset_lock::StoredAssetLockInfo;

/// Reads the item Drive keeps for an asset lock outpoint: none before any
/// transition used the lock, an empty one once a transition used it up, and
/// otherwise the lock's remaining value, which `decode_remaining`
/// deserializes as far as the caller trusts the bytes.
pub fn stored_asset_lock_info<E>(
    item: Option<&[u8]>,
    decode_remaining: impl FnOnce(&[u8]) -> Result<AssetLockValue, E>,
) -> Result<StoredAssetLockInfo, E> {
    match item {
        None => Ok(StoredAssetLockInfo::NotPresent),
        Some([]) => Ok(StoredAssetLockInfo::FullyConsumed),
        Some(bytes) => decode_remaining(bytes).map(StoredAssetLockInfo::PartiallyConsumed),
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::drive::RootTree;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::platform_value::Bytes36;
    use dpp::serialization::{PlatformDeserializableTrusted, PlatformDeserializableUntrusted};
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    /// Drive's writer, its consensus fetch and this reader agree on every
    /// record: none, a used-up lock, and a partly used one.
    #[test]
    fn should_read_records_as_drive_writes_and_fetches_them() {
        let platform_version = PlatformVersion::latest();
        let drive_version = &platform_version.drive;
        let drive = setup_drive_with_initial_state_structure(None);
        let left = AssetLockValue::new(2_000, vec![0x76], 1_500, vec![], platform_version)
            .expect("an asset lock value");

        let cases = [
            (Bytes36::new([1; 36]), None),
            (
                Bytes36::new([2; 36]),
                Some(AssetLockValue::new(2_000, vec![0x76], 0, vec![], platform_version).unwrap()),
            ),
            (Bytes36::new([3; 36]), Some(left.clone())),
        ];
        for (outpoint, value) in &cases {
            let Some(value) = value else { continue };
            let operations = drive
                .add_asset_lock_outpoint_operations(
                    outpoint,
                    value.clone(),
                    &mut None,
                    platform_version,
                )
                .expect("the operations adding an asset lock outpoint");
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    None,
                    operations,
                    &mut vec![],
                    drive_version,
                )
                .expect("an asset lock outpoint added");
        }

        let path: [&[u8]; 1] = [&[RootTree::SpentAssetLockTransactions as u8]];
        let expected = [
            StoredAssetLockInfo::NotPresent,
            StoredAssetLockInfo::FullyConsumed,
            StoredAssetLockInfo::PartiallyConsumed(left),
        ];
        for ((outpoint, _), expected) in cases.iter().zip(expected) {
            let fetched = drive
                .fetch_asset_lock_outpoint_info(outpoint, None, drive_version)
                .expect("an asset lock record fetched");
            let element = drive
                .grove
                .get_raw_optional(
                    (&path).into(),
                    outpoint.as_slice(),
                    None,
                    &drive_version.grove_version,
                )
                .unwrap()
                .expect("a raw read");
            let item = element
                .as_ref()
                .map(Element::as_item_bytes)
                .transpose()
                .unwrap();

            assert_eq!(fetched, expected);
            assert_eq!(
                stored_asset_lock_info(item, AssetLockValue::deserialize_from_bytes_trusted)
                    .unwrap(),
                expected
            );
            assert_eq!(
                stored_asset_lock_info(
                    item,
                    AssetLockValue::deserialize_from_bytes_untrusted_exact
                )
                .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn should_hand_a_nonempty_item_to_the_callers_decoder() {
        let refuse = |_: &[u8]| Err::<AssetLockValue, _>("undecodable");
        assert_eq!(
            stored_asset_lock_info(Some(&[0xff; 3]), refuse),
            Err("undecodable")
        );
        assert_eq!(
            stored_asset_lock_info(Some(&[]), refuse),
            Ok(StoredAssetLockInfo::FullyConsumed)
        );
        assert_eq!(
            stored_asset_lock_info(None, refuse),
            Ok(StoredAssetLockInfo::NotPresent)
        );
    }
}
