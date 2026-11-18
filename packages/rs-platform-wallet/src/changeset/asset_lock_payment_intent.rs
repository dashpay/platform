//! What an asset lock was built to pay, recorded with its tracked row.
//!
//! A lock that funds the wallet's own Platform addresses or identities needs
//! no record: any owned destination a resume names is the wallet's own money.
//! A lock built to pay someone else does: without a record, recovery cannot
//! tell it from an owned top-up and would consume it into the wallet,
//! refunding the sender while the payee is never paid. The intent is the
//! record. It is written on the row before the lock is sent (see
//! `AssetLockManager::broadcast_funded_asset_lock_tracked`), is immutable
//! afterwards, and a resume either pays it or is refused.
//!
//! Persistence backends carry it as opaque bytes ([`AssetLockPaymentIntent::to_bytes`]);
//! only this module reads them.

use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::Credits;

/// The largest encoded intent a backend has to store, in bytes. An intent
/// over it is refused before anything is built, so a host can load any
/// stored intent into a buffer of this size. It covers 136 Platform-address
/// recipients; consensus admits 128 outputs per funding.
pub const ASSET_LOCK_PAYMENT_INTENT_MAX_SIZE: usize = 4096;

const KIND_PLATFORM_ADDRESSES: u8 = 1;
const PLATFORM_ADDRESS_SIZE: usize = 21;

/// What an asset lock pays when it does not fund the wallet itself.
///
/// A sum type so the destinations that can pay a third party share one
/// record: Platform addresses today, an identity (dashpay/platform#5924) and
/// the shielded recipient (still on `AssetLockEntry::shielded_recipient`)
/// later. Exhaustive on purpose, like `AssetLockStatus`: a new variant must
/// reach every match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetLockPaymentIntent {
    /// The recipient map of `fund_from_asset_lock_external`, exactly as the
    /// funding was built from it: explicit-amount entries may be any
    /// Platform address, and the single `None` entry is the sender's own
    /// remainder (change and fee).
    PlatformAddresses(BTreeMap<PlatformAddress, Option<Credits>>),
}

/// Why stored intent bytes could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("asset-lock payment intent is malformed: {0}")]
pub struct AssetLockPaymentIntentDecodeError(&'static str);

/// Why an intent has no stored form: it is not one
/// [`AssetLockPaymentIntent::from_bytes`] would read back (not exactly one
/// remainder recipient), or its stored form is over
/// [`ASSET_LOCK_PAYMENT_INTENT_MAX_SIZE`] (which also bounds the `u16` count).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("asset-lock payment intent has no stored form: {0}")]
pub struct AssetLockPaymentIntentEncodeError(&'static str);

impl AssetLockPaymentIntent {
    /// The stored form: a kind byte, then the kind's body. For Platform
    /// addresses the body is a big-endian `u16` count and, per recipient in
    /// map order, the 21-byte `PlatformAddress::to_bytes` form, a `0`/`1`
    /// amount flag and, after `1`, the big-endian `u64` amount in credits.
    /// Only what [`Self::from_bytes`] reads back is encoded, and only within
    /// [`ASSET_LOCK_PAYMENT_INTENT_MAX_SIZE`]: a map without exactly one
    /// remainder (`None`) recipient, or one too large to store, is refused
    /// rather than stored in a form restoration would reject or a host could
    /// not load.
    pub fn to_bytes(&self) -> Result<Vec<u8>, AssetLockPaymentIntentEncodeError> {
        match self {
            Self::PlatformAddresses(recipients) => {
                if recipients
                    .values()
                    .filter(|amount| amount.is_none())
                    .count()
                    != 1
                {
                    return Err(AssetLockPaymentIntentEncodeError(
                        "not exactly one remainder recipient",
                    ));
                }
                if !self.fits_storage() {
                    return Err(AssetLockPaymentIntentEncodeError(
                        "larger than the stored-form limit",
                    ));
                }
                let count = u16::try_from(recipients.len()).map_err(|_| {
                    AssetLockPaymentIntentEncodeError("larger than the stored-form limit")
                })?;
                let mut bytes = Vec::with_capacity(3 + recipients.len() * 30);
                bytes.push(KIND_PLATFORM_ADDRESSES);
                bytes.extend_from_slice(&count.to_be_bytes());
                for (address, amount) in recipients {
                    bytes.extend_from_slice(&address.to_bytes());
                    match amount {
                        Some(credits) => {
                            bytes.push(1);
                            bytes.extend_from_slice(&credits.to_be_bytes());
                        }
                        None => bytes.push(0),
                    }
                }
                Ok(bytes)
            }
        }
    }

    /// Read the stored form back. Strict: an unknown kind, a short or long
    /// body, an unordered or repeated address, an empty map, or a map without
    /// exactly one remainder (`None`) recipient is refused,
    /// so a row whose intent cannot be read is never treated as having none.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AssetLockPaymentIntentDecodeError> {
        let (&kind, body) = bytes
            .split_first()
            .ok_or(AssetLockPaymentIntentDecodeError("empty"))?;
        if kind != KIND_PLATFORM_ADDRESSES {
            return Err(AssetLockPaymentIntentDecodeError("unknown kind"));
        }
        let (count, mut rest) = body
            .split_first_chunk::<2>()
            .ok_or(AssetLockPaymentIntentDecodeError("missing recipient count"))?;
        let count = u16::from_be_bytes(*count);
        if count == 0 {
            return Err(AssetLockPaymentIntentDecodeError("no recipients"));
        }
        let mut recipients = BTreeMap::new();
        let mut previous: Option<PlatformAddress> = None;
        for _ in 0..count {
            let (address, after) = rest
                .split_first_chunk::<PLATFORM_ADDRESS_SIZE>()
                .ok_or(AssetLockPaymentIntentDecodeError("truncated address"))?;
            let address = PlatformAddress::from_bytes(address)
                .map_err(|_| AssetLockPaymentIntentDecodeError("invalid address"))?;
            if previous.is_some_and(|previous| previous >= address) {
                return Err(AssetLockPaymentIntentDecodeError(
                    "recipients out of order or repeated",
                ));
            }
            let (&flag, after) = after
                .split_first()
                .ok_or(AssetLockPaymentIntentDecodeError("truncated amount"))?;
            let (amount, after) = match flag {
                0 => (None, after),
                1 => {
                    let (credits, after) = after
                        .split_first_chunk::<8>()
                        .ok_or(AssetLockPaymentIntentDecodeError("truncated amount"))?;
                    (Some(u64::from_be_bytes(*credits)), after)
                }
                _ => return Err(AssetLockPaymentIntentDecodeError("invalid amount flag")),
            };
            recipients.insert(address, amount);
            previous = Some(address);
            rest = after;
        }
        if !rest.is_empty() {
            return Err(AssetLockPaymentIntentDecodeError("trailing bytes"));
        }
        // The funding was built only from a map with exactly one remainder
        // (`None`) entry (`validate_recipient_shape` in
        // `fund_from_asset_lock`), so a stored map with any other count was
        // not written by it.
        if recipients
            .values()
            .filter(|amount| amount.is_none())
            .count()
            != 1
        {
            return Err(AssetLockPaymentIntentDecodeError(
                "not exactly one remainder recipient",
            ));
        }
        Ok(Self::PlatformAddresses(recipients))
    }

    /// Whether the stored form fits [`ASSET_LOCK_PAYMENT_INTENT_MAX_SIZE`].
    pub fn fits_storage(&self) -> bool {
        match self {
            Self::PlatformAddresses(recipients) => {
                recipients.len() <= usize::from(u16::MAX)
                    && 3 + recipients.len() * (PLATFORM_ADDRESS_SIZE + 9)
                        <= ASSET_LOCK_PAYMENT_INTENT_MAX_SIZE
            }
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for AssetLockPaymentIntent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = self.to_bytes().map_err(serde::ser::Error::custom)?;
        serializer.serialize_bytes(&bytes)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for AssetLockPaymentIntent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;
        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> AssetLockPaymentIntent {
        AssetLockPaymentIntent::PlatformAddresses(BTreeMap::from([
            (PlatformAddress::P2pkh([0x11; 20]), Some(5_000_000)),
            (PlatformAddress::P2pkh([0x22; 20]), None),
            (PlatformAddress::P2sh([0x01; 20]), Some(7)),
        ]))
    }

    #[test]
    fn should_round_trip_a_platform_address_intent_through_its_stored_form() {
        let bytes = intent().to_bytes().unwrap();
        assert_eq!(bytes.len(), 3 + 21 * 3 + 9 + 1 + 9);
        assert_eq!(
            AssetLockPaymentIntent::from_bytes(&bytes).unwrap(),
            intent()
        );
    }

    #[test]
    fn should_refuse_malformed_stored_intents() {
        let bytes = intent().to_bytes().unwrap();
        let mut unknown_kind = bytes.clone();
        unknown_kind[0] = 9;
        let mut trailing = bytes.clone();
        trailing.push(0);
        let mut bad_flag = bytes.clone();
        bad_flag[3 + 21] = 2;
        let empty = vec![KIND_PLATFORM_ADDRESSES, 0, 0];
        let mut repeated = vec![KIND_PLATFORM_ADDRESSES, 0, 2];
        for _ in 0..2 {
            repeated.extend_from_slice(&PlatformAddress::P2pkh([0x11; 20]).to_bytes());
            repeated.push(0);
        }
        for malformed in [
            &[][..],
            &unknown_kind,
            &trailing,
            &bytes[..bytes.len() - 1],
            &bad_flag,
            &empty,
            &repeated,
        ] {
            assert!(
                AssetLockPaymentIntent::from_bytes(malformed).is_err(),
                "{malformed:?} must not decode"
            );
        }
    }

    /// The stored form of `recipients` written by hand, bypassing the
    /// encoder's shape check.
    fn raw(recipients: &[(PlatformAddress, Option<Credits>)]) -> Vec<u8> {
        let mut bytes = vec![KIND_PLATFORM_ADDRESSES];
        bytes.extend_from_slice(&(recipients.len() as u16).to_be_bytes());
        for (address, amount) in recipients {
            bytes.extend_from_slice(&address.to_bytes());
            match amount {
                Some(credits) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&credits.to_be_bytes());
                }
                None => bytes.push(0),
            }
        }
        bytes
    }

    #[test]
    fn should_refuse_a_stored_intent_without_a_remainder_recipient() {
        let none = raw(&[
            (PlatformAddress::P2pkh([0x11; 20]), Some(5_000_000)),
            (PlatformAddress::P2pkh([0x22; 20]), Some(7)),
        ]);
        assert!(AssetLockPaymentIntent::from_bytes(&none).is_err());
    }

    #[test]
    fn should_refuse_a_stored_intent_with_two_remainder_recipients() {
        let two = raw(&[
            (PlatformAddress::P2pkh([0x11; 20]), Some(5_000_000)),
            (PlatformAddress::P2pkh([0x22; 20]), None),
            (PlatformAddress::P2pkh([0x33; 20]), None),
        ]);
        assert!(AssetLockPaymentIntent::from_bytes(&two).is_err());
    }

    /// The encoder refuses every shape the decoder refuses, so no stored
    /// intent fails restoration: an empty map, none or two remainder
    /// recipients.
    #[test]
    fn should_refuse_to_encode_an_intent_its_decoder_would_reject() {
        for recipients in [
            BTreeMap::new(),
            BTreeMap::from([(PlatformAddress::P2pkh([0x11; 20]), Some(5))]),
            BTreeMap::from([
                (PlatformAddress::P2pkh([0x11; 20]), None),
                (PlatformAddress::P2pkh([0x22; 20]), None),
            ]),
        ] {
            let intent = AssetLockPaymentIntent::PlatformAddresses(recipients);
            assert_eq!(
                intent.to_bytes(),
                Err(AssetLockPaymentIntentEncodeError(
                    "not exactly one remainder recipient"
                )),
                "{intent:?}"
            );
        }
    }

    /// An intent whose stored form is over the size a host loads is refused
    /// by the encoder, so no persistence path can store one.
    #[test]
    fn should_refuse_to_encode_an_intent_over_the_stored_size_limit() {
        let recipients = (0..=136u8)
            .map(|i| (PlatformAddress::P2pkh([i; 20]), Some(1)))
            .chain([(PlatformAddress::P2pkh([0xff; 20]), None)])
            .collect::<BTreeMap<_, _>>();
        let intent = AssetLockPaymentIntent::PlatformAddresses(recipients);
        assert!(!intent.fits_storage());
        assert_eq!(
            intent.to_bytes(),
            Err(AssetLockPaymentIntentEncodeError(
                "larger than the stored-form limit"
            ))
        );
    }

    #[test]
    fn should_fit_storage_for_every_recipient_count_consensus_admits() {
        let recipients = (0..=127u8)
            .map(|i| (PlatformAddress::P2pkh([i; 20]), Some(1)))
            .chain([(PlatformAddress::P2pkh([0xff; 20]), None)])
            .collect::<BTreeMap<_, _>>();
        let intent = AssetLockPaymentIntent::PlatformAddresses(recipients);
        assert!(intent.fits_storage());
        assert!(intent.to_bytes().unwrap().len() <= ASSET_LOCK_PAYMENT_INTENT_MAX_SIZE);
    }

    #[test]
    fn should_refuse_to_encode_more_recipients_than_the_count_can_hold() {
        let recipients = (0..=u32::from(u16::MAX))
            .map(|i| {
                let mut hash = [0; 20];
                hash[..4].copy_from_slice(&i.to_be_bytes());
                (PlatformAddress::P2pkh(hash), Some(1))
            })
            .chain([(PlatformAddress::P2pkh([0xff; 20]), None)])
            .collect::<BTreeMap<_, _>>();
        let intent = AssetLockPaymentIntent::PlatformAddresses(recipients);
        assert_eq!(
            intent.to_bytes(),
            Err(AssetLockPaymentIntentEncodeError(
                "larger than the stored-form limit"
            ))
        );
    }
}
