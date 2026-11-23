//! Marshalling of the address-funds elements a trunk or branch query returns.

use dash_sdk::drive::grovedb::Element;
use dash_sdk::platform::address_sync::AddressFunds;

use crate::types::DashSDKTrunkStateElement;
use crate::FFIError;

/// The address-funds elements of a trunk or branch result as FFI elements.
///
/// Each element is decoded with the SDK's `AddressFunds::try_from`, the
/// decoder the address sync itself uses, so every encoding the SDK accepts
/// (address funds are stored as `ItemWithSumItem`: the nonce as the item,
/// the balance as the sum) is accepted here, and an element it cannot decode
/// fails the whole call instead of being skipped. Every element is decoded
/// before any key is moved to the heap, so an error leaves nothing to free.
/// On success each element's `key` is owned by the returned value and freed
/// with the trunk or branch state that carries it.
pub(crate) fn address_funds_elements_into_ffi<'a>(
    elements: impl IntoIterator<Item = (&'a Vec<u8>, &'a Element)>,
) -> Result<Vec<DashSDKTrunkStateElement>, FFIError> {
    let decoded = elements
        .into_iter()
        .map(|(key, element)| {
            AddressFunds::try_from(element)
                .map(|funds| (key.clone(), funds))
                .map_err(|source| FFIError::SDKCallFailed {
                    context: format!(
                        "cannot decode the address funds element at key {}",
                        hex::encode(key)
                    ),
                    source,
                })
        })
        .collect::<Result<Vec<_>, FFIError>>()?;

    Ok(decoded
        .into_iter()
        .map(|(key, funds)| {
            let key_len = key.len();
            DashSDKTrunkStateElement {
                key: Box::into_raw(key.into_boxed_slice()) as *mut u8,
                key_len,
                nonce: funds.nonce,
                balance: funds.balance,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dash_sdk::drive::grovedb::Element;

    use super::address_funds_elements_into_ffi;
    use crate::types::DashSDKTrunkStateElement;
    use crate::FFIError;

    /// Free the keys `address_funds_elements_into_ffi` moved to the heap.
    fn free_keys(elements: Vec<DashSDKTrunkStateElement>) {
        for element in elements {
            drop(unsafe { Vec::from_raw_parts(element.key, element.key_len, element.key_len) });
        }
    }

    #[test]
    fn should_decode_an_item_with_sum_item_element_to_its_balance_and_nonce() {
        let key = vec![0x00; 21];
        let nonce: u32 = 7;
        let balance: u64 = 123_456_789;
        // How Drive stores address funds (`set_balance_to_address_v0`): the
        // nonce as big-endian item bytes, the balance as the sum value.
        let element = Element::new_item_with_sum_item_with_flags(
            nonce.to_be_bytes().to_vec(),
            balance as i64,
            None,
        );
        let elements = BTreeMap::from([(key.clone(), element)]);

        let ffi = address_funds_elements_into_ffi(elements.iter()).expect("valid funds element");

        assert_eq!(ffi.len(), 1);
        assert_eq!(ffi[0].nonce, nonce);
        assert_eq!(ffi[0].balance, balance);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(ffi[0].key, ffi[0].key_len) },
            key.as_slice()
        );
        free_keys(ffi);
    }

    #[test]
    fn should_fail_on_an_element_that_is_not_address_funds() {
        let elements = BTreeMap::from([
            (
                vec![0x01; 21],
                Element::new_item_with_sum_item_with_flags(1u32.to_be_bytes().to_vec(), 5, None),
            ),
            (vec![0x02; 21], Element::new_item(vec![0; 12])),
        ]);

        let result = address_funds_elements_into_ffi(elements.iter());

        assert!(matches!(result, Err(FFIError::SDKCallFailed { .. })));
    }
}
