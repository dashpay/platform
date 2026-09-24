use dpp::data_contract::config::moderation::{
    ContractBan, ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationDocument,
    ContractModerationList, ContractModerationReason, ContractSuspension, ContractWarning,
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
    /// The identity on the list.
    pub identity_id: Identifier,
    /// The end of the suspension for a suspension list entry, `None` for the other lists.
    pub until: Option<TimestampMillis>,
    /// Why the identity is on the list: the ban's reason, the suspension's, or for a warning
    /// list entry the reason of the latest warning (which is kept once, with the warnings, on
    /// the wire).
    pub reason: ContractModerationReason,
    /// For a warning list entry, every warning the identity carries, oldest first; empty for
    /// the other lists.
    pub warnings: Vec<ContractWarning>,
}

impl ContractModerationEntry {
    /// Decodes one stored entry: see [`encode_ban`], [`encode_suspension`] and
    /// [`encode_warnings`].
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
                    warnings: vec![],
                })
            }
            ContractModerationList::Suspensions => {
                let ContractSuspension { until, reason } = decode_suspension(value)?;
                Ok(Self {
                    identity_id,
                    until: Some(until),
                    reason,
                    warnings: vec![],
                })
            }
            ContractModerationList::Warnings => {
                let warnings = decode_warnings(value)?;
                // A stored entry holds at least one warning: `decode_warnings` refuses none.
                let reason = warnings
                    .last()
                    .map(|warning| warning.reason.clone())
                    .unwrap_or_default();
                Ok(Self {
                    identity_id,
                    until: None,
                    reason,
                    warnings,
                })
            }
        }
    }
}

/// The stored size of the `until` a suspension entry starts with: a u64.
pub const CONTRACT_SUSPENSION_UNTIL_SIZE: usize = 8;

/// The stored size of what each warning of a warning list entry starts with: its block time
/// as a u64 and the length of its reason as a u16.
pub const CONTRACT_WARNING_FIXED_SIZE: usize = 8 + 2;

/// The number of warnings a warning list entry is estimated to hold when it is not known: the
/// entries a write walks past, and the entry a clearing removes. An entry being written is
/// priced by its own size.
pub const ESTIMATED_CONTRACT_WARNINGS_PER_ENTRY: u32 = 2;

/// The most bytes a reason's code takes in an entry: the tag and the u16.
pub const CONTRACT_MODERATION_REASON_CODE_MAX_SIZE: u32 = 3;

/// The bytes a reason's reason document id takes in an entry. Every entry a seated elected team
/// writes carries one, so an entry whose value is not known is estimated with it.
pub const CONTRACT_MODERATION_REASON_DOCUMENT_ID_SIZE: u32 = 32;

/// The length a reason's text is estimated at when it is not known: a sentence. Estimating
/// every entry at the longest reason the protocol admits made the dry-run processing fee of a
/// moderation some 25 times the applied one.
pub const ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE: u32 = 128;

/// The size an entry of `list` is estimated at when its value is not known: the entries a
/// write walks past, and the entry a delete removes. An entry being written is priced by its
/// own size.
pub fn estimated_entry_value_size(list: ContractModerationList) -> u32 {
    let reason_size = CONTRACT_MODERATION_REASON_CODE_MAX_SIZE
        + CONTRACT_MODERATION_REASON_DOCUMENT_ID_SIZE
        + ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE;
    match list {
        ContractModerationList::Banlist => reason_size,
        ContractModerationList::Suspensions => CONTRACT_SUSPENSION_UNTIL_SIZE as u32 + reason_size,
        ContractModerationList::Warnings => {
            ESTIMATED_CONTRACT_WARNINGS_PER_ENTRY
                * (CONTRACT_WARNING_FIXED_SIZE as u32 + reason_size)
        }
    }
}

/// Which removal records of one document type to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractDocumentRemovalsSelection {
    /// The records of these document ids. An id with no record is proved absent.
    DocumentIds(Vec<Identifier>),
    /// One page: at most `limit` records in document id order, continuing after
    /// `start_after`. A page shorter than the limit is the last one.
    Page {
        /// Continue after this document id.
        start_after: Option<Identifier>,
        /// At most this many records.
        limit: u16,
    },
}

/// A read of the records of the documents a contract's moderators deleted, within one document
/// type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractDocumentRemovalsQuery {
    /// The document type the documents belonged to.
    pub document_type_name: String,
    /// Which records.
    pub selection: ContractDocumentRemovalsSelection,
}

impl ContractDocumentRemovalsQuery {
    /// The most records the read can return: what bounds its proof.
    pub fn limit(&self) -> u16 {
        match &self.selection {
            ContractDocumentRemovalsSelection::DocumentIds(ids) => {
                ids.len().min(u16::MAX as usize) as u16
            }
            ContractDocumentRemovalsSelection::Page { limit, .. } => *limit,
        }
    }
}

/// The record of one document a moderator deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractDocumentRemovalEntry {
    /// The id the document had.
    pub document_id: Identifier,
    /// Whose it was, who removed it, why and when.
    pub removal: ContractDocumentRemoval,
}

impl ContractDocumentRemovalEntry {
    /// Decodes one stored record: see [`encode_document_removal`].
    pub fn from_key_element(key: &[u8], element: &Element) -> Result<Self, String> {
        let document_id = Identifier::from_bytes(key)
            .map_err(|_| format!("document removal key is not a document id: {:?}", key))?;
        let Element::Item(value, _) = element else {
            return Err("document removal is not an item".to_string());
        };
        Ok(Self {
            document_id,
            removal: decode_document_removal(value)?,
        })
    }
}

/// The stored size of a moderation action count: a u32, big-endian.
pub const CONTRACT_MODERATION_ACTION_COUNT_SIZE: usize = 4;

/// Encodes a moderation action count.
pub fn encode_moderation_action_count(count: u32) -> Vec<u8> {
    count.to_be_bytes().to_vec()
}

/// Decodes a moderation action count.
pub fn decode_moderation_action_count(value: &[u8]) -> Result<u32, String> {
    let bytes: [u8; CONTRACT_MODERATION_ACTION_COUNT_SIZE] = value.try_into().map_err(|_| {
        format!(
            "moderation action count holds {} bytes, expected {}",
            value.len(),
            CONTRACT_MODERATION_ACTION_COUNT_SIZE
        )
    })?;
    Ok(u32::from_be_bytes(bytes))
}

/// The stored size of what a document removal starts with: the document owner's id, the
/// moderator's id, the removal time as a u64, the hash of the removed document and the tag
/// byte that says whether a restoration follows.
pub const CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE: usize = 32 + 32 + 8 + 32 + 1;

/// The stored size of the restoration a restored record carries after its tag: the restoring
/// moderator's id and the restoration time as a u64.
pub const CONTRACT_DOCUMENT_RESTORATION_SIZE: usize = 32 + 8;

const NOT_RESTORED: u8 = 0;
const RESTORED: u8 = 1;

/// The size a document removal record is estimated at when its value is not known: the
/// records a write walks past, sized like a list entry's typical reason and as if restored,
/// the larger of the two shapes. A record being written is priced by its own size.
pub fn estimated_document_removal_value_size() -> u32 {
    (CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE + CONTRACT_DOCUMENT_RESTORATION_SIZE) as u32
        + CONTRACT_MODERATION_REASON_CODE_MAX_SIZE
        + CONTRACT_MODERATION_REASON_DOCUMENT_ID_SIZE
        + ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE
}

/// The stored size of a document removal record.
pub fn document_removal_encoded_size(removal: &ContractDocumentRemoval) -> usize {
    CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE
        + if removal.restoration.is_some() {
            CONTRACT_DOCUMENT_RESTORATION_SIZE
        } else {
            0
        }
        + reason_encoded_size(&removal.reason)
}

/// Encodes a document removal record: the document owner's id, the moderator's id, the removal
/// time as eight big-endian bytes, the hash of the removed document, a tag byte (`0`: not
/// restored, `1`: restored) followed when restored by the restoring moderator's id and the
/// restoration time as eight big-endian bytes, then the reason as in [`encode_ban`].
pub fn encode_document_removal(removal: &ContractDocumentRemoval) -> Vec<u8> {
    let mut value = Vec::with_capacity(document_removal_encoded_size(removal));
    value.extend_from_slice(removal.document_owner_id.as_slice());
    value.extend_from_slice(removal.moderator_id.as_slice());
    value.extend_from_slice(&removal.removed_at.to_be_bytes());
    value.extend_from_slice(&removal.document_hash);
    match &removal.restoration {
        None => value.push(NOT_RESTORED),
        Some(restoration) => {
            value.push(RESTORED);
            value.extend_from_slice(restoration.moderator_id.as_slice());
            value.extend_from_slice(&restoration.restored_at.to_be_bytes());
        }
    }
    encode_reason_into(&removal.reason, &mut value);
    value
}

/// Decodes a document removal record.
pub fn decode_document_removal(value: &[u8]) -> Result<ContractDocumentRemoval, String> {
    let cut_short = || {
        format!(
            "document removal holds {} bytes, expected at least {}",
            value.len(),
            CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE
        )
    };
    let (document_owner_id, rest) = value.split_first_chunk::<32>().ok_or_else(cut_short)?;
    let (moderator_id, rest) = rest.split_first_chunk::<32>().ok_or_else(cut_short)?;
    let (removed_at, rest) = rest.split_first_chunk::<8>().ok_or_else(cut_short)?;
    let (document_hash, rest) = rest.split_first_chunk::<32>().ok_or_else(cut_short)?;
    let (tag, rest) = rest.split_first().ok_or_else(cut_short)?;
    let (restoration, reason) = match *tag {
        NOT_RESTORED => (None, rest),
        RESTORED => {
            let (restored_by, rest) = rest.split_first_chunk::<32>().ok_or_else(|| {
                "document removal is cut short inside its restoration".to_string()
            })?;
            let (restored_at, reason) = rest.split_first_chunk::<8>().ok_or_else(|| {
                "document removal is cut short inside its restoration".to_string()
            })?;
            (
                Some(ContractDocumentRestoration {
                    moderator_id: Identifier::from(*restored_by),
                    restored_at: TimestampMillis::from_be_bytes(*restored_at),
                }),
                reason,
            )
        }
        tag => {
            return Err(format!(
                "document removal has unknown restoration tag {}",
                tag
            ))
        }
    };
    // A list entry with no reason bytes is one written before entries carried a reason. No
    // record was ever written without one, so here the same bytes are a record cut short.
    if reason.is_empty() {
        return Err("document removal holds no reason".to_string());
    }
    Ok(ContractDocumentRemoval {
        document_owner_id: Identifier::from(*document_owner_id),
        moderator_id: Identifier::from(*moderator_id),
        reason: decode_reason(reason)?,
        removed_at: TimestampMillis::from_be_bytes(*removed_at),
        document_hash: *document_hash,
        restoration,
    })
}

/// The tag byte of a reason: bit 0 says a code follows, bit 1 that documents follow, bit 2
/// that a reason document id follows.
const REASON_WITHOUT_CODE: u8 = 0;
const REASON_WITH_CODE: u8 = 1;
const REASON_WITH_DOCUMENTS: u8 = 2;
const REASON_WITH_REASON_DOCUMENT: u8 = 4;

/// Encodes a banlist entry: its reason.
///
/// A reason is a tag byte (bit 0: a code, bit 1: documents, bit 2: a reason document), the
/// code as two big-endian bytes when the tag says so, the 32-byte id of the reason document
/// when the tag says so, then, when the tag says so, the documents it cites as their count in
/// one byte followed by each one's type name (its length in one byte, then the name) and its
/// 32-byte id, and the text as UTF-8 up to the end of the value. A reason is always written
/// with its tag; a value without one is an entry from before reasons existed and decodes as
/// the empty reason, a tag without bit 1 is a reason from before documents could be cited, and
/// a tag without bit 2 one from before a reason could name a reason document.
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

/// Encodes a warning list entry: for each warning, oldest first, its block time as eight
/// big-endian bytes, the length of its encoded reason as two big-endian bytes, then the reason
/// as in [`encode_ban`]. The length prefix is what lets one value hold several reasons, each
/// of which would otherwise run to the end of the value.
///
/// A reason is at most a tag, a code and `max_contract_moderation_reason_length` bytes of
/// text, well within a u16, and a longer one never gets past basic structure validation. One
/// that does not fit the prefix is refused rather than written short: an entry whose prefix
/// undercounts its reason could never be read back.
pub fn encode_warnings(warnings: &[ContractWarning]) -> Result<Vec<u8>, String> {
    let mut value = Vec::with_capacity(
        warnings
            .iter()
            .map(|warning| CONTRACT_WARNING_FIXED_SIZE + reason_encoded_size(&warning.reason))
            .sum(),
    );
    for warning in warnings {
        value.extend_from_slice(&warning.warned_at.to_be_bytes());
        let reason_size = u16::try_from(reason_encoded_size(&warning.reason)).map_err(|_| {
            format!(
                "a warning's reason of {} bytes exceeds the {} the entry can hold",
                reason_encoded_size(&warning.reason),
                u16::MAX
            )
        })?;
        value.extend_from_slice(&reason_size.to_be_bytes());
        encode_reason_into(&warning.reason, &mut value);
    }
    Ok(value)
}

/// Decodes a warning list entry. An entry holds at least one warning: one that holds none was
/// never written, since clearing the last warning deletes the entry.
pub fn decode_warnings(value: &[u8]) -> Result<Vec<ContractWarning>, String> {
    let mut warnings = Vec::new();
    let mut rest = value;
    while !rest.is_empty() {
        let cut_short = || {
            format!(
                "warning list entry is cut short inside warning {}",
                warnings.len() + 1
            )
        };
        let (warned_at, after_time) = rest.split_first_chunk::<8>().ok_or_else(cut_short)?;
        let (reason_size, after_size) =
            after_time.split_first_chunk::<2>().ok_or_else(cut_short)?;
        let reason_size = usize::from(u16::from_be_bytes(*reason_size));
        if after_size.len() < reason_size {
            return Err(cut_short());
        }
        let (reason, after_reason) = after_size.split_at(reason_size);
        // A reason is always written with its tag: no warning was ever written before reasons
        // existed, so an empty one is a malformed entry rather than the empty reason.
        if reason.is_empty() {
            return Err(format!(
                "warning {} of the warning list entry holds no reason",
                warnings.len() + 1
            ));
        }
        warnings.push(ContractWarning {
            warned_at: TimestampMillis::from_be_bytes(*warned_at),
            reason: decode_reason(reason)?,
        });
        rest = after_reason;
    }
    if warnings.is_empty() {
        return Err("warning list entry holds no warning".to_string());
    }
    Ok(warnings)
}

fn reason_encoded_size(reason: &ContractModerationReason) -> usize {
    let documents_size = if reason.documents.is_empty() {
        0
    } else {
        1 + reason
            .documents
            .iter()
            .map(|document| 1 + document.document_type_name.len() + 32)
            .sum::<usize>()
    };
    1 + if reason.code.is_some() { 2 } else { 0 }
        + if reason.reason_document_id.is_some() {
            32
        } else {
            0
        }
        + documents_size
        + reason.text.len()
}

fn encode_reason_into(reason: &ContractModerationReason, value: &mut Vec<u8>) {
    let mut tag = REASON_WITHOUT_CODE;
    if reason.code.is_some() {
        tag |= REASON_WITH_CODE;
    }
    if !reason.documents.is_empty() {
        tag |= REASON_WITH_DOCUMENTS;
    }
    if reason.reason_document_id.is_some() {
        tag |= REASON_WITH_REASON_DOCUMENT;
    }
    value.push(tag);
    if let Some(code) = reason.code {
        value.extend_from_slice(&code.to_be_bytes());
    }
    if let Some(reason_document_id) = reason.reason_document_id {
        value.extend_from_slice(reason_document_id.as_slice());
    }
    if !reason.documents.is_empty() {
        // The count and each name fit a byte: the reason's validation bounds both, so a
        // longer one never reaches a writer.
        value.push(reason.documents.len().min(u8::MAX as usize) as u8);
        for document in &reason.documents {
            let name = document.document_type_name.as_bytes();
            value.push(name.len().min(u8::MAX as usize) as u8);
            value.extend_from_slice(name);
            value.extend_from_slice(document.document_id.as_slice());
        }
    }
    value.extend_from_slice(reason.text.as_bytes());
}

fn decode_reason(value: &[u8]) -> Result<ContractModerationReason, String> {
    // An entry written before entries carried a reason: an empty banlist item, a bare
    // `until`. It reads as the empty reason rather than as corrupted state, which would turn
    // every document transition of the identity, and its unban, into an internal error.
    let Some((&tag, mut rest)) = value.split_first() else {
        return Ok(ContractModerationReason::default());
    };
    if tag & !(REASON_WITH_CODE | REASON_WITH_DOCUMENTS | REASON_WITH_REASON_DOCUMENT) != 0 {
        return Err(format!("moderation reason has unknown tag {}", tag));
    }
    let code = if tag & REASON_WITH_CODE != 0 {
        let Some((code, after)) = rest.split_first_chunk::<2>() else {
            return Err("moderation reason is cut short inside its code".to_string());
        };
        rest = after;
        Some(u16::from_be_bytes(*code))
    } else {
        None
    };
    let reason_document_id = if tag & REASON_WITH_REASON_DOCUMENT != 0 {
        let Some((id, after)) = rest.split_first_chunk::<32>() else {
            return Err("moderation reason is cut short inside its reason document id".to_string());
        };
        rest = after;
        Some(Identifier::from(*id))
    } else {
        None
    };
    let mut documents = Vec::new();
    if tag & REASON_WITH_DOCUMENTS != 0 {
        let Some((&count, after)) = rest.split_first() else {
            return Err("moderation reason is cut short before its documents".to_string());
        };
        rest = after;
        for index in 0..count {
            let cut_short = || {
                format!(
                    "moderation reason is cut short inside document {}",
                    index + 1
                )
            };
            let (&name_length, after_length) = rest.split_first().ok_or_else(cut_short)?;
            if after_length.len() < usize::from(name_length) + 32 {
                return Err(cut_short());
            }
            let (name, after_name) = after_length.split_at(usize::from(name_length));
            let (id, after_id) = after_name.split_first_chunk::<32>().ok_or_else(cut_short)?;
            documents.push(ContractModerationDocument {
                document_type_name: std::str::from_utf8(name)
                    .map_err(|_| "moderation reason document type name is not UTF-8".to_string())?
                    .to_string(),
                document_id: Identifier::from(*id),
            });
            rest = after_id;
        }
    }
    let text = std::str::from_utf8(rest)
        .map_err(|_| "moderation reason text is not UTF-8".to_string())?
        .to_string();
    Ok(ContractModerationReason {
        code,
        text,
        documents,
        reason_document_id,
    })
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
            documents: vec![],
            reason_document_id: None,
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
    fn should_round_trip_the_reason_document_a_reason_names() {
        let named = ContractModerationReason {
            code: Some(0x0102),
            text: "spam".to_string(),
            documents: vec![ContractModerationDocument {
                document_type_name: "post".to_string(),
                document_id: Identifier::from([7; 32]),
            }],
            reason_document_id: Some(Identifier::from([9; 32])),
        };
        let value = encode_ban(&named);
        // tag with all three bits, the code, the reason document id, then the documents
        assert_eq!(value[0], 7);
        assert_eq!(&value[1..3], &[1, 2]);
        assert_eq!(&value[3..35], &[9; 32]);
        assert_eq!(value[35], 1);
        assert_eq!(value.len(), reason_encoded_size(&named));
        assert_eq!(decode_ban(&value).expect("decode").reason, named);

        // Alone, right after the tag.
        let alone = ContractModerationReason::from_text("spam")
            .with_reason_document(Identifier::from([9; 32]));
        let value = encode_ban(&alone);
        assert_eq!(value[0], 4);
        assert_eq!(&value[1..33], &[9; 32]);
        assert_eq!(&value[33..], b"spam");
        assert_eq!(decode_ban(&value).expect("decode").reason, alone);

        // A reason written before bit 2 existed reads as naming none, and one cut short
        // inside the id is refused.
        assert_eq!(
            decode_ban(&[&[0u8][..], b"spam"].concat())
                .expect("decode")
                .reason
                .reason_document_id,
            None
        );
        assert!(decode_ban(&[4u8, 9, 9]).is_err());
        assert!(decode_ban(&[8u8]).is_err(), "an unknown tag bit is refused");
    }

    #[test]
    fn should_round_trip_the_documents_a_reason_cites() {
        let cited = ContractModerationReason {
            code: Some(0x0102),
            text: "spam".to_string(),
            documents: vec![
                ContractModerationDocument {
                    document_type_name: "post".to_string(),
                    document_id: Identifier::from([7; 32]),
                },
                ContractModerationDocument {
                    document_type_name: "reply".to_string(),
                    document_id: Identifier::from([8; 32]),
                },
            ],
            reason_document_id: None,
        };
        let value = encode_ban(&cited);
        // tag with both bits, the code, the count, then each name (length, bytes) and id
        assert_eq!(value[0], 3);
        assert_eq!(&value[1..3], &[1, 2]);
        assert_eq!(value[3], 2);
        assert_eq!(&value[4..9], [&[4u8][..], b"post"].concat());
        assert_eq!(&value[9..41], &[7; 32]);
        assert_eq!(&value[41..47], [&[5u8][..], b"reply"].concat());
        assert_eq!(&value[47..79], &[8; 32]);
        assert_eq!(&value[79..], b"spam");
        assert_eq!(decode_ban(&value).expect("decode").reason, cited);
        assert_eq!(reason_encoded_size(&cited), value.len());

        // Without a code the tag carries the documents bit alone.
        let uncoded = ContractModerationReason {
            code: None,
            ..cited.clone()
        };
        let value = encode_ban(&uncoded);
        assert_eq!(value[0], 2);
        assert_eq!(decode_ban(&value).expect("decode").reason, uncoded);
        // Inside a suspension and a warning the reason decodes the same.
        let suspension = encode_suspension(5, &cited);
        assert_eq!(
            decode_suspension(&suspension).expect("decode").reason,
            cited
        );
        let warnings = encode_warnings(&[ContractWarning {
            warned_at: 1,
            reason: cited.clone(),
        }])
        .expect("encode");
        assert_eq!(decode_warnings(&warnings).expect("decode")[0].reason, cited);
    }

    #[test]
    fn should_refuse_a_reason_cut_short_inside_its_documents() {
        decode_ban(&[2]).expect_err("no count");
        decode_ban(&[2, 1]).expect_err("no name length");
        decode_ban(&[2, 1, 4, b'p', b'o']).expect_err("name cut short");
        let mut short_id = vec![2, 1, 4];
        short_id.extend_from_slice(b"post");
        short_id.extend_from_slice(&[7; 31]);
        decode_ban(&short_id).expect_err("id cut short");
        decode_ban(&[4, b'x']).expect_err("unknown tag bit");
        // A count of zero under the documents bit is a reason about no document.
        assert!(decode_ban(&[2, 0, b'x'])
            .expect("decode")
            .reason
            .documents
            .is_empty());
    }

    #[test]
    fn should_round_trip_a_suspension() {
        let reason = ContractModerationReason {
            code: Some(9),
            text: "flooding, second time".to_string(),
            documents: vec![],
            reason_document_id: None,
        };
        let value = encode_suspension(77, &reason);
        assert_eq!(&value[..8], &77u64.to_be_bytes());
        assert_eq!(
            decode_suspension(&value).expect("decode"),
            ContractSuspension { until: 77, reason }
        );
    }

    #[test]
    fn should_round_trip_warnings_oldest_first() {
        let first = ContractWarning {
            warned_at: 1_000,
            reason: ContractModerationReason::from_text("first strike"),
        };
        let second = ContractWarning {
            warned_at: 2_000,
            reason: ContractModerationReason {
                code: Some(3),
                text: String::new(),
                documents: vec![],
                reason_document_id: None,
            },
        };
        let value = encode_warnings(&[first.clone(), second.clone()]).expect("encode");
        // block time, length, tag + text; block time, length, tag + code + no text
        assert_eq!(&value[..8], &1_000u64.to_be_bytes());
        assert_eq!(&value[8..10], &13u16.to_be_bytes());
        assert_eq!(&value[10..23], [&[0u8][..], b"first strike"].concat());
        assert_eq!(&value[23..31], &2_000u64.to_be_bytes());
        assert_eq!(&value[31..33], &3u16.to_be_bytes());
        assert_eq!(&value[33..], &[1u8, 0, 3]);
        assert_eq!(
            decode_warnings(&value).expect("decode"),
            vec![first.clone(), second]
        );

        let id = Identifier::from([5; 32]);
        let entry = ContractModerationEntry::from_key_element(
            ContractModerationList::Warnings,
            id.as_slice(),
            &Element::new_item(value),
        )
        .expect("decode");
        assert_eq!(entry.identity_id, id);
        assert_eq!(entry.until, None);
        // The entry's reason is the latest warning's.
        assert_eq!(entry.reason.code, Some(3));
        assert_eq!(entry.warnings.len(), 2);
        assert_eq!(entry.warnings[0], first);
    }

    #[test]
    fn should_refuse_a_malformed_warning_list_entry() {
        decode_warnings(&[]).expect_err("no warning");
        decode_warnings(&[0; 9]).expect_err("cut short inside the time");
        let mut no_reason = 1_000u64.to_be_bytes().to_vec();
        no_reason.extend_from_slice(&0u16.to_be_bytes());
        decode_warnings(&no_reason).expect_err("a warning holds a reason");
        let mut short_reason = 1_000u64.to_be_bytes().to_vec();
        short_reason.extend_from_slice(&5u16.to_be_bytes());
        short_reason.push(0);
        decode_warnings(&short_reason).expect_err("reason cut short");
        let mut trailing = encode_warnings(&[ContractWarning {
            warned_at: 1,
            reason: ContractModerationReason::default(),
        }])
        .expect("encode");
        trailing.push(0);
        decode_warnings(&trailing).expect_err("trailing byte");
        // A reason the length prefix can not measure is refused, not written short.
        encode_warnings(&[ContractWarning {
            warned_at: 1,
            reason: ContractModerationReason::from_text("x".repeat(usize::from(u16::MAX))),
        }])
        .expect_err("reason past a u16");
    }

    #[test]
    fn should_round_trip_a_document_removal() {
        let removal = ContractDocumentRemoval {
            document_owner_id: Identifier::from([1; 32]),
            moderator_id: Identifier::from([2; 32]),
            reason: ContractModerationReason {
                code: Some(3),
                text: "spam".to_string(),
                documents: vec![],
                reason_document_id: None,
            },
            removed_at: 1_700_000_000_123,
            document_hash: [4; 32],
            restoration: None,
        };
        let value = encode_document_removal(&removal);
        assert_eq!(&value[..32], &[1; 32]);
        assert_eq!(&value[32..64], &[2; 32]);
        assert_eq!(&value[64..72], &1_700_000_000_123u64.to_be_bytes());
        assert_eq!(&value[72..104], &[4; 32]);
        assert_eq!(value[104], 0, "not restored");
        assert_eq!(&value[105..], [&[1u8, 0, 3][..], b"spam"].concat());
        assert_eq!(value.len(), document_removal_encoded_size(&removal));
        assert_eq!(decode_document_removal(&value).expect("decode"), removal);

        // Restored: the restoring moderator and the time follow the tag, before the reason.
        let restored = ContractDocumentRemoval {
            restoration: Some(ContractDocumentRestoration {
                moderator_id: Identifier::from([5; 32]),
                restored_at: 1_700_000_000_999,
            }),
            ..removal.clone()
        };
        let value = encode_document_removal(&restored);
        assert_eq!(value[104], 1, "restored");
        assert_eq!(&value[105..137], &[5; 32]);
        assert_eq!(&value[137..145], &1_700_000_000_999u64.to_be_bytes());
        assert_eq!(&value[145..], [&[1u8, 0, 3][..], b"spam"].concat());
        assert_eq!(value.len(), document_removal_encoded_size(&restored));
        assert_eq!(decode_document_removal(&value).expect("decode"), restored);
        assert!(
            decode_document_removal(&value[..140]).is_err(),
            "a record cut short inside its restoration is refused"
        );
        let mut unknown_tag = value.clone();
        unknown_tag[104] = 2;
        assert!(decode_document_removal(&unknown_tag).is_err());

        // No code and no text: what a moderator that gives no reason leaves.
        let bare = ContractDocumentRemoval {
            reason: ContractModerationReason::default(),
            ..removal
        };
        let value = encode_document_removal(&bare);
        assert_eq!(value.len(), CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE + 1);
        assert_eq!(decode_document_removal(&value).expect("decode"), bare);

        let id = Identifier::from([9; 32]);
        assert_eq!(
            ContractDocumentRemovalEntry::from_key_element(
                id.as_slice(),
                &Element::new_item(value)
            )
            .expect("decode"),
            ContractDocumentRemovalEntry {
                document_id: id,
                removal: bare,
            }
        );
    }

    #[test]
    fn should_refuse_a_malformed_document_removal() {
        decode_document_removal(&[0; 71]).expect_err("cut short");
        decode_document_removal(&[0; 72]).expect_err("no reason");
        let mut unknown_tag = vec![0; 72];
        unknown_tag.push(7);
        decode_document_removal(&unknown_tag).expect_err("unknown tag");
        ContractDocumentRemovalEntry::from_key_element(&[1, 2, 3], &Element::new_item(vec![0; 73]))
            .expect_err("key is not an id");
        ContractDocumentRemovalEntry::from_key_element(&[4; 32], &Element::empty_tree())
            .expect_err("not an item");
    }

    #[test]
    fn should_refuse_a_malformed_entry() {
        decode_ban(&[4, b'x']).expect_err("unknown tag");
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
                warnings: vec![],
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
                warnings: vec![],
            }
        );
    }
}
