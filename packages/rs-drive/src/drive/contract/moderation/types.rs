use dpp::data_contract::config::moderation::ContractModerationList;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractModerationEntry {
    /// The barred identity.
    pub identity_id: Identifier,
    /// The end of the suspension for a suspension list entry, `None` for a banlist entry.
    pub until: Option<TimestampMillis>,
}

impl ContractModerationEntry {
    /// Decodes one stored entry. A banlist entry is an empty item, a suspension entry holds
    /// `until` as eight big-endian bytes.
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
        let until = match list {
            ContractModerationList::Banlist => None,
            ContractModerationList::Suspensions => Some(decode_until(value)?),
        };
        Ok(Self { identity_id, until })
    }
}

/// Decodes the `until` of a suspension entry.
pub fn decode_until(value: &[u8]) -> Result<TimestampMillis, String> {
    let bytes: [u8; 8] = value
        .try_into()
        .map_err(|_| format!("suspension entry holds {} bytes, expected 8", value.len()))?;
    Ok(TimestampMillis::from_be_bytes(bytes))
}

/// Encodes the `until` of a suspension entry.
pub fn encode_until(until: TimestampMillis) -> Vec<u8> {
    until.to_be_bytes().to_vec()
}
