use crate::drive::constants::ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE;
use crate::drive::contract::paths::{
    CONTRACT_TEAM_ACTION_INFO_KEY, CONTRACT_TEAM_ACTION_SIGNERS_KEY,
};
use dpp::data_contract::config::moderation::{
    ContractBan, ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationDocument,
    ContractModerationList, ContractModerationReason, ContractSuspension, ContractTeamAction,
    ContractTeamActionEvent, ContractWarning,
};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::property_constraints::SystemProperty;
use dpp::data_contract::document_type::{property_at_path, DocumentTypeRef};
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::TimestampMillis;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
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

/// Decodes one moderation action count as a query of the counts returns it: the member's
/// identity id as the key, and the count as an item.
pub fn decode_moderation_action_count_entry(
    key: &[u8],
    element: &Element,
) -> Result<(Identifier, u32), String> {
    let identity_id =
        Identifier::from_bytes(key).map_err(|_| format!("key {:?} is not an identity id", key))?;
    let Element::Item(value, _) = element else {
        return Err(format!("the count of {} is not an item", identity_id));
    };
    Ok((identity_id, decode_moderation_action_count(value)?))
}

/// The stored size of what a document removal starts with: the document owner's id, the
/// moderator's id, the removal time as a u64, the hash of the removed document and the tag
/// byte that says what follows.
pub const CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE: usize = 32 + 32 + 8 + 32 + 1;

/// The stored size of the restoration a restored record carries after its tag: the restoring
/// moderator's id and the restoration time as a u64.
pub const CONTRACT_DOCUMENT_RESTORATION_SIZE: usize = 32 + 8;

/// The stored size of what the kept fields of a record start with: their length as a u32.
pub const CONTRACT_DOCUMENT_REMOVAL_KEPT_FIELDS_LENGTH_SIZE: usize = 4;

/// The tag byte of a record: bit 0 says a restoration follows, bit 1 that kept fields follow.
/// A record from before fields could be kept has bit 1 clear, and reads as keeping none.
const RECORD_RESTORED: u8 = 1;
const RECORD_WITH_KEPT_FIELDS: u8 = 2;

/// The size a document removal record is estimated at when its value is not known: the
/// records a write walks past, sized like a list entry's typical reason and as if restored,
/// the larger of the two shapes, plus `estimated_kept_fields_size`, what the type's records
/// are estimated to keep ([`estimated_document_removal_kept_fields_size`]). A record being
/// written is priced by its own size.
pub fn estimated_document_removal_value_size(estimated_kept_fields_size: u32) -> u32 {
    ((CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE + CONTRACT_DOCUMENT_RESTORATION_SIZE) as u32
        + CONTRACT_MODERATION_REASON_CODE_MAX_SIZE
        + CONTRACT_MODERATION_REASON_DOCUMENT_ID_SIZE
        + ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE)
        .saturating_add(estimated_kept_fields_size)
}

/// The stored size the fields a removal record of `document_type` keeps
/// (`moderatorAbilities.deleteKeepsFields`) are estimated at when the record is not known: the
/// records a write walks past. They are encoded as the document encodes its properties
/// (`dpp::data_contract::config::moderation::encode_kept_fields`), so each kept path is sized
/// as a document of the type is estimated: its presence byte and its property's middle size,
/// an object kept whole at the middle sizes of its members, a system time or height at its
/// eight bytes and a core block height at its four. The records of a type keep the same paths
/// but not the same values (a path the document held no value at takes its presence byte
/// alone), so they are estimated from the type, not from whichever record is being written.
/// `0` for a type that keeps none.
pub fn estimated_document_removal_kept_fields_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<u32, ProtocolError> {
    let kept_paths = document_type.moderator_deletion_kept_fields();
    if kept_paths.is_empty() {
        return Ok(0);
    }
    let mut size = CONTRACT_DOCUMENT_REMOVAL_KEPT_FIELDS_LENGTH_SIZE as u32;
    for path in kept_paths {
        let value_size = match SystemProperty::from_name(path) {
            Some(
                SystemProperty::CreatedAtCoreBlockHeight
                | SystemProperty::UpdatedAtCoreBlockHeight
                | SystemProperty::TransferredAtCoreBlockHeight,
            ) => 4,
            Some(_) => 8,
            None => property_at_path(document_type.properties(), path)
                .map(|property| {
                    property
                        .property_type
                        .saturating_middle_byte_size_ceil(platform_version)
                })
                .transpose()?
                .flatten()
                .map_or(0, u32::from),
        };
        // The presence byte, then the value
        size = size.saturating_add(1).saturating_add(value_size);
    }
    Ok(size)
}

/// The stored size of a document removal record. Fails as [`encode_document_removal`] does.
pub fn document_removal_encoded_size(removal: &ContractDocumentRemoval) -> Result<usize, String> {
    Ok(CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE
        + if removal.restoration.is_some() {
            CONTRACT_DOCUMENT_RESTORATION_SIZE
        } else {
            0
        }
        + document_removal_kept_fields_encoded_size(&removal.kept_fields)
        + reason_encoded_size(&removal.reason))
}

/// The stored size of the fields a record keeps: their length and the bytes, nothing when it
/// keeps none.
pub fn document_removal_kept_fields_encoded_size(kept_fields: &[u8]) -> usize {
    if kept_fields.is_empty() {
        0
    } else {
        CONTRACT_DOCUMENT_REMOVAL_KEPT_FIELDS_LENGTH_SIZE + kept_fields.len()
    }
}

/// Encodes a document removal record: the document owner's id, the moderator's id, the removal
/// time as eight big-endian bytes, the hash of the removed document, a tag byte (bit 0: a
/// restoration follows, bit 1: kept fields follow), the restoring moderator's id and the
/// restoration time as eight big-endian bytes when restored, the fields the record keeps when it
/// keeps any (their length as four big-endian bytes, then the bytes, encoded as the document
/// encodes its properties and read under its type, see
/// `dpp::data_contract::config::moderation::encode_kept_fields`), then the reason as in
/// [`encode_ban`]. A record that keeps no fields is written without them, as before fields
/// could be kept.
///
/// Fails only on kept fields too long for their length prefix, which the document size limits
/// keep far out of reach: a record whose prefix undercounts what follows could never be read
/// back.
pub fn encode_document_removal(removal: &ContractDocumentRemoval) -> Result<Vec<u8>, String> {
    let mut value = Vec::with_capacity(
        CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE
            + CONTRACT_DOCUMENT_RESTORATION_SIZE
            + reason_encoded_size(&removal.reason),
    );
    value.extend_from_slice(removal.document_owner_id.as_slice());
    value.extend_from_slice(removal.moderator_id.as_slice());
    value.extend_from_slice(&removal.removed_at.to_be_bytes());
    value.extend_from_slice(&removal.document_hash);
    let mut tag = 0;
    if removal.restoration.is_some() {
        tag |= RECORD_RESTORED;
    }
    if !removal.kept_fields.is_empty() {
        tag |= RECORD_WITH_KEPT_FIELDS;
    }
    value.push(tag);
    if let Some(restoration) = &removal.restoration {
        value.extend_from_slice(restoration.moderator_id.as_slice());
        value.extend_from_slice(&restoration.restored_at.to_be_bytes());
    }
    if !removal.kept_fields.is_empty() {
        let length = u32::try_from(removal.kept_fields.len()).map_err(|_| {
            format!(
                "kept fields of {} bytes exceed the {} a record can hold",
                removal.kept_fields.len(),
                u32::MAX
            )
        })?;
        value.extend_from_slice(&length.to_be_bytes());
        value.extend_from_slice(&removal.kept_fields);
    }
    encode_reason_into(&removal.reason, &mut value);
    Ok(value)
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
    if *tag & !(RECORD_RESTORED | RECORD_WITH_KEPT_FIELDS) != 0 {
        return Err(format!("document removal has unknown tag {}", tag));
    }
    let (restoration, rest) = if *tag & RECORD_RESTORED != 0 {
        let (restored_by, rest) = rest
            .split_first_chunk::<32>()
            .ok_or_else(|| "document removal is cut short inside its restoration".to_string())?;
        let (restored_at, rest) = rest
            .split_first_chunk::<8>()
            .ok_or_else(|| "document removal is cut short inside its restoration".to_string())?;
        (
            Some(ContractDocumentRestoration {
                moderator_id: Identifier::from(*restored_by),
                restored_at: TimestampMillis::from_be_bytes(*restored_at),
            }),
            rest,
        )
    } else {
        (None, rest)
    };
    let (kept_fields, reason) = if *tag & RECORD_WITH_KEPT_FIELDS != 0 {
        let cut_short = || "document removal is cut short inside its kept fields".to_string();
        let (length, rest) = rest
            .split_first_chunk::<CONTRACT_DOCUMENT_REMOVAL_KEPT_FIELDS_LENGTH_SIZE>()
            .ok_or_else(cut_short)?;
        let length = usize::try_from(u32::from_be_bytes(*length)).map_err(|_| cut_short())?;
        // The bit is set only for a record that keeps fields
        if length == 0 {
            return Err("document removal says it keeps fields and keeps none".to_string());
        }
        let (kept_fields, reason) = rest.split_at_checked(length).ok_or_else(cut_short)?;
        (kept_fields.to_vec(), reason)
    } else {
        (Vec::new(), rest)
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
        kept_fields,
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

/// One page of a contract's team actions, active or closed, in action id order: at most
/// `limit`, from `start_at` (the id, and whether it is included) when set. A page shorter than
/// the limit is the last one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTeamActionsQuery {
    /// Active (still collecting approvals) or closed (ran).
    pub status: GroupActionStatus,
    /// Start at this action id, included when the flag is set.
    pub start_at: Option<(Identifier, bool)>,
    /// At most this many actions.
    pub limit: u16,
}

/// One of a contract's team actions, by its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTeamActionEntry {
    /// The action's id.
    pub action_id: Identifier,
    /// Who proposed it, when, and what it does.
    pub action: ContractTeamAction,
    /// How many approvals it holds, the proposer's among them: an active action's so far, a
    /// closed one's that counted when it ran. An active action's is an upper bound: the
    /// approval of a member who left the team is dropped only when a later approval reads the
    /// team, and is counted here until then. The exact figure is the action's signers
    /// (`getContractTeamActionSigners`) still on the team, worth reading only for an action
    /// whose count could meet its rule.
    pub approval_count: u32,
}

/// What one member's signature writes of a team action: see
/// `Drive::add_contract_team_action_signature_operations`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractTeamActionWrite {
    /// A proposal, the proposer's approval: the action written active, or closed when its
    /// proposer meets the rule alone (`closes`).
    Propose {
        /// The action.
        action: ContractTeamAction,
        /// Whether the proposal meets the action's rule alone, the action then written straight
        /// to the closed actions.
        closes: bool,
    },
    /// An approval of an active action that stays active.
    Approve {
        /// The approvals of members no longer on the team, which the approval found when it
        /// read the team and which it deletes: they no longer count, nor prove, and a member
        /// who comes back after this approves again.
        dropped_signers: Vec<Identifier>,
    },
    /// The approval that meets the rule: the action, with the approvals that counted and its
    /// info, moves to the closed actions.
    Close {
        /// The action as stored, whose info moves: an estimate prices the move by its size.
        action: ContractTeamAction,
        /// The approvals given before this one that counted, which move with the action.
        earlier_signers: Vec<Identifier>,
        /// The approvals of members no longer on the team, which are deleted.
        dropped_signers: Vec<Identifier>,
    },
}

/// The tag a team action's value starts with: what it does.
const TEAM_ACTION_DELETE_SETTLED_DOCUMENT: u8 = 0;

/// The stored size of a team action less its document type name and its reason: the tag, the
/// proposer id, the proposal time, the name's length, the document id, the document's last
/// modification and its revision.
pub const CONTRACT_TEAM_ACTION_FIXED_SIZE: usize = 1 + 32 + 8 + 1 + 32 + 8 + 8;

/// The size a team action is estimated at: the fixed part, a typical document type name and a
/// typical reason, whose tag, code and reason document it always carries (a seated team's reason
/// names one its proposal lists).
pub fn estimated_contract_team_action_value_size() -> u32 {
    CONTRACT_TEAM_ACTION_FIXED_SIZE as u32
        + u32::from(ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE)
        + CONTRACT_MODERATION_REASON_CODE_MAX_SIZE
        + CONTRACT_MODERATION_REASON_DOCUMENT_ID_SIZE
        + ESTIMATED_CONTRACT_MODERATION_REASON_TEXT_SIZE
}

/// The stored size of a team action.
pub fn contract_team_action_encoded_size(action: &ContractTeamAction) -> usize {
    match &action.event {
        ContractTeamActionEvent::DeleteSettledDocument {
            document_type_name,
            reason,
            ..
        } => {
            CONTRACT_TEAM_ACTION_FIXED_SIZE + document_type_name.len() + reason_encoded_size(reason)
        }
    }
}

/// Encodes a team action: the tag (`0`, the deletion of a settled document), the proposer id,
/// the proposal time as a u64 big-endian, the document type name's length as a byte and the
/// name, the document id, the document's last modification as a u64 big-endian, its revision as
/// a u64 big-endian (`0` for a type whose documents carry none: a revision starts at 1), and the
/// reason, see [`encode_ban`].
pub fn encode_contract_team_action(action: &ContractTeamAction) -> Vec<u8> {
    let mut value = Vec::with_capacity(contract_team_action_encoded_size(action));
    match &action.event {
        ContractTeamActionEvent::DeleteSettledDocument {
            document_type_name,
            document_id,
            document_last_modified_at,
            document_revision,
            reason,
        } => {
            value.push(TEAM_ACTION_DELETE_SETTLED_DOCUMENT);
            value.extend_from_slice(action.proposer_id.as_slice());
            value.extend_from_slice(&action.proposed_at.to_be_bytes());
            // A document type name is at most 64 bytes, which a byte holds
            value.push(document_type_name.len().min(u8::MAX as usize) as u8);
            value.extend_from_slice(document_type_name.as_bytes());
            value.extend_from_slice(document_id.as_slice());
            value.extend_from_slice(&document_last_modified_at.to_be_bytes());
            value.extend_from_slice(&document_revision.unwrap_or_default().to_be_bytes());
            encode_reason_into(reason, &mut value);
        }
    }
    value
}

/// Decodes a team action: see [`encode_contract_team_action`].
pub fn decode_contract_team_action(value: &[u8]) -> Result<ContractTeamAction, String> {
    let cut_short = |inside: &str| format!("team action is cut short inside its {}", inside);
    let (&tag, rest) = value.split_first().ok_or_else(|| cut_short("tag"))?;
    if tag != TEAM_ACTION_DELETE_SETTLED_DOCUMENT {
        return Err(format!("team action has unknown tag {}", tag));
    }
    let (proposer_id, rest) = rest
        .split_first_chunk::<32>()
        .ok_or_else(|| cut_short("proposer id"))?;
    let (proposed_at, rest) = rest
        .split_first_chunk::<8>()
        .ok_or_else(|| cut_short("proposal time"))?;
    let (&name_length, rest) = rest
        .split_first()
        .ok_or_else(|| cut_short("document type name"))?;
    if rest.len() < usize::from(name_length) {
        return Err(cut_short("document type name"));
    }
    let (name, rest) = rest.split_at(usize::from(name_length));
    let (document_id, rest) = rest
        .split_first_chunk::<32>()
        .ok_or_else(|| cut_short("document id"))?;
    let (last_modified_at, rest) = rest
        .split_first_chunk::<8>()
        .ok_or_else(|| cut_short("document's last modification"))?;
    let (revision, rest) = rest
        .split_first_chunk::<8>()
        .ok_or_else(|| cut_short("document's revision"))?;
    // A team action always carries a reason, its tag at least: an empty rest is corrupted
    // state, not an entry written before reasons.
    if rest.is_empty() {
        return Err(cut_short("reason"));
    }
    let revision = u64::from_be_bytes(*revision);
    Ok(ContractTeamAction {
        proposer_id: Identifier::from(*proposer_id),
        proposed_at: u64::from_be_bytes(*proposed_at),
        event: ContractTeamActionEvent::DeleteSettledDocument {
            document_type_name: std::str::from_utf8(name)
                .map_err(|_| "team action document type name is not UTF-8".to_string())?
                .to_string(),
            document_id: Identifier::from(*document_id),
            document_last_modified_at: u64::from_be_bytes(*last_modified_at),
            document_revision: (revision != 0).then_some(revision),
            reason: decode_reason(rest)?,
        },
    })
}

impl ContractTeamActionEntry {
    /// Decodes a page of team actions as `Drive::contract_team_actions_query` returns it: for
    /// each action, in action id order, its info item (`I`) and then its approvals sum tree
    /// (`S`), both under the path whose last segment is the action id. Each approval is a sum
    /// item of 1, so the tree's sum is how many the action holds.
    pub fn from_path_key_elements(
        results: impl IntoIterator<Item = (Vec<Vec<u8>>, Vec<u8>, Element)>,
    ) -> Result<Vec<Self>, String> {
        let mut results = results.into_iter();
        let mut entries = vec![];
        while let Some((path, key, info)) = results.next() {
            let action_id = path
                .last()
                .and_then(|key| Identifier::from_bytes(key).ok())
                .ok_or_else(|| format!("team action path {:?} does not end in an id", path))?;
            if key != CONTRACT_TEAM_ACTION_INFO_KEY {
                return Err(format!(
                    "team action {} lists {:?} before its info",
                    action_id, key
                ));
            }
            let Element::Item(value, _) = info else {
                return Err(format!("team action {} info is not an item", action_id));
            };
            let Some((approvals_path, approvals_key, approvals)) = results.next() else {
                return Err(format!("team action {} has no approvals tree", action_id));
            };
            if approvals_path != path || approvals_key != CONTRACT_TEAM_ACTION_SIGNERS_KEY {
                return Err(format!(
                    "team action {} is not followed by its approvals tree",
                    action_id
                ));
            }
            let Element::SumTree(_, approvals_sum, _) = approvals else {
                return Err(format!(
                    "team action {} approvals are not a sum tree",
                    action_id
                ));
            };
            let approval_count = u32::try_from(approvals_sum).map_err(|_| {
                format!(
                    "team action {} holds {} approvals",
                    action_id, approvals_sum
                )
            })?;
            entries.push(Self {
                action_id,
                action: decode_contract_team_action(&value)?,
                approval_count,
            });
        }
        Ok(entries)
    }
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
            kept_fields: Vec::new(),
        };
        let value = encode_document_removal(&removal).expect("encode");
        assert_eq!(&value[..32], &[1; 32]);
        assert_eq!(&value[32..64], &[2; 32]);
        assert_eq!(&value[64..72], &1_700_000_000_123u64.to_be_bytes());
        assert_eq!(&value[72..104], &[4; 32]);
        assert_eq!(value[104], 0, "not restored");
        assert_eq!(&value[105..], [&[1u8, 0, 3][..], b"spam"].concat());
        assert_eq!(value.len(), CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE + 3 + 4);
        assert_eq!(decode_document_removal(&value).expect("decode"), removal);

        // Restored: the restoring moderator and the time follow the tag, before the reason.
        let restored = ContractDocumentRemoval {
            restoration: Some(ContractDocumentRestoration {
                moderator_id: Identifier::from([5; 32]),
                restored_at: 1_700_000_000_999,
            }),
            ..removal.clone()
        };
        let value = encode_document_removal(&restored).expect("encode");
        assert_eq!(value[104], 1, "restored");
        assert_eq!(&value[105..137], &[5; 32]);
        assert_eq!(&value[137..145], &1_700_000_000_999u64.to_be_bytes());
        assert_eq!(&value[145..], [&[1u8, 0, 3][..], b"spam"].concat());
        assert_eq!(
            value.len(),
            CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE + CONTRACT_DOCUMENT_RESTORATION_SIZE + 3 + 4
        );
        assert_eq!(decode_document_removal(&value).expect("decode"), restored);
        assert!(
            decode_document_removal(&value[..140]).is_err(),
            "a record cut short inside its restoration is refused"
        );
        let mut unknown_tag = value.clone();
        unknown_tag[104] = 4;
        assert!(decode_document_removal(&unknown_tag).is_err());

        // No code and no text: what a moderator that gives no reason leaves.
        let bare = ContractDocumentRemoval {
            reason: ContractModerationReason::default(),
            ..removal
        };
        let value = encode_document_removal(&bare).expect("encode");
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
    fn should_round_trip_a_document_removal_that_keeps_fields() {
        // What a type keeps, encoded as its documents encode their properties: opaque here,
        // read under the type by `ContractDocumentRemoval::kept_values`
        let kept_fields = vec![1, 4, b'd', b'a', b's', b'h', 0];
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
            kept_fields: kept_fields.clone(),
        };
        let value = encode_document_removal(&removal).expect("encode");
        assert_eq!(value[104], 2, "kept fields, not restored");
        // Their length, then the bytes, then the reason
        assert_eq!(&value[105..109], &7u32.to_be_bytes());
        assert_eq!(&value[109..116], kept_fields.as_slice());
        assert_eq!(&value[116..], [&[1u8, 0, 3][..], b"spam"].concat());
        assert_eq!(
            value.len(),
            document_removal_encoded_size(&removal).expect("size")
        );
        assert_eq!(decode_document_removal(&value).expect("decode"), removal);

        // Restored: the restoration comes first, then the kept fields, then the reason.
        let restored = ContractDocumentRemoval {
            restoration: Some(ContractDocumentRestoration {
                moderator_id: Identifier::from([5; 32]),
                restored_at: 1_700_000_000_999,
            }),
            ..removal.clone()
        };
        let value = encode_document_removal(&restored).expect("encode");
        assert_eq!(value[104], 3, "restored, with kept fields");
        assert_eq!(&value[105..137], &[5; 32]);
        assert_eq!(&value[145..149], &7u32.to_be_bytes());
        assert_eq!(
            value.len(),
            document_removal_encoded_size(&restored).expect("size")
        );
        assert_eq!(decode_document_removal(&value).expect("decode"), restored);

        // No fields kept: the record is the one written before fields could be kept.
        assert_eq!(document_removal_kept_fields_encoded_size(&[]), 0);
        assert_eq!(
            document_removal_kept_fields_encoded_size(&kept_fields),
            4 + 7
        );
    }

    #[test]
    fn should_refuse_a_record_whose_kept_fields_are_malformed() {
        let removal = ContractDocumentRemoval {
            document_owner_id: Identifier::from([1; 32]),
            moderator_id: Identifier::from([2; 32]),
            reason: ContractModerationReason::from_text("spam".to_string()),
            removed_at: 1,
            document_hash: [4; 32],
            restoration: None,
            kept_fields: vec![1, 2, 3],
        };
        let value = encode_document_removal(&removal).expect("encode");
        let reason = 105 + 4 + 3;

        // Cut short inside the length or inside the bytes it announces
        for end in [106, 108, 110] {
            decode_document_removal(&value[..end]).expect_err("cut short inside the kept fields");
        }

        // The tag says fields follow and none do.
        let mut none_kept = value[..105].to_vec();
        none_kept.extend_from_slice(&0u32.to_be_bytes());
        none_kept.extend_from_slice(&value[reason..]);
        assert_eq!(
            decode_document_removal(&none_kept),
            Err("document removal says it keeps fields and keeps none".to_string())
        );

        // A record ends with its reason: a whole prefix, or a whole kept section, followed by
        // nothing is refused as a record without one, not read as the empty reason
        assert_eq!(
            decode_document_removal(&value[..reason]),
            Err("document removal holds no reason".to_string())
        );
        let mut bare_prefix = value[..CONTRACT_DOCUMENT_REMOVAL_FIXED_SIZE].to_vec();
        bare_prefix[104] = 0;
        assert_eq!(
            decode_document_removal(&bare_prefix),
            Err("document removal holds no reason".to_string())
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
    fn should_round_trip_a_team_action() {
        let action = ContractTeamAction {
            proposer_id: Identifier::from([2; 32]),
            proposed_at: 1_000,
            event: ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name: "post".to_string(),
                document_id: Identifier::from([5; 32]),
                document_last_modified_at: 10,
                document_revision: Some(3),
                reason: ContractModerationReason {
                    code: Some(7),
                    text: "doxxing".to_string(),
                    documents: vec![],
                    reason_document_id: None,
                },
            },
        };
        let value = encode_contract_team_action(&action);
        assert_eq!(value.len(), contract_team_action_encoded_size(&action));
        assert_eq!(decode_contract_team_action(&value).expect("decode"), action);

        let no_revision = ContractTeamAction {
            event: ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name: "note".to_string(),
                document_id: Identifier::from([6; 32]),
                document_last_modified_at: 11,
                document_revision: None,
                reason: ContractModerationReason::from_text("spam"),
            },
            ..action.clone()
        };
        let value = encode_contract_team_action(&no_revision);
        assert_eq!(
            decode_contract_team_action(&value).expect("decode"),
            no_revision
        );

        let path = vec![
            vec![64],
            vec![9; 32],
            vec![2],
            vec![24],
            b"M".to_vec(),
            vec![4; 32],
        ];
        let info = Element::new_item(encode_contract_team_action(&action));
        let approvals = Element::new_sum_tree_with_flags_and_sum_value(None, 3, None);
        assert_eq!(
            ContractTeamActionEntry::from_path_key_elements([
                (path.clone(), b"I".to_vec(), info.clone()),
                (path.clone(), b"S".to_vec(), approvals.clone()),
            ])
            .expect("entry"),
            vec![ContractTeamActionEntry {
                action_id: Identifier::from([4; 32]),
                action,
                approval_count: 3,
            }]
        );

        // Each action's info is followed by its approvals tree, and nothing else
        ContractTeamActionEntry::from_path_key_elements([(
            path.clone(),
            b"I".to_vec(),
            info.clone(),
        )])
        .expect_err("no approvals tree");
        ContractTeamActionEntry::from_path_key_elements([
            (path.clone(), b"S".to_vec(), approvals.clone()),
            (path.clone(), b"I".to_vec(), info.clone()),
        ])
        .expect_err("approvals before the info");
        ContractTeamActionEntry::from_path_key_elements([
            (path.clone(), b"I".to_vec(), info.clone()),
            (path.clone(), b"S".to_vec(), Element::empty_tree()),
        ])
        .expect_err("approvals not a sum tree");
        ContractTeamActionEntry::from_path_key_elements([
            (path.clone(), b"I".to_vec(), info),
            (
                path,
                b"S".to_vec(),
                Element::new_sum_tree_with_flags_and_sum_value(None, -1, None),
            ),
        ])
        .expect_err("a negative count");
    }

    #[test]
    fn should_refuse_a_malformed_team_action() {
        decode_contract_team_action(&[]).expect_err("no tag");
        decode_contract_team_action(&[1]).expect_err("unknown tag");
        decode_contract_team_action(&[0; 33]).expect_err("cut short before the proposal time");
        let mut no_reason = vec![0; 42];
        no_reason.extend_from_slice(&[0; 48]);
        decode_contract_team_action(&no_reason).expect_err("no reason");
        let mut name_cut_short = vec![0; 41];
        name_cut_short.push(10);
        name_cut_short.extend_from_slice(b"post");
        decode_contract_team_action(&name_cut_short).expect_err("name cut short");
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
