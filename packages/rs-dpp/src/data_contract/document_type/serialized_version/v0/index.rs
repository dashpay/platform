//! The stored form of a document type's indexes ([`Index`]) and derived index properties
//! ([`DerivedIndexProperty`]), with what the parse filled in: an `integerRange`'s key type, the
//! terminal, the skip set, the indexed array, and a derived property's resolved type.

use super::property::StoredPropertyType;
use super::stored_error;
use crate::data_contract::document_type::{
    ContestedIndexFieldMatch, ContestedIndexInformation, ContestedIndexResolution,
    DerivedIndexField, DerivedIndexProperty, DocumentReferenceKind, Index, IndexCountability,
    IndexProperty, IntegerRangeKeyType, IntegerRangeTransform, LazyRegex, TimeRangeTransform,
};
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use std::collections::BTreeMap;

/// [`Index`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredIndex {
    name: String,
    properties: Vec<(String, bool)>,
    unique: bool,
    null_searchable: bool,
    contested_index: Option<StoredContestedIndex>,
    countable: StoredIndexCountability,
    range_countable: bool,
    summable: Option<String>,
    range_summable: bool,
    ranked_countable: bool,
    ranked_countable_at: Vec<String>,
    ranked_summable: bool,
    ranked_averageable: bool,
    ranked_summable_at: Vec<String>,
    ranked_averageable_at: Vec<String>,
    time_range: Option<StoredTimeRange>,
    integer_range: Option<StoredIntegerRange>,
    terminal: Option<Vec<String>>,
    preallocated: bool,
    outlives_delete: bool,
    skip_if_absent: bool,
    skip_if_absent_properties: Vec<String>,
    summable_off_count_index: Option<String>,
    indexed_array: Option<String>,
}

/// [`ContestedIndexInformation`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredContestedIndex {
    field_matches: BTreeMap<String, StoredContestedFieldMatch>,
    resolution: StoredContestedResolution,
}

/// [`ContestedIndexFieldMatch`]: a regex as its pattern, compiled when the index is read back,
/// which refuses a pattern that does not compile.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredContestedFieldMatch {
    Regex(String),
    PositiveIntegerMatch(u128),
}

/// [`ContestedIndexResolution`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredContestedResolution {
    MasternodeVote,
    MasternodeVoteNoLocking,
}

/// [`IndexCountability`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredIndexCountability {
    NotCountable,
    Countable,
    CountableAllowingOffset,
}

/// [`TimeRangeTransform`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredTimeRange {
    source: String,
    range_seconds: u64,
    step_seconds: u64,
    phase_seconds: u64,
    ttl_seconds: Option<u64>,
}

/// [`IntegerRangeTransform`], its key type included: the parse resolves it from the source
/// property, and `IntegerRangeTransform`'s equality leaves it out.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredIntegerRange {
    source: String,
    range: u64,
    step: u64,
    phase: u64,
    key_type: StoredIntegerRangeKeyType,
}

/// [`IntegerRangeKeyType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredIntegerRangeKeyType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
}

/// [`DerivedIndexProperty`], its property type as resolved across the contract's document
/// types.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredDerivedIndexProperty {
    reference_property: String,
    referenced_document_type_name: String,
    kind: StoredReferenceKind,
    field: StoredDerivedIndexField,
    property_type: Option<StoredPropertyType>,
}

/// [`DocumentReferenceKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredReferenceKind {
    Permanent,
    Moderated,
    Deletable,
}

/// [`DerivedIndexField`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredDerivedIndexField {
    OwnerId,
    CreatorId,
    Property(String),
}

impl From<&Index> for StoredIndex {
    fn from(index: &Index) -> Self {
        let Index {
            name,
            properties,
            unique,
            null_searchable,
            contested_index,
            countable,
            range_countable,
            summable,
            range_summable,
            ranked_countable,
            ranked_countable_at,
            ranked_summable,
            ranked_averageable,
            ranked_summable_at,
            ranked_averageable_at,
            time_range,
            integer_range,
            terminal,
            preallocated,
            outlives_delete,
            skip_if_absent,
            skip_if_absent_properties,
            summable_off_count_index,
            indexed_array,
        } = index;
        StoredIndex {
            name: name.clone(),
            properties: properties
                .iter()
                .map(|property| (property.name.clone(), property.ascending))
                .collect(),
            unique: *unique,
            null_searchable: *null_searchable,
            contested_index: contested_index
                .as_ref()
                .map(|contested| StoredContestedIndex {
                    field_matches: contested
                        .field_matches
                        .iter()
                        .map(|(name, field_match)| {
                            (
                                name.clone(),
                                match field_match {
                                    ContestedIndexFieldMatch::Regex(regex) => {
                                        StoredContestedFieldMatch::Regex(regex.as_str().to_string())
                                    }
                                    ContestedIndexFieldMatch::PositiveIntegerMatch(value) => {
                                        StoredContestedFieldMatch::PositiveIntegerMatch(*value)
                                    }
                                },
                            )
                        })
                        .collect(),
                    resolution: match contested.resolution {
                        ContestedIndexResolution::MasternodeVote => {
                            StoredContestedResolution::MasternodeVote
                        }
                        ContestedIndexResolution::MasternodeVoteNoLocking => {
                            StoredContestedResolution::MasternodeVoteNoLocking
                        }
                    },
                }),
            countable: match countable {
                IndexCountability::NotCountable => StoredIndexCountability::NotCountable,
                IndexCountability::Countable => StoredIndexCountability::Countable,
                IndexCountability::CountableAllowingOffset => {
                    StoredIndexCountability::CountableAllowingOffset
                }
            },
            range_countable: *range_countable,
            summable: summable.clone(),
            range_summable: *range_summable,
            ranked_countable: *ranked_countable,
            ranked_countable_at: ranked_countable_at.clone(),
            ranked_summable: *ranked_summable,
            ranked_averageable: *ranked_averageable,
            ranked_summable_at: ranked_summable_at.clone(),
            ranked_averageable_at: ranked_averageable_at.clone(),
            time_range: time_range.as_ref().map(|time_range| StoredTimeRange {
                source: time_range.source.clone(),
                range_seconds: time_range.range_seconds,
                step_seconds: time_range.step_seconds,
                phase_seconds: time_range.phase_seconds,
                ttl_seconds: time_range.ttl_seconds,
            }),
            integer_range: integer_range
                .as_ref()
                .map(|integer_range| StoredIntegerRange {
                    source: integer_range.source.clone(),
                    range: integer_range.range,
                    step: integer_range.step,
                    phase: integer_range.phase,
                    key_type: match integer_range.key_type {
                        IntegerRangeKeyType::U8 => StoredIntegerRangeKeyType::U8,
                        IntegerRangeKeyType::I8 => StoredIntegerRangeKeyType::I8,
                        IntegerRangeKeyType::U16 => StoredIntegerRangeKeyType::U16,
                        IntegerRangeKeyType::I16 => StoredIntegerRangeKeyType::I16,
                        IntegerRangeKeyType::U32 => StoredIntegerRangeKeyType::U32,
                        IntegerRangeKeyType::I32 => StoredIntegerRangeKeyType::I32,
                        IntegerRangeKeyType::U64 => StoredIntegerRangeKeyType::U64,
                        IntegerRangeKeyType::I64 => StoredIntegerRangeKeyType::I64,
                    },
                }),
            terminal: terminal.clone(),
            preallocated: *preallocated,
            outlives_delete: *outlives_delete,
            skip_if_absent: *skip_if_absent,
            skip_if_absent_properties: skip_if_absent_properties.clone(),
            summable_off_count_index: summable_off_count_index.clone(),
            indexed_array: indexed_array.clone(),
        }
    }
}

/// Refuses a contested field's regex that does not compile, as the schema parser does: a stored
/// form can come from anyone through the untrusted decoder.
impl TryFrom<StoredIndex> for Index {
    type Error = ProtocolError;

    fn try_from(stored: StoredIndex) -> Result<Self, Self::Error> {
        let StoredIndex {
            name,
            properties,
            unique,
            null_searchable,
            contested_index,
            countable,
            range_countable,
            summable,
            range_summable,
            ranked_countable,
            ranked_countable_at,
            ranked_summable,
            ranked_averageable,
            ranked_summable_at,
            ranked_averageable_at,
            time_range,
            integer_range,
            terminal,
            preallocated,
            outlives_delete,
            skip_if_absent,
            skip_if_absent_properties,
            summable_off_count_index,
            indexed_array,
        } = stored;
        let contested_index = contested_index
            .map(|contested| {
                let field_matches = contested
                    .field_matches
                    .into_iter()
                    .map(|(field, field_match)| {
                        let field_match = match field_match {
                            StoredContestedFieldMatch::Regex(pattern) => {
                                ContestedIndexFieldMatch::Regex(
                                    LazyRegex::try_compiled(pattern).map_err(|error| {
                                        stored_error(format!(
                                            "index {name} has a contested regex on {field} that \
                                             does not compile: {error}"
                                        ))
                                    })?,
                                )
                            }
                            StoredContestedFieldMatch::PositiveIntegerMatch(value) => {
                                ContestedIndexFieldMatch::PositiveIntegerMatch(value)
                            }
                        };
                        Ok((field, field_match))
                    })
                    .collect::<Result<_, ProtocolError>>()?;
                Ok::<_, ProtocolError>(ContestedIndexInformation {
                    field_matches,
                    resolution: match contested.resolution {
                        StoredContestedResolution::MasternodeVote => {
                            ContestedIndexResolution::MasternodeVote
                        }
                        StoredContestedResolution::MasternodeVoteNoLocking => {
                            ContestedIndexResolution::MasternodeVoteNoLocking
                        }
                    },
                })
            })
            .transpose()?;
        Ok(Index {
            name,
            properties: properties
                .into_iter()
                .map(|(name, ascending)| IndexProperty { name, ascending })
                .collect(),
            unique,
            null_searchable,
            contested_index,
            countable: match countable {
                StoredIndexCountability::NotCountable => IndexCountability::NotCountable,
                StoredIndexCountability::Countable => IndexCountability::Countable,
                StoredIndexCountability::CountableAllowingOffset => {
                    IndexCountability::CountableAllowingOffset
                }
            },
            range_countable,
            summable,
            range_summable,
            ranked_countable,
            ranked_countable_at,
            ranked_summable,
            ranked_averageable,
            ranked_summable_at,
            ranked_averageable_at,
            time_range: time_range.map(|time_range| TimeRangeTransform {
                source: time_range.source,
                range_seconds: time_range.range_seconds,
                step_seconds: time_range.step_seconds,
                phase_seconds: time_range.phase_seconds,
                ttl_seconds: time_range.ttl_seconds,
            }),
            integer_range: integer_range.map(|integer_range| IntegerRangeTransform {
                source: integer_range.source,
                range: integer_range.range,
                step: integer_range.step,
                phase: integer_range.phase,
                key_type: match integer_range.key_type {
                    StoredIntegerRangeKeyType::U8 => IntegerRangeKeyType::U8,
                    StoredIntegerRangeKeyType::I8 => IntegerRangeKeyType::I8,
                    StoredIntegerRangeKeyType::U16 => IntegerRangeKeyType::U16,
                    StoredIntegerRangeKeyType::I16 => IntegerRangeKeyType::I16,
                    StoredIntegerRangeKeyType::U32 => IntegerRangeKeyType::U32,
                    StoredIntegerRangeKeyType::I32 => IntegerRangeKeyType::I32,
                    StoredIntegerRangeKeyType::U64 => IntegerRangeKeyType::U64,
                    StoredIntegerRangeKeyType::I64 => IntegerRangeKeyType::I64,
                },
            }),
            terminal,
            preallocated,
            outlives_delete,
            skip_if_absent,
            skip_if_absent_properties,
            summable_off_count_index,
            indexed_array,
        })
    }
}

impl From<&DerivedIndexProperty> for StoredDerivedIndexProperty {
    fn from(derived: &DerivedIndexProperty) -> Self {
        let DerivedIndexProperty {
            reference_property,
            referenced_document_type_name,
            kind,
            field,
            property_type,
        } = derived;
        StoredDerivedIndexProperty {
            reference_property: reference_property.clone(),
            referenced_document_type_name: referenced_document_type_name.clone(),
            kind: match kind {
                DocumentReferenceKind::Permanent => StoredReferenceKind::Permanent,
                DocumentReferenceKind::Moderated => StoredReferenceKind::Moderated,
                DocumentReferenceKind::Deletable => StoredReferenceKind::Deletable,
            },
            field: match field {
                DerivedIndexField::OwnerId => StoredDerivedIndexField::OwnerId,
                DerivedIndexField::CreatorId => StoredDerivedIndexField::CreatorId,
                DerivedIndexField::Property(path) => {
                    StoredDerivedIndexField::Property(path.clone())
                }
            },
            property_type: property_type.as_ref().map(StoredPropertyType::from),
        }
    }
}

impl TryFrom<StoredDerivedIndexProperty> for DerivedIndexProperty {
    type Error = ProtocolError;

    fn try_from(stored: StoredDerivedIndexProperty) -> Result<Self, Self::Error> {
        let StoredDerivedIndexProperty {
            reference_property,
            referenced_document_type_name,
            kind,
            field,
            property_type,
        } = stored;
        Ok(DerivedIndexProperty {
            reference_property,
            referenced_document_type_name,
            kind: match kind {
                StoredReferenceKind::Permanent => DocumentReferenceKind::Permanent,
                StoredReferenceKind::Moderated => DocumentReferenceKind::Moderated,
                StoredReferenceKind::Deletable => DocumentReferenceKind::Deletable,
            },
            field: match field {
                StoredDerivedIndexField::OwnerId => DerivedIndexField::OwnerId,
                StoredDerivedIndexField::CreatorId => DerivedIndexField::CreatorId,
                StoredDerivedIndexField::Property(path) => DerivedIndexField::Property(path),
            },
            property_type: property_type.map(TryInto::try_into).transpose()?,
        })
    }
}
