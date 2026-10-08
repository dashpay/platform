//! The elements Drive writes when it inserts one document, with the keys,
//! element sizes and parent trees the storage cost depends on. The shape
//! follows `drive::document::layout`, which takes it from the index
//! walkers' rules; the keys and elements are the document's own, built with
//! the functions the walkers use (dpp's serialization and key encoding, the
//! document reference builders, the indexOnly row commitment).

use crate::drive::document::cost::grove_costs::PricedElement;
use crate::drive::document::expiration::paths::encode_expiration_time;
use crate::drive::document::expiration::pricing::document_expires_at;
use crate::drive::document::expiration::DocumentExpirationEntry;
use crate::drive::document::index_level_tree_types::{
    continuation_contributes_zero, document_carries,
    index_level_tree_types_with_continuation_demotion, index_only_level_skips_when_absent,
    level_reaches_entry_by, takes_part_in_index_by, terminal_member_tree_type,
    zero_contribution_wrapper,
};
use crate::drive::document::layout::{index_ending_at, index_paths, indexes_through, LayoutRole};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::{
    bound_value_fits_referring_property, encode_index_only_entry_payload, index_only_member_key,
    index_only_row_commitment, make_document_reference, make_document_reference_with_sum_item,
    preallocation_bindings_targeting, read_document_sum_contribution,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U16;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::{
    is_flat_level_key, DocumentPropertyType, DocumentTypeRef, Index, IndexLevel,
    IndexLevelTypeInfo, PreallocatedKeySource,
};
use dpp::data_contract::DataContract;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use grovedb::element::reference_path::ReferencePathType::SiblingReference;
use grovedb::element::IndexAxis;
use grovedb::Element;
use grovedb_merk::tree_type::TreeType;
use std::collections::{BTreeSet, HashSet};

/// One element an insert writes.
#[derive(Clone, Debug)]
pub(crate) struct Write {
    /// The keys from the document type tree down to the element's tree.
    pub path: Vec<Vec<u8>>,
    /// The element's key.
    pub key: Vec<u8>,
    /// The element, as its storage cost sees it.
    pub element: PricedElement,
    /// The tree the element is inserted into.
    pub parent: TreeType,
    /// What the element is in the layout.
    pub role: LayoutRole,
    /// The indexes that use it; empty for primary storage.
    pub indexes: Vec<String>,
    /// Written only when absent: a tree an earlier document with the same
    /// values created. Every insert writes the others.
    pub if_absent: bool,
    /// On a time window with a `ttl`: priced as processing, not storage.
    pub ephemeral: bool,
    /// A ranked tree's row for one of its entries, when the element is one.
    pub ranking: Option<RankingRow>,
    /// The document type whose tree the element is in, when it is not the
    /// inserted document's: a preallocated index of a type referring to it.
    pub referring_type: Option<String>,
    /// In the documents expirations tree (`[Misc, "E"]`) rather than under
    /// a document type: a document with a `ttl`'s entry there.
    pub expiration: bool,
    /// Whether the element carries the owner's storage flags, which route
    /// its refund when it is removed.
    pub flagged: bool,
}

/// The row a ranked (indexed) tree keeps for one of its entries in the
/// secondary tree of one axis, ordered by the entry's aggregate. Every
/// insert under the entry rewrites it: the aggregate, hence the sort key,
/// changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RankingRow {
    /// The axis the secondary tree orders by.
    pub axis: IndexAxis,
    /// The path of the entry: the ranked tree.
    pub entry_path: Vec<Vec<u8>>,
    /// The entry's key: the document's value.
    pub entry_key: Vec<u8>,
}

/// The key and element of the row of the entry `entry_key` holding
/// `count` and `sum` in the secondary tree of `axis`: the sort key followed
/// by the entry key, holding a one-hop reference to the entry that carries
/// the axis's aggregate (grovedb `make_axis_secondary_key`,
/// `axis_row_reference`).
pub(crate) fn ranking_row(
    axis: IndexAxis,
    entry_key: &[u8],
    count: u64,
    sum: i64,
    platform_version: &PlatformVersion,
) -> Result<(Vec<u8>, PricedElement), Error> {
    let sort_key_len = match axis {
        IndexAxis::Count | IndexAxis::Sum => 8,
        IndexAxis::Avg => 16,
    };
    let mut key = vec![0u8; sort_key_len];
    key.extend_from_slice(entry_key);
    let payload = match axis {
        IndexAxis::Count => i64::try_from(count).unwrap_or(i64::MAX),
        IndexAxis::Sum | IndexAxis::Avg => sum,
    };
    let row = Element::new_reference_with_sum_item_with_hops(
        SiblingReference(entry_key.to_vec()),
        Some(1),
        payload,
    );
    Ok((
        key,
        PricedElement::Serialized {
            serialized_len: serialized_len(&row, platform_version)?,
        },
    ))
}

/// The `(count, sum)` an empty tree of `tree_type` holds as an entry of a
/// ranked tree (grovedb `Element::count_sum_value_or_default`).
fn empty_tree_aggregate(tree_type: TreeType) -> (u64, i64) {
    match tree_type {
        TreeType::CountTree
        | TreeType::CountSumTree
        | TreeType::ProvableCountTree
        | TreeType::ProvableCountSumTree
        | TreeType::ProvableCountProvableSumTree
        | TreeType::ProvableCountIndexedTree
        | TreeType::ProvableCountProvableSumIndexedTree => (0, 0),
        _ => (1, 0),
    }
}

/// The tree a ranked tree's secondary rows live in.
pub(crate) const RANKING_ROW_TREE: TreeType = TreeType::ProvableCountProvableSumTree;

/// What the walk needs besides the document type.
struct Context<'a> {
    document_type: DocumentTypeRef<'a>,
    document: &'a Document,
    platform_version: &'a PlatformVersion,
    index_paths: Vec<(String, Vec<String>)>,
    /// The flags every document element carries; none for a document with
    /// a `ttl` (`without_storage_flags_if_expiring`).
    document_flags: Option<StorageFlags>,
    /// The flags the index trees and indexOnly entries carry, when they
    /// carry any (`add_indices_for_top_index_level_for_contract_operations`).
    index_flags: Option<StorageFlags>,
    /// The type whose tree the walk is in, when it is a referring type's.
    referring_type: Option<String>,
    /// The `skipIfAbsent` indexes that skip the document: they write
    /// nothing, so no write is theirs.
    skipped_indexes: BTreeSet<String>,
    writes: Vec<Write>,
    /// Where each recorded write is, to record it once.
    recorded: HashSet<WriteLocation>,
}

/// Where a write is: the referring type whose tree it is in (if any),
/// whether it is in the expirations tree, its path and its key.
type WriteLocation = (Option<String>, bool, Vec<Vec<u8>>, Vec<u8>);

fn serialized_len(element: &Element, platform_version: &PlatformVersion) -> Result<u32, Error> {
    Ok(element.serialized_size(&platform_version.drive.grove_version)? as u32)
}

fn flags_len(flags: Option<&StorageFlags>) -> Option<u32> {
    flags.map(|flags| flags.serialized_size())
}

/// A `summableOffCountIndex` index's counter, a sum item carrying `flags`.
fn counter(flags: Option<&StorageFlags>) -> PricedElement {
    PricedElement::SumItem {
        flags_len: flags_len(flags),
    }
}

fn empty_tree(tree_type: TreeType, wrapped: bool, flags: Option<&StorageFlags>) -> PricedElement {
    PricedElement::Tree {
        tree_type,
        wrapped,
        flags_len: flags_len(flags),
    }
}

/// Whether `document` carries `property`, a skip property (the walkers'
/// `document_carries`). A derived index property's value is read from the
/// document a reference points at, which the caller may not have put in: it is
/// then carried while the reference is, its key priced at the field's typical
/// size ([`Context::raw`]), and absent with the reference.
fn carries(document_type: DocumentTypeRef, document: &Document, property: &str) -> bool {
    match document_type.derived_index_properties().get(property) {
        Some(derived) if !document.properties().contains_key(property) => {
            document_carries(document, &derived.reference_property)
        }
        _ => document_carries(document, property),
    }
}

/// Every element inserting `document`, serialized as `serialized`, writes,
/// owned by its owner in one epoch, as a document create stores it.
pub(crate) fn document_writes(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    document: &Document,
    serialized: &[u8],
    platform_version: &PlatformVersion,
) -> Result<Vec<Write>, Error> {
    let document_flags = document_type
        .documents_ttl_seconds()
        .is_none()
        .then(|| StorageFlags::new_single_epoch(0, Some(document.owner_id().to_buffer())));
    let index_flags = document_flags.clone().filter(|_| {
        document_type.documents_mutable()
            || contract.config().can_be_deleted()
            || (document_type.index_only() && document_type.documents_can_be_deleted())
    });
    let skipped_indexes = document_type
        .indexes()
        .values()
        .filter(|index| {
            !index
                .skip_if_absent_properties
                .iter()
                .all(|property| carries(document_type, document, property))
        })
        .map(|index| index.name.clone())
        .collect();
    let mut context = Context {
        document_type,
        document,
        platform_version,
        index_paths: index_paths(document_type),
        document_flags,
        index_flags,
        referring_type: None,
        skipped_indexes,
        writes: Vec::new(),
        recorded: HashSet::new(),
    };
    if !document_type.index_only() {
        context.primary(contract, serialized)?;
    }
    for (level_key, level) in document_type.index_structure().sub_levels() {
        context.top_level(level_key, level)?;
    }
    context.preallocations(contract)?;
    if let Some(ttl_seconds) = document_type.documents_ttl_seconds() {
        context.expiration(contract, ttl_seconds)?;
    }
    Ok(context.writes)
}

impl Context<'_> {
    fn push(&mut self, mut write: Write) {
        write.referring_type = self.referring_type.clone();
        // Indexes sharing a prefix reach the same tree more than once; the
        // walkers insert it once.
        if self.recorded.insert((
            write.referring_type.clone(),
            write.expiration,
            write.path.clone(),
            write.key.clone(),
        )) {
            self.writes.push(write);
        }
    }

    /// The document's contribution to the sum of its value trees at `level`.
    /// A chain carrying a `summableOffCountIndex` index's sums gains one at
    /// every level, as the document's counter does (`counter_write`).
    fn sum_contribution(&self, level: &IndexLevel) -> Result<i64, Error> {
        if level.chain_carries_sums() {
            return Ok(1);
        }
        match level
            .has_index_with_type()
            .and_then(|info| info.summable.as_deref())
        {
            Some(property) => read_document_sum_contribution(self.document, property),
            None => Ok(0),
        }
    }

    /// The rows of the entry `key` under the ranked tree at `path`, one per
    /// axis, for an entry holding `count` and `sum`. Under a time window with
    /// a ttl the rows are `ephemeral`, like the window: Drive writes them in
    /// the window's ephemeral batch, priced as processing.
    #[allow(clippy::too_many_arguments)]
    fn ranking_rows(
        &mut self,
        path: &[Vec<u8>],
        key: &[u8],
        axes: &[IndexAxis],
        count: u64,
        sum: i64,
        indexes: &[String],
        ephemeral: bool,
    ) -> Result<(), Error> {
        for axis in axes {
            let (row_key, element) = ranking_row(*axis, key, count, sum, self.platform_version)?;
            let mut row_path = path.to_vec();
            row_path.push(key.to_vec());
            row_path.push(vec![0xff, *axis as u8]);
            self.push(Write {
                path: row_path,
                key: row_key,
                element,
                parent: RANKING_ROW_TREE,
                role: LayoutRole::IndexValue,
                indexes: indexes.to_vec(),
                // An entry already stored only re-keys its row, which GroveDB
                // bills as replaced bytes, not added ones.
                if_absent: true,
                ephemeral,
                ranking: Some(RankingRow {
                    axis: *axis,
                    entry_path: path.to_vec(),
                    entry_key: key.to_vec(),
                }),
                referring_type: None,
                expiration: false,
                flagged: false,
            });
        }
        Ok(())
    }

    fn raw(&self, property: &str) -> Result<Option<Vec<u8>>, Error> {
        // A derived index property's value is read from the document a reference points at,
        // which an estimate does not have (unless the caller put it in): its key is priced at
        // the typical size of the field's type
        if self
            .document_type
            .derived_index_properties()
            .contains_key(property)
            && !self.document.properties().contains_key(property)
        {
            let size = match self.document_type.derived_index_property_type(property) {
                Some(property_type) => property_type
                    .middle_byte_size_ceil(self.platform_version)?
                    .unwrap_or(DEFAULT_HASH_SIZE_U16),
                None => DEFAULT_HASH_SIZE_U16,
            };
            return Ok(Some(vec![0; usize::from(size)]));
        }
        Ok(self.document.get_raw_for_document_type(
            property,
            self.document_type,
            None,
            self.platform_version,
        )?)
    }

    /// Whether the document takes part in an index whose skip set is
    /// `skip_set` (the walkers' `document_takes_part_in_index`).
    fn takes_part(&self, skip_set: &[String]) -> Result<bool, Error> {
        takes_part_in_index_by(skip_set, &mut |property| {
            Ok(carries(self.document_type, self.document, property))
        })
    }

    /// The indexes through `names` whose path the document writes: those
    /// that do not skip it.
    fn writing_indexes(&self, names: &[String]) -> Vec<String> {
        indexes_through(&self.index_paths, names)
            .into_iter()
            .filter(|index| !self.skipped_indexes.contains(index))
            .collect()
    }

    /// Whether the document writes an entry at or below `level` (the walkers'
    /// `level_reaches_entry`): only then do they build the level.
    fn reaches_entry(&self, level: &IndexLevel) -> Result<bool, Error> {
        level_reaches_entry_by(level, &mut |property| {
            Ok(carries(self.document_type, self.document, property))
        })
    }

    /// The document by id (`add_document_to_primary_storage`).
    fn primary(&mut self, contract: &DataContract, serialized: &[u8]) -> Result<(), Error> {
        let document_type = self.document_type;
        let primary_key_tree_type = document_type.primary_key_tree_type(self.platform_version)?;
        let serialized = serialized.to_vec();
        let sum_property = document_type.documents_summable();
        let document_flags = self.document_flags.clone();
        let id = self.document.id().to_vec();

        if !document_type.documents_keep_history() {
            let element = match sum_property {
                Some(_) => PricedElement::ItemWithSumItem {
                    item_len: serialized.len() as u32,
                    flags_len: flags_len(document_flags.as_ref()),
                },
                None => PricedElement::Serialized {
                    serialized_len: serialized_len(
                        &Element::Item(
                            serialized,
                            StorageFlags::map_to_some_element_flags(document_flags.as_ref()),
                        ),
                        self.platform_version,
                    )?,
                },
            };
            self.push(Write {
                path: vec![vec![0]],
                key: id,
                element,
                parent: primary_key_tree_type,
                role: LayoutRole::Document,
                indexes: vec![],
                if_absent: false,
                ephemeral: false,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: document_flags.is_some(),
            });
            return Ok(());
        }

        // With history: a tree per document holding each revision by time
        // and a pointer to the newest. The tree and the pointer carry flags
        // only when the contract can be deleted.
        let tree_flags = document_flags
            .clone()
            .filter(|_| contract.config().can_be_deleted());
        let document_tree_type = if sum_property.is_some() {
            TreeType::SumTree
        } else {
            TreeType::NormalTree
        };
        self.push(Write {
            path: vec![vec![0]],
            key: id.clone(),
            element: empty_tree(document_tree_type, false, tree_flags.as_ref()),
            parent: primary_key_tree_type,
            role: LayoutRole::Document,
            indexes: vec![],
            if_absent: false,
            ephemeral: false,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: tree_flags.is_some(),
        });
        // The revision key is the block time, 8 bytes whatever the time.
        let encoded_time = DocumentPropertyType::encode_date_timestamp(0);
        self.push(Write {
            path: vec![vec![0], id.clone()],
            key: encoded_time.clone(),
            element: PricedElement::Serialized {
                serialized_len: serialized_len(
                    &Element::Item(
                        serialized,
                        StorageFlags::map_to_some_element_flags(document_flags.as_ref()),
                    ),
                    self.platform_version,
                )?,
            },
            parent: document_tree_type,
            role: LayoutRole::Revision,
            indexes: vec![],
            if_absent: false,
            ephemeral: false,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: document_flags.is_some(),
        });
        let pointer_flags = StorageFlags::map_to_some_element_flags(tree_flags.as_ref());
        let pointer = match sum_property {
            Some(property) => Element::new_reference_with_sum_item_with_max_hops_and_flags(
                SiblingReference(encoded_time),
                Some(1),
                read_document_sum_contribution(self.document, property)?,
                pointer_flags,
            ),
            None => Element::Reference(SiblingReference(encoded_time), Some(1), pointer_flags),
        };
        self.push(Write {
            path: vec![vec![0], id],
            key: vec![0],
            element: PricedElement::Serialized {
                serialized_len: serialized_len(&pointer, self.platform_version)?,
            },
            parent: document_tree_type,
            role: LayoutRole::LatestRevision,
            indexes: vec![],
            if_absent: false,
            ephemeral: false,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: tree_flags.is_some(),
        });
        Ok(())
    }

    /// A first-level index tree (created with the contract) and what the
    /// document writes under it (`add_indices_for_top_index_level_for_contract_operations`).
    fn top_level(&mut self, level_key: &str, level: &IndexLevel) -> Result<(), Error> {
        let tree_types = index_level_tree_types_with_continuation_demotion(level)?;
        let path = vec![level_key.as_bytes().to_vec()];
        let names = vec![level_key.to_string()];

        if is_flat_level_key(level_key) {
            let Some(info) = level.has_index_with_type() else {
                return Ok(());
            };
            let index_flags = self.index_flags.clone();
            return self.terminal(
                &path,
                &names,
                tree_types.property_name_tree_type,
                info,
                false,
                false,
                index_flags.as_ref(),
                false,
            );
        }

        // A branch under which the document writes no entry (every index
        // through it skips the document) is not entered.
        if !self.reaches_entry(level)? {
            return Ok(());
        }

        let property = level
            .bucketing()
            .map(|bucketing| bucketing.source().to_string())
            .unwrap_or_else(|| level_key.to_string());
        let raw = match self.raw(&property)? {
            Some(raw) => raw,
            None if index_only_level_skips_when_absent(self.document_type, &property) => {
                return Ok(())
            }
            None => Vec::new(),
        };
        let null = raw.is_empty();
        let keys = match level.bucketing() {
            Some(bucketing) => bucketing.entry_keys_for_raw(&raw),
            None => vec![raw],
        };
        // A time window with a ttl is ephemeral: no flags, priced as
        // processing.
        let ephemeral = level
            .time_range()
            .is_some_and(|transform| transform.ttl_seconds.is_some());
        let flags = if ephemeral {
            None
        } else {
            self.index_flags.clone()
        };
        // A summableOffCountIndex index never ends at its first property: it
        // holds every property of its source and at least one the source
        // fixes, so one property would repeat its source's levels
        // (`DuplicateIndexError`). Drive's top-level walk keeps no counter
        // either (`add_reference_for_index_level_for_contract_operations`
        // refuses one).
        if level.summable_off_count_index_info().is_some() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a summableOffCountIndex index ends at its first property",
            )));
        }
        for key in keys {
            self.push(Write {
                path: path.clone(),
                key: key.clone(),
                element: empty_tree(tree_types.value_tree_type, false, flags.as_ref()),
                parent: tree_types.property_name_tree_type,
                role: LayoutRole::IndexValue,
                indexes: self.writing_indexes(&names),
                if_absent: true,
                ephemeral,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: flags.is_some(),
            });
            // The first document under a value leaves it a count of one and
            // its own sum.
            let indexes = self.writing_indexes(&names);
            let sum = self.sum_contribution(level)?;
            self.ranking_rows(
                &path,
                &key,
                &tree_types.ranked_axes,
                1,
                sum,
                &indexes,
                ephemeral,
            )?;
            let mut value_path = path.clone();
            value_path.push(key);
            self.level(
                &value_path,
                &names,
                level,
                tree_types.value_tree_type,
                null,
                null,
                flags.as_ref(),
                ephemeral,
            )?;
        }
        Ok(())
    }

    /// What a document writes under one of its value trees: the terminal
    /// of an index ending here, and the continuations of longer indexes
    /// (`add_indices_for_index_level_for_contract_operations` v2).
    #[allow(clippy::too_many_arguments)]
    fn level(
        &mut self,
        path: &[Vec<u8>],
        names: &[String],
        level: &IndexLevel,
        value_tree_type: TreeType,
        any_null: bool,
        all_null: bool,
        flags: Option<&StorageFlags>,
        ephemeral: bool,
    ) -> Result<(), Error> {
        if let Some(info) = level.has_index_with_type() {
            // The index ending here writes its entry only for a document it
            // does not skip.
            if self.takes_part(&info.skip_if_absent_properties)? {
                self.terminal(
                    path,
                    names,
                    value_tree_type,
                    info,
                    any_null,
                    all_null,
                    flags,
                    ephemeral,
                )?;
            }
        }
        let parent_counts_continuations = level.is_ranked_chain_level();
        for (sub_key, sub_level) in level.sub_levels() {
            // A sub-level under which the document writes no entry is not
            // built.
            if !self.reaches_entry(sub_level)? {
                continue;
            }
            let sub_tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let wrapped = continuation_contributes_zero(
                value_tree_type,
                parent_counts_continuations,
                sub_level,
            ) && zero_contribution_wrapper(
                value_tree_type,
                sub_tree_types.property_name_tree_type,
            )
            .map_err(|refusal| {
                Error::Drive(DriveError::CorruptedContractIndexes(format!(
                    "index level {sub_key:?} cannot hang under its value tree: {refusal:?}"
                )))
            })?
            .is_some();
            let mut sub_names = names.to_vec();
            sub_names.push(sub_key.clone());
            let indexes = self.writing_indexes(&sub_names);
            self.push(Write {
                path: path.to_vec(),
                key: sub_key.as_bytes().to_vec(),
                element: empty_tree(sub_tree_types.property_name_tree_type, wrapped, flags),
                parent: value_tree_type,
                role: LayoutRole::NextIndexProperty,
                indexes: indexes.clone(),
                if_absent: true,
                ephemeral,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: flags.is_some(),
            });
            let raw = self.raw(sub_key)?.unwrap_or_default();
            let null = raw.is_empty();
            let mut property_path = path.to_vec();
            property_path.push(sub_key.as_bytes().to_vec());
            if sub_level.summable_off_count_index_info().is_some() {
                let indexes = self.writing_indexes(&sub_names);
                self.counter_write(
                    &property_path,
                    raw,
                    sub_tree_types.property_name_tree_type,
                    &sub_tree_types.ranked_axes,
                    indexes,
                    1,
                    flags,
                    ephemeral,
                )?;
                continue;
            }
            self.push(Write {
                path: property_path.clone(),
                key: raw.clone(),
                element: empty_tree(sub_tree_types.value_tree_type, false, flags),
                parent: sub_tree_types.property_name_tree_type,
                role: LayoutRole::IndexValue,
                indexes: indexes.clone(),
                if_absent: true,
                ephemeral,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: flags.is_some(),
            });
            let sum = self.sum_contribution(sub_level)?;
            self.ranking_rows(
                &property_path,
                &raw,
                &sub_tree_types.ranked_axes,
                1,
                sum,
                &indexes,
                ephemeral,
            )?;
            property_path.push(raw);
            self.level(
                &property_path,
                &sub_names,
                sub_level,
                sub_tree_types.value_tree_type,
                any_null || null,
                all_null && null,
                flags,
                ephemeral,
            )?;
        }
        Ok(())
    }

    /// A `summableOffCountIndex` index's counter at `key` under the
    /// property-name tree at `path`, holding `sum`: inserted the first time,
    /// rewritten in place (at its fixed size, adding no storage) after that,
    /// so it adds storage only when absent; the processing of the rewrite is
    /// counted whether or not it is absent (`processing_costs`). Its ranking
    /// rows hold the group's one count and `sum`: a document's entry adds one,
    /// a preallocated counter starts at zero. A document reaches it only when
    /// the index does not skip it (`reaches_entry`).
    #[allow(clippy::too_many_arguments)]
    fn counter_write(
        &mut self,
        path: &[Vec<u8>],
        key: Vec<u8>,
        property_name_tree_type: TreeType,
        ranked_axes: &[IndexAxis],
        indexes: Vec<String>,
        sum: i64,
        flags: Option<&StorageFlags>,
        ephemeral: bool,
    ) -> Result<(), Error> {
        self.push(Write {
            path: path.to_vec(),
            key: key.clone(),
            element: counter(flags),
            parent: property_name_tree_type,
            role: LayoutRole::IndexValue,
            indexes: indexes.clone(),
            if_absent: true,
            ephemeral,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: flags.is_some(),
        });
        self.ranking_rows(path, &key, ranked_axes, 1, sum, &indexes, ephemeral)
    }

    /// A document with a `ttl`'s entry in the documents expirations tree:
    /// the tree of the documents expiring at its expiry time, and in it the
    /// document's contract and type by its id, without flags
    /// (`add_document_expiration_operations`).
    fn expiration(&mut self, contract: &DataContract, ttl_seconds: u32) -> Result<(), Error> {
        let created_at = self.document.created_at().unwrap_or_default();
        let time_key =
            encode_expiration_time(document_expires_at(created_at, ttl_seconds)?).to_vec();
        let entry = DocumentExpirationEntry {
            contract_id: contract.id(),
            document_type_name: self.document_type.name().clone(),
        };
        self.push(Write {
            path: vec![],
            key: time_key.clone(),
            element: empty_tree(TreeType::NormalTree, false, None),
            parent: TreeType::NormalTree,
            role: LayoutRole::IndexValue,
            indexes: vec![],
            if_absent: true,
            ephemeral: false,
            ranking: None,
            referring_type: None,
            expiration: true,
            flagged: false,
        });
        self.push(Write {
            path: vec![time_key],
            key: self.document.id().to_vec(),
            element: PricedElement::Serialized {
                serialized_len: serialized_len(
                    &Element::Item(entry.to_bytes(), None),
                    self.platform_version,
                )?,
            },
            parent: TreeType::NormalTree,
            role: LayoutRole::Member,
            indexes: vec![],
            if_absent: false,
            ephemeral: false,
            ranking: None,
            referring_type: None,
            expiration: true,
            flagged: false,
        });
        Ok(())
    }

    /// The trees of every preallocated index (on an indexOnly type of the
    /// contract) whose entries will reference the document, created with it
    /// and charged to its creator (`add_preallocated_index_tree_operations`).
    fn preallocations(&mut self, contract: &DataContract) -> Result<(), Error> {
        let target = self.document_type;
        // Preallocated trees are only deleted with the contract, so they
        // carry flags only when it can be.
        let flags = self
            .document_flags
            .clone()
            .filter(|_| contract.config().can_be_deleted());
        for (referring, index, binding) in preallocation_bindings_targeting(contract, target) {
            self.referring_type = Some(referring.name().clone());
            let result = self.preallocation(referring, index, &binding.key_sources, flags.as_ref());
            self.referring_type = None;
            result?;
        }
        Ok(())
    }

    /// The trees of one preallocated index for entries referencing the
    /// document: each level's key resolved from the document, then the
    /// property-name trees below the first, the value trees and the empty
    /// member tree.
    fn preallocation(
        &mut self,
        referring: DocumentTypeRef,
        index: &Index,
        key_sources: &[PreallocatedKeySource],
        flags: Option<&StorageFlags>,
    ) -> Result<(), Error> {
        let mut levels = Vec::with_capacity(index.properties.len());
        let mut current = referring.index_structure();
        for (property, source) in index.properties.iter().zip(key_sources) {
            let name = property.name.as_str();
            let Some(sub_level) = current.sub_levels().get(name) else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a preallocated index's property must exist in the index structure",
                )));
            };
            let raw = match *source {
                PreallocatedKeySource::ReferencedDocumentId => self.raw("$id")?,
                PreallocatedKeySource::ReferencedDocumentProperty(referenced) => {
                    let Some(referring_property) = referring.flattened_properties().get(name)
                    else {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a preallocated index's property must be a property of its \
                             document type",
                        )));
                    };
                    if !bound_value_fits_referring_property(
                        self.document,
                        referenced,
                        &referring_property.property_type,
                        self.platform_version,
                    )? {
                        return Ok(());
                    }
                    self.raw(referenced)?
                }
            };
            // A value the document does not carry, or a null one: no entry
            // can agree with it, so nothing is preallocated.
            match raw {
                Some(raw) if !raw.is_empty() => levels.push((name, sub_level, raw)),
                _ => return Ok(()),
            }
            current = sub_level;
        }
        let Some(info) = current.has_index_with_type() else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a preallocated index must terminate at its last property",
            )));
        };

        let indexes = vec![index.name.clone()];
        let mut path: Vec<Vec<u8>> = Vec::new();
        let mut parent_value_tree_type = TreeType::NormalTree;
        let mut parent_counts_continuations = false;
        for (position, (name, sub_level, raw)) in levels.into_iter().enumerate() {
            let tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            if position > 0 {
                let wrapped = continuation_contributes_zero(
                    parent_value_tree_type,
                    parent_counts_continuations,
                    sub_level,
                ) && zero_contribution_wrapper(
                    parent_value_tree_type,
                    tree_types.property_name_tree_type,
                )
                .map_err(|refusal| {
                    Error::Drive(DriveError::CorruptedContractIndexes(format!(
                        "index level {name:?} cannot hang under its value tree: {refusal:?}"
                    )))
                })?
                .is_some();
                self.push(Write {
                    path: path.clone(),
                    key: name.as_bytes().to_vec(),
                    element: empty_tree(tree_types.property_name_tree_type, wrapped, flags),
                    parent: parent_value_tree_type,
                    role: LayoutRole::NextIndexProperty,
                    indexes: indexes.clone(),
                    if_absent: true,
                    ephemeral: false,
                    ranking: None,
                    referring_type: None,
                    expiration: false,
                    flagged: flags.is_some(),
                });
            }
            path.push(name.as_bytes().to_vec());
            // A `summableOffCountIndex` index's counter, created at zero in
            // place of the value tree and its empty terminal: it counts one
            // group and nothing towards the sum.
            if info.is_summable_off_count_index() && position + 1 == index.properties.len() {
                // A preallocated index is never under a time window.
                return self.counter_write(
                    &path,
                    raw,
                    tree_types.property_name_tree_type,
                    &tree_types.ranked_axes,
                    indexes.clone(),
                    0,
                    flags,
                    false,
                );
            }
            self.push(Write {
                path: path.clone(),
                key: raw.clone(),
                element: empty_tree(tree_types.value_tree_type, false, flags),
                parent: tree_types.property_name_tree_type,
                role: LayoutRole::IndexValue,
                indexes: indexes.clone(),
                if_absent: true,
                ephemeral: false,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: flags.is_some(),
            });
            // An empty value tree ranks with its empty aggregate.
            let (count, sum) = empty_tree_aggregate(tree_types.value_tree_type);
            // A preallocated index is never under a time window.
            self.ranking_rows(
                &path,
                &raw,
                &tree_types.ranked_axes,
                count,
                sum,
                &indexes,
                false,
            )?;
            path.push(raw);
            parent_value_tree_type = tree_types.value_tree_type;
            parent_counts_continuations = sub_level.is_ranked_chain_level();
        }
        self.push(Write {
            path,
            key: vec![0],
            element: empty_tree(terminal_member_tree_type(info), false, flags),
            parent: parent_value_tree_type,
            role: LayoutRole::Terminal,
            indexes,
            if_absent: true,
            ephemeral: false,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: flags.is_some(),
        });
        Ok(())
    }

    /// Where an index ends: its entry for the document
    /// (`add_reference_for_index_level_for_contract_operations`).
    #[allow(clippy::too_many_arguments)]
    fn terminal(
        &mut self,
        path: &[Vec<u8>],
        names: &[String],
        value_tree_type: TreeType,
        info: &IndexLevelTypeInfo,
        any_null: bool,
        all_null: bool,
        flags: Option<&StorageFlags>,
        ephemeral: bool,
    ) -> Result<(), Error> {
        if all_null && !info.should_insert_with_all_null {
            return Ok(());
        }
        let indexes = index_ending_at(&self.index_paths, names);
        let member_tree_type = terminal_member_tree_type(info);
        let mut members_path = path.to_vec();
        members_path.push(vec![0]);

        if let Some(terminal) = info.terminal.as_deref() {
            // indexOnly: a tree of entries keyed by the terminal components,
            // each holding the row commitment and the entry payload.
            self.push(Write {
                path: path.to_vec(),
                key: vec![0],
                element: empty_tree(member_tree_type, false, flags),
                parent: value_tree_type,
                role: LayoutRole::Terminal,
                indexes: indexes.clone(),
                if_absent: true,
                ephemeral,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: flags.is_some(),
            });
            let member_key = index_only_member_key(
                self.document,
                self.document_type,
                terminal,
                None,
                self.platform_version,
            )?;
            let mut item = index_only_row_commitment(
                self.document,
                self.document_type,
                self.platform_version,
            )?
            .to_vec();
            item.extend(encode_index_only_entry_payload(
                self.document,
                self.document_type,
            )?);
            let element_flags = StorageFlags::map_to_some_element_flags(flags);
            let element = match info.summable.as_deref() {
                Some(_) => PricedElement::ItemWithSumItem {
                    item_len: item.len() as u32,
                    flags_len: flags_len(flags),
                },
                None => PricedElement::Serialized {
                    serialized_len: serialized_len(
                        &Element::new_item_with_flags(item, element_flags),
                        self.platform_version,
                    )?,
                },
            };
            self.push(Write {
                path: members_path,
                key: member_key,
                element,
                parent: member_tree_type,
                role: LayoutRole::Member,
                indexes,
                if_absent: false,
                ephemeral,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: flags.is_some(),
            });
            return Ok(());
        }

        // References carry the document's own flags, except under a time
        // window with a ttl, whose elements Drive strips of every flag
        // (`retag_ephemeral_with`).
        let reference_flags = if ephemeral {
            None
        } else {
            self.document_flags.clone()
        };
        let reference = match info.summable.as_deref() {
            Some(property) => make_document_reference_with_sum_item(
                self.document,
                self.document_type,
                read_document_sum_contribution(self.document, property)?,
                reference_flags.as_ref(),
            ),
            None => {
                make_document_reference(self.document, self.document_type, reference_flags.as_ref())
            }
        };
        let reference = PricedElement::Serialized {
            serialized_len: serialized_len(&reference, self.platform_version)?,
        };

        if info.index_type.is_unique() && !any_null {
            self.push(Write {
                path: path.to_vec(),
                key: vec![0],
                element: reference,
                parent: value_tree_type,
                role: LayoutRole::Terminal,
                indexes,
                if_absent: false,
                ephemeral,
                ranking: None,
                referring_type: None,
                expiration: false,
                flagged: reference_flags.is_some(),
            });
            return Ok(());
        }

        self.push(Write {
            path: path.to_vec(),
            key: vec![0],
            element: empty_tree(member_tree_type, false, flags),
            parent: value_tree_type,
            role: LayoutRole::Terminal,
            indexes: indexes.clone(),
            if_absent: true,
            ephemeral,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: flags.is_some(),
        });
        self.push(Write {
            path: members_path,
            key: self.document.id().to_vec(),
            element: reference,
            parent: member_tree_type,
            role: LayoutRole::Member,
            indexes,
            if_absent: false,
            ephemeral,
            ranking: None,
            referring_type: None,
            expiration: false,
            flagged: reference_flags.is_some(),
        });
        Ok(())
    }
}
