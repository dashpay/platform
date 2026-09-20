use dpp::data_contract::config::moderation::{
    ContractBan, ContractModerationList, ContractModerationReason, ContractSuspension,
};
use dpp::identity::TimestampMillis;
use dpp::platform_value::Identifier;
use grovedb::Element;

/// One page of one of a contract's moderation lists: at most `limit` identities in id order,
/// continuing after `start_after`. A page shorter than the limit is the last one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractModerationEntriesQuery {
    /// The list to read.
    pub list: ContractModerationList,
    /// Continue after this identity id.
    pub start_after: Option<Identifier>,
    /// At most this many entries.
    pub limit: u16,
}

/// One entry of a moderation list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractModerationEntry {
    /// The barred identity.
    pub identity_id: Identifier,
    /// The end of the suspension for a suspension list entry, `None` for a banlist entry.
    pub until: Option<TimestampMillis>,
    /// Why the moderator banned or suspended the identity.
    pub reason: ContractModerationReason,
}

impl ContractModerationEntry {
    /// Decodes one stored entry: see [`encode_ban`] and [`encode_suspension`].
    pub fn from_key_element(
        list: ContractModerationList,
        key: &[u8],
        element: &Element,
    ) -> Result<Self, String> {
        let identity_id = Identifier::from_bytes(key)
            .map_err(|_| format!("moderation entry key is not an identity id: {:?}", key))?;
        let Element::Item(value, _) = element else {
            return Err("moderation entry is not an item".to_string());
        };
        match list {
            ContractModerationList::Banlist => {
                let ContractBan { reason } = decode_ban(value)?;
                Ok(Self {
                    identity_id,
                    until: None,
                    reason,
                })
            }
            ContractModerationList::Suspensions => {
                let ContractSuspension { until, reason } = decode_suspension(value)?;
                Ok(Self {
                    identity_id,
                    until: Some(until),
                    reason,
                })
            }
        }
    }
}

/// The stored size of the `until` a suspension entry starts with: a u64.
pub const CONTRACT_SUSPENSION_UNTIL_SIZE: usize = 8;

/// The most bytes a reason's code takes in an entry: the tag and the u16.
pub const CONTRACT_MODERATION_REASON_CODE_MAX_SIZE: u32 = 3;

/// The length a reason's text is estimated at when it is not known: a sentence. Estimating
/// every entry at the longest reason the protocol admits made the dry-run processing fee of a
/// moderation some 25 times the applied one.
pub const ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE: u32 = 128;

/// The size an entry of `list` is estimated at when its value is not known: the entries a
/// write walks past, and the entry a delete removes. An entry being written is priced by its
/// own size.
pub fn estimated_entry_value_size(list: ContractModerationList) -> u32 {
    let until_size = match list {
        ContractModerationList::Banlist => 0,
        ContractModerationList::Suspensions => CONTRACT_SUSPENSION_UNTIL_SIZE as u32,
    };
    until_size
        + CONTRACT_MODERATION_REASON_CODE_MAX_SIZE
        + ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE
}

const REASON_WITHOUT_CODE: u8 = 0;
const REASON_WITH_CODE: u8 = 1;

/// Encodes a banlist entry: its reason.
///
/// A reason is a tag byte (`0`: no code, `1`: a code), the code as two big-endian bytes when
/// the tag says so, and the text as UTF-8 up to the end of the value. A reason is always
/// written with its tag; a value without one is an entry from before reasons existed and
/// decodes as the empty reason.
pub fn encode_ban(reason: &ContractModerationReason) -> Vec<u8> {
    let mut value = Vec::with_capacity(reason_encoded_size(reason));
    encode_reason_into(reason, &mut value);
    value
}

/// Decodes a banlist entry.
pub fn decode_ban(value: &[u8]) -> Result<ContractBan, String> {
    Ok(ContractBan {
        reason: decode_reason(value)?,
    })
}

/// Encodes a suspension list entry: `until` as eight big-endian bytes, then the reason as in
/// [`encode_ban`].
pub fn encode_suspension(until: TimestampMillis, reason: &ContractModerationReason) -> Vec<u8> {
    let mut value =
        Vec::with_capacity(CONTRACT_SUSPENSION_UNTIL_SIZE + reason_encoded_size(reason));
    value.extend_from_slice(&until.to_be_bytes());
    encode_reason_into(reason, &mut value);
    value
}

/// Decodes a suspension list entry.
pub fn decode_suspension(value: &[u8]) -> Result<ContractSuspension, String> {
    let Some((until, reason)) = value.split_first_chunk::<CONTRACT_SUSPENSION_UNTIL_SIZE>() else {
        return Err(format!(
            "suspension entry holds {} bytes, expected at least {}",
            value.len(),
            CONTRACT_SUSPENSION_UNTIL_SIZE
        ));
    };
    Ok(ContractSuspension {
        until: TimestampMillis::from_be_bytes(*until),
        reason: decode_reason(reason)?,
    })
}

fn reason_encoded_size(reason: &ContractModerationReason) -> usize {
    1 + if reason.code.is_some() { 2 } else { 0 } + reason.text.len()
}

fn encode_reason_into(reason: &ContractModerationReason, value: &mut Vec<u8>) {
    match reason.code {
        None => value.push(REASON_WITHOUT_CODE),
        Some(code) => {
            value.push(REASON_WITH_CODE);
            value.extend_from_slice(&code.to_be_bytes());
        }
    }
    value.extend_from_slice(reason.text.as_bytes());
}

fn decode_reason(value: &[u8]) -> Result<ContractModerationReason, String> {
    let (code, text) = match value.split_first() {
        Some((&REASON_WITHOUT_CODE, text)) => (None, text),
        Some((&REASON_WITH_CODE, rest)) => {
            let Some((code, text)) = rest.split_first_chunk::<2>() else {
                return Err("moderation reason is cut short inside its code".to_string());
            };
            (Some(u16::from_be_bytes(*code)), text)
        }
        Some((tag, _)) => return Err(format!("moderation reason has unknown code tag {}", tag)),
        // An entry written before entries carried a reason: an empty banlist item, a bare
        // `until`. It reads as the empty reason rather than as corrupted state, which would
        // turn every document transition of the identity, and its unban, into an internal error.
        None => return Ok(ContractModerationReason::default()),
    };
    let text = std::str::from_utf8(text)
        .map_err(|_| "moderation reason text is not UTF-8".to_string())?
        .to_string();
    Ok(ContractModerationReason { code, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_a_ban_with_and_without_a_code() {
        let plain = ContractModerationReason::from_text("spam");
        assert_eq!(encode_ban(&plain), [&[0u8][..], b"spam"].concat());
        assert_eq!(
            decode_ban(&encode_ban(&plain)).expect("decode").reason,
            plain
        );

        let coded = ContractModerationReason {
            code: Some(0x0102),
            text: "spam".to_string(),
        };
        assert_eq!(encode_ban(&coded), [&[1u8, 1, 2][..], b"spam"].concat());
        assert_eq!(
            decode_ban(&encode_ban(&coded)).expect("decode").reason,
            coded
        );

        let empty = ContractModerationReason::default();
        assert_eq!(encode_ban(&empty), vec![0]);
        assert_eq!(decode_ban(&[0]).expect("decode").reason, empty);
    }

    #[test]
    fn should_round_trip_a_suspension() {
        let reason = ContractModerationReason {
            code: Some(9),
            text: "flooding, second time".to_string(),
        };
        let value = encode_suspension(77, &reason);
        assert_eq!(&value[..8], &77u64.to_be_bytes());
        assert_eq!(
            decode_suspension(&value).expect("decode"),
            ContractSuspension { until: 77, reason }
        );
    }

    #[test]
    fn should_refuse_a_malformed_entry() {
        decode_ban(&[2, b'x']).expect_err("unknown tag");
        decode_ban(&[1, 0]).expect_err("code cut short");
        decode_ban(&[0, 0xff, 0xfe]).expect_err("not utf-8");
        decode_suspension(&[0; 7]).expect_err("until cut short");
    }

    #[test]
    fn should_read_an_entry_written_before_reasons_as_the_empty_reason() {
        assert_eq!(
            decode_ban(&[]).expect("decode"),
            ContractBan {
                reason: ContractModerationReason::default()
            }
        );
        assert_eq!(
            decode_suspension(&77u64.to_be_bytes()).expect("decode"),
            ContractSuspension {
                until: 77,
                reason: ContractModerationReason::default()
            }
        );
    }

    #[test]
    fn should_decode_an_entry_by_its_list() {
        let id = Identifier::from([5; 32]);
        let reason = ContractModerationReason::from_text("spam");
        let ban = Element::new_item(encode_ban(&reason));
        assert_eq!(
            ContractModerationEntry::from_key_element(
                ContractModerationList::Banlist,
                id.as_slice(),
                &ban
            )
            .expect("decode"),
            ContractModerationEntry {
                identity_id: id,
                until: None,
                reason: reason.clone(),
            }
        );
        let suspension = Element::new_item(encode_suspension(12, &reason));
        assert_eq!(
            ContractModerationEntry::from_key_element(
                ContractModerationList::Suspensions,
                id.as_slice(),
                &suspension
            )
            .expect("decode"),
            ContractModerationEntry {
                identity_id: id,
                until: Some(12),
                reason,
            }
        );
    }
}
