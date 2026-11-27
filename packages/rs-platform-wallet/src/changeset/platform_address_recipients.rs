//! The Platform addresses an external Platform-address funding pays,
//! recorded with its tracked asset lock.
//!
//! A lock that funds the wallet's own Platform addresses needs no record: any
//! owned destination a resume names is the wallet's own money. A lock built
//! by `fund_from_asset_lock_external` pays someone else as well: without a
//! record, a resume cannot tell it from an owned top-up and would consume it
//! into the wallet, refunding the sender while the payee is never paid. The
//! record is written on the row before the lock is sent (see
//! `AssetLockManager::broadcast_funded_asset_lock_tracked`), is immutable
//! afterwards, and a resume either pays it or is refused.
//!
//! Persistence backends carry it as opaque bytes
//! ([`PlatformAddressRecipients::to_bytes`]); only this module reads them.

use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::Credits;

/// The largest stored form a backend has to keep, in bytes. Recipients over
/// it are refused before anything is built, so a host can load any stored
/// record into a buffer of this size. It covers 136 recipients; consensus
/// admits 128 outputs per funding.
pub const PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE: usize = 4096;

/// The first byte of the stored form: the format the rest is in.
const FORMAT_V1: u8 = 1;
const PLATFORM_ADDRESS_SIZE: usize = 21;
/// Format byte and the `u16` recipient count.
const HEADER_SIZE: usize = 3;
/// Address, amount flag and the `u64` amount.
const MAX_RECIPIENT_SIZE: usize = PLATFORM_ADDRESS_SIZE + 9;

/// The recipient map of `fund_from_asset_lock_external`, exactly as the
/// funding was built from it: explicit-amount entries may be any Platform
/// address, and the single `None` entry is the sender's own remainder (change
/// and fee).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAddressRecipients(BTreeMap<PlatformAddress, Option<Credits>>);

/// Why stored recipient bytes could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("asset-lock Platform-address recipients are malformed: {0}")]
pub struct PlatformAddressRecipientsDecodeError(&'static str);

/// Why recipients have no stored form: not exactly one remainder recipient,
/// or a stored form over [`PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE`] (which also
/// bounds the `u16` count).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("asset-lock Platform-address recipients have no stored form: {0}")]
pub struct PlatformAddressRecipientsEncodeError(&'static str);

impl PlatformAddressRecipients {
    /// Recipients a funding is built from. Refused, so they are never
    /// recorded in a form restoration would reject or a host could not load:
    /// a map without exactly one remainder (`None`) recipient, or one whose
    /// stored form is over [`PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE`].
    pub fn new(
        recipients: BTreeMap<PlatformAddress, Option<Credits>>,
    ) -> Result<Self, PlatformAddressRecipientsEncodeError> {
        if recipients
            .values()
            .filter(|amount| amount.is_none())
            .count()
            != 1
        {
            return Err(PlatformAddressRecipientsEncodeError(
                "not exactly one remainder recipient",
            ));
        }
        if recipients.len() > usize::from(u16::MAX)
            || HEADER_SIZE + recipients.len() * MAX_RECIPIENT_SIZE
                > PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE
        {
            return Err(PlatformAddressRecipientsEncodeError(
                "larger than the stored-form limit",
            ));
        }
        Ok(Self(recipients))
    }

    /// The recipient map, in the order consensus resolves it.
    pub fn as_map(&self) -> &BTreeMap<PlatformAddress, Option<Credits>> {
        &self.0
    }

    /// The single remainder recipient: the sender's own change address.
    pub fn remainder(&self) -> Option<&PlatformAddress> {
        self.0
            .iter()
            .find_map(|(address, amount)| amount.is_none().then_some(address))
    }

    /// The stored form: a format byte, a big-endian `u16` count and, per
    /// recipient in map order, the 21-byte `PlatformAddress::to_bytes` form,
    /// a `0`/`1` amount flag and, after `1`, the big-endian `u64` amount in
    /// credits. At most [`PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE`] bytes, which
    /// [`Self::new`] guarantees.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE + self.0.len() * MAX_RECIPIENT_SIZE);
        bytes.push(FORMAT_V1);
        // `new` bounds the count well below `u16::MAX`.
        bytes.extend_from_slice(&(self.0.len() as u16).to_be_bytes());
        for (address, amount) in &self.0 {
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

    /// Read the stored form back. Strict: input over
    /// [`PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE`] is refused before it is
    /// parsed, and so is an unknown format, a short or long body, an
    /// unordered or repeated address, an empty map, or a map without exactly
    /// one remainder (`None`) recipient, so a row whose recipients cannot be
    /// read is never treated as having none.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PlatformAddressRecipientsDecodeError> {
        if bytes.len() > PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE {
            return Err(PlatformAddressRecipientsDecodeError(
                "larger than the stored-form limit",
            ));
        }
        let (&format, body) = bytes
            .split_first()
            .ok_or(PlatformAddressRecipientsDecodeError("empty"))?;
        if format != FORMAT_V1 {
            return Err(PlatformAddressRecipientsDecodeError("unknown format"));
        }
        let (count, mut rest) =
            body.split_first_chunk::<2>()
                .ok_or(PlatformAddressRecipientsDecodeError(
                    "missing recipient count",
                ))?;
        let count = u16::from_be_bytes(*count);
        if count == 0 {
            return Err(PlatformAddressRecipientsDecodeError("no recipients"));
        }
        let mut recipients = BTreeMap::new();
        let mut previous: Option<PlatformAddress> = None;
        for _ in 0..count {
            let (address, after) = rest
                .split_first_chunk::<PLATFORM_ADDRESS_SIZE>()
                .ok_or(PlatformAddressRecipientsDecodeError("truncated address"))?;
            let address = PlatformAddress::from_bytes(address)
                .map_err(|_| PlatformAddressRecipientsDecodeError("invalid address"))?;
            if previous.is_some_and(|previous| previous >= address) {
                return Err(PlatformAddressRecipientsDecodeError(
                    "recipients out of order or repeated",
                ));
            }
            let (&flag, after) = after
                .split_first()
                .ok_or(PlatformAddressRecipientsDecodeError("truncated amount"))?;
            let (amount, after) = match flag {
                0 => (None, after),
                1 => {
                    let (credits, after) = after
                        .split_first_chunk::<8>()
                        .ok_or(PlatformAddressRecipientsDecodeError("truncated amount"))?;
                    (Some(u64::from_be_bytes(*credits)), after)
                }
                _ => return Err(PlatformAddressRecipientsDecodeError("invalid amount flag")),
            };
            recipients.insert(address, amount);
            previous = Some(address);
            rest = after;
        }
        if !rest.is_empty() {
            return Err(PlatformAddressRecipientsDecodeError("trailing bytes"));
        }
        // The funding was built only from recipients `new` accepted, so a
        // stored map it would refuse was not written by it.
        Self::new(recipients).map_err(|_| {
            PlatformAddressRecipientsDecodeError("not exactly one remainder recipient")
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PlatformAddressRecipients {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.to_bytes())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PlatformAddressRecipients {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;
        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipients() -> PlatformAddressRecipients {
        PlatformAddressRecipients::new(BTreeMap::from([
            (PlatformAddress::P2pkh([0x11; 20]), Some(5_000_000)),
            (PlatformAddress::P2pkh([0x22; 20]), None),
            (PlatformAddress::P2sh([0x01; 20]), Some(7)),
        ]))
        .expect("one remainder")
    }

    #[test]
    fn should_round_trip_platform_address_recipients_through_their_stored_form() {
        let bytes = recipients().to_bytes();
        assert_eq!(bytes.len(), 3 + 21 * 3 + 9 + 1 + 9);
        assert_eq!(
            PlatformAddressRecipients::from_bytes(&bytes).unwrap(),
            recipients()
        );
        assert_eq!(
            recipients().remainder(),
            Some(&PlatformAddress::P2pkh([0x22; 20]))
        );
    }

    #[test]
    fn should_refuse_malformed_stored_recipients() {
        let bytes = recipients().to_bytes();
        let mut unknown_format = bytes.clone();
        unknown_format[0] = 9;
        let mut trailing = bytes.clone();
        trailing.push(0);
        let mut bad_flag = bytes.clone();
        bad_flag[3 + 21] = 2;
        let empty = vec![FORMAT_V1, 0, 0];
        let mut repeated = vec![FORMAT_V1, 0, 2];
        for _ in 0..2 {
            repeated.extend_from_slice(&PlatformAddress::P2pkh([0x11; 20]).to_bytes());
            repeated.push(0);
        }
        for malformed in [
            &[][..],
            &unknown_format,
            &trailing,
            &bytes[..bytes.len() - 1],
            &bad_flag,
            &empty,
            &repeated,
        ] {
            assert!(
                PlatformAddressRecipients::from_bytes(malformed).is_err(),
                "{malformed:?} must not decode"
            );
        }
    }

    /// The stored form of `recipients` written by hand, bypassing the
    /// constructor's shape check.
    fn raw(recipients: &[(PlatformAddress, Option<Credits>)]) -> Vec<u8> {
        let mut bytes = vec![FORMAT_V1];
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
    fn should_refuse_stored_recipients_without_exactly_one_remainder() {
        let none = raw(&[
            (PlatformAddress::P2pkh([0x11; 20]), Some(5_000_000)),
            (PlatformAddress::P2pkh([0x22; 20]), Some(7)),
        ]);
        let two = raw(&[
            (PlatformAddress::P2pkh([0x11; 20]), Some(5_000_000)),
            (PlatformAddress::P2pkh([0x22; 20]), None),
            (PlatformAddress::P2pkh([0x33; 20]), None),
        ]);
        for malformed in [none, two] {
            assert!(PlatformAddressRecipients::from_bytes(&malformed).is_err());
        }
    }

    /// Input over the limit is refused before it is parsed, even when every
    /// byte of it would otherwise read back; input at the limit is parsed.
    #[test]
    fn should_refuse_stored_recipients_over_the_size_limit_before_parsing() {
        let mut over = recipients().to_bytes();
        over.resize(PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE + 1, 0);
        assert_eq!(
            PlatformAddressRecipients::from_bytes(&over),
            Err(PlatformAddressRecipientsDecodeError(
                "larger than the stored-form limit"
            ))
        );
        let mut at_limit = recipients().to_bytes();
        at_limit.resize(PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE, 0);
        assert_eq!(
            PlatformAddressRecipients::from_bytes(&at_limit),
            Err(PlatformAddressRecipientsDecodeError("trailing bytes"))
        );
    }

    /// The constructor refuses every shape the decoder refuses, so no record
    /// fails restoration: an empty map, none or two remainder recipients.
    #[test]
    fn should_refuse_recipients_its_decoder_would_reject() {
        for recipients in [
            BTreeMap::new(),
            BTreeMap::from([(PlatformAddress::P2pkh([0x11; 20]), Some(5))]),
            BTreeMap::from([
                (PlatformAddress::P2pkh([0x11; 20]), None),
                (PlatformAddress::P2pkh([0x22; 20]), None),
            ]),
        ] {
            assert_eq!(
                PlatformAddressRecipients::new(recipients),
                Err(PlatformAddressRecipientsEncodeError(
                    "not exactly one remainder recipient"
                ))
            );
        }
    }

    /// Recipients whose stored form is over the size a host loads are
    /// refused, so no persistence path can store them.
    #[test]
    fn should_refuse_recipients_over_the_stored_size_limit() {
        let recipients = (0..=136u8)
            .map(|i| (PlatformAddress::P2pkh([i; 20]), Some(1)))
            .chain([(PlatformAddress::P2pkh([0xff; 20]), None)])
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            PlatformAddressRecipients::new(recipients),
            Err(PlatformAddressRecipientsEncodeError(
                "larger than the stored-form limit"
            ))
        );
    }

    #[test]
    fn should_fit_every_recipient_count_consensus_admits() {
        let recipients = (0..=127u8)
            .map(|i| (PlatformAddress::P2pkh([i; 20]), Some(1)))
            .chain([(PlatformAddress::P2pkh([0xff; 20]), None)])
            .collect::<BTreeMap<_, _>>();
        let recipients = PlatformAddressRecipients::new(recipients).expect("fits");
        assert!(recipients.to_bytes().len() <= PLATFORM_ADDRESS_RECIPIENTS_MAX_SIZE);
    }
}
