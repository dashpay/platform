//! What creating one document costs: the storage every element the insert
//! writes adds (exact, from the elements Drive builds and GroveDB's byte
//! formulas), split by index into the layers an index shares with others
//! and the layers only it uses; the processing (the fixed charges of a
//! signed batch, exact, and the work of the writes, estimated for a given
//! number of stored documents); what the contract adds (action fees, a
//! token cost, a contest's vote fund); and what a delete refunds.
//!
//! Two scenarios bound the storage: every index value new (the first
//! document with these values creates their trees) and every value already
//! stored (a later document with the same values adds only its own entries;
//! a unique index still adds its value, which no earlier document can hold).
//!
//! The storage is held to real inserts by `tests::should_price_what_drive_charges`.

mod document;
mod grove_costs;
mod writes;

#[cfg(feature = "server")]
pub(crate) use document::value_of;
pub use document::{sized_document, FieldChoice, FieldSize};

use crate::drive::document::cost::grove_costs::{new_element_bytes, NodeKind, PricedElement};
use crate::drive::document::cost::writes::{document_writes, Write};
use crate::drive::document::expiration::pricing::{
    document_expiration_cleanup_fee, document_ttl_credit_per_byte,
};
use crate::drive::document::layout::LayoutRole;
use crate::drive::document::sdk_value::{map, number, text, texts};
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV1Getters, DocumentTypeV2Getters,
};
use dpp::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
#[cfg(feature = "fee-distribution")]
use dpp::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
use dpp::fee::epoch::DEFAULT_EPOCHS_PER_ERA;
use dpp::fee::Credits;
use dpp::identity::KeyType;
use dpp::platform_value::Value;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_amount_on_contract_token::{
    DocumentActionTokenCost, DocumentActionTokenEffect,
};
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::required_vote_resolution_fund_to_join;
use grovedb::element::IndexAxis;
use std::collections::BTreeMap;

/// Credits in one Dash.
pub const CREDITS_PER_DASH: Credits = 100_000_000_000;

/// What the estimate assumes about the network and the transition.
#[derive(Clone, Debug, PartialEq)]
pub struct CostAssumptions {
    /// Documents of the type already stored, each with its own values: the
    /// trees an insert walks are as deep as that many entries make them.
    pub existing_documents: u64,
    /// The type of the key the transition is signed with.
    pub signature_key_type: KeyType,
    /// The fee increase the transition asks for, in percent of the
    /// processing fee.
    pub user_fee_increase: u16,
    /// The epoch's fee multiplier in permille, for action fees priced by it.
    pub fee_multiplier_permille: u64,
    /// Contenders already in the contest, for a contested create.
    pub contenders: u16,
}

impl CostAssumptions {
    /// A thousand stored documents, an ECDSA key, no fee increase, the
    /// version's fee multiplier and an empty contest.
    pub fn new(platform_version: &PlatformVersion) -> Self {
        CostAssumptions {
            existing_documents: 1_000,
            signature_key_type: KeyType::ECDSA_SECP256K1,
            user_fee_increase: 0,
            fee_multiplier_permille: platform_version
                .fee_version
                .uses_version_fee_multiplier_permille
                .unwrap_or(1_000),
            contenders: 0,
        }
    }
}

/// A byte count or an amount under both scenarios.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scenarios {
    /// Every index value new.
    pub new_values: u64,
    /// Every value an earlier document can hold already stored.
    pub known_values: u64,
}

impl Scenarios {
    fn add(&mut self, other: Scenarios) {
        self.new_values = self.new_values.saturating_add(other.new_values);
        self.known_values = self.known_values.saturating_add(other.known_values);
    }

    fn scale(self, factor: u64) -> Scenarios {
        Scenarios {
            new_values: self.new_values.saturating_mul(factor),
            known_values: self.known_values.saturating_mul(factor),
        }
    }
}

/// One element the insert writes, and the storage it adds.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementCost {
    /// What the element is.
    pub role: LayoutRole,
    /// The element's path below the document type tree, and its key, as
    /// readable text.
    pub path: Vec<String>,
    /// The storage bytes the element adds when it is written.
    pub bytes: u64,
    /// The indexes that use the element; empty for primary storage.
    pub indexes: Vec<String>,
    /// Written only when absent: a tree an earlier document with the same
    /// values created.
    pub if_absent: bool,
    /// Written even when every value is already stored.
    pub written_when_values_known: bool,
    /// On a time window with a `ttl`: priced as processing, not storage.
    pub ephemeral: bool,
    /// The document type whose tree the element is in, when it is not the
    /// created document's: a preallocated index of a type referring to it.
    pub referring_type: Option<String>,
    /// In the documents expirations tree: a document with a `ttl`'s entry.
    pub expiration: bool,
    /// The axis, when the element is a ranked tree's row for the value.
    pub ranking_axis: Option<IndexAxis>,
}

/// The storage one index adds.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexCost {
    /// The index.
    pub name: String,
    /// The indexes it shares layers with (a common prefix of properties).
    pub shared_with: Vec<String>,
    /// The bytes of the layers it shares; counted once for the document.
    pub shared_bytes: Scenarios,
    /// The bytes of the layers only it uses.
    pub own_bytes: Scenarios,
}

/// A part of the processing fee.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingCost {
    /// A stable code.
    pub code: &'static str,
    /// What it is.
    pub text: String,
    /// The credits.
    pub credits: Scenarios,
    /// Whether the amount is exact, or estimated from the assumptions.
    pub exact: bool,
}

/// A charge the contract adds to the create.
#[derive(Clone, Debug, PartialEq)]
pub enum ContractCharge {
    /// The create's action fee, paid into the contract's fee pots.
    ActionFee {
        /// The amounts the contract declares.
        declared: DocumentActionFee,
        /// How they are priced.
        pricing: ActionFeePricing,
        /// The amounts charged at the assumed fee multiplier.
        charged: DocumentActionFee,
    },
    /// The create's token cost, in tokens.
    TokenCost(DocumentActionTokenCost),
    /// The vote fund a contender pays when the value is contested. Such a
    /// create is stored in the contest's vote poll until the contest ends,
    /// not in the index: the storage priced here is an uncontested create's.
    ContestFund {
        /// The contested index.
        index: String,
        /// The credits, at the assumed number of contenders.
        credits: Credits,
    },
}

/// What creating one document of a type costs.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentCreateCost {
    /// The document type name.
    pub document_type: String,
    /// What the estimate assumes.
    pub assumptions: CostAssumptions,
    /// The serialized document, in bytes.
    pub document_bytes: u64,
    /// Credits per stored byte.
    pub credits_per_byte: Credits,
    /// Every element the insert writes.
    pub elements: Vec<ElementCost>,
    /// The bytes of primary storage (the document by id).
    pub primary_bytes: Scenarios,
    /// The bytes of the trees preallocated for entries of other types that
    /// will reference the document.
    pub preallocated_bytes: Scenarios,
    /// The bytes of a document with a `ttl`'s entry in the documents
    /// expirations tree.
    pub expiration_bytes: Scenarios,
    /// The bytes each index adds.
    pub indexes: Vec<IndexCost>,
    /// All the storage bytes.
    pub storage_bytes: Scenarios,
    /// The storage fee.
    pub storage_credits: Scenarios,
    /// The processing fee, part by part (the fee increase included).
    pub processing: Vec<ProcessingCost>,
    /// What the contract adds.
    pub contract_charges: Vec<ContractCharge>,
    /// The storage fee a delete in the same epoch refunds (with the
    /// `fee-distribution` feature).
    pub refund_same_epoch: Option<Scenarios>,
    /// The storage fee a delete a year (an era of epochs) later refunds
    /// (with the `fee-distribution` feature).
    pub refund_after_one_year: Option<Scenarios>,
    /// How each field of the priced document was filled, when it was built
    /// from sizes ([`document_type_create_cost`]).
    pub fields: Vec<FieldSize>,
}

impl DocumentCreateCost {
    /// The processing fee.
    pub fn processing_credits(&self) -> Scenarios {
        let mut total = Scenarios::default();
        for part in &self.processing {
            total.add(part.credits);
        }
        total
    }

    /// The credits the create costs in fees and action fees, without a
    /// token cost or a contest fund.
    pub fn total_credits(&self) -> Scenarios {
        let mut total = self.storage_credits;
        total.add(self.processing_credits());
        for charge in &self.contract_charges {
            if let ContractCharge::ActionFee { charged, .. } = charge {
                let credits = charged.owner.saturating_add(charged.moderators);
                total.add(Scenarios {
                    new_values: credits,
                    known_values: credits,
                });
            }
        }
        total
    }
}

/// What creating `document` as a document of `document_type` costs under
/// `assumptions`.
///
/// Follows the insert methods of protocol version 14, like
/// `drive::document::layout`; a version with other ones is refused. A
/// document with a `ttl` pays for its bytes by its lifetime, carries no flags
/// (so nothing is refunded) and prepays its deletion. The storage is an
/// uncontested create's: a value that starts a contest is stored in the
/// contest's vote poll until it ends, which this does not price.
pub fn document_create_cost(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    document: &Document,
    assumptions: &CostAssumptions,
    platform_version: &PlatformVersion,
) -> Result<DocumentCreateCost, Error> {
    check_mirrored_method_versions(platform_version)?;
    let fee_version = &platform_version.fee_version;
    // A document with a `ttl` pays for its bytes by the lifetime it has
    // left, all of its `ttl` when it is created.
    let credits_per_byte = match document_type.documents_ttl_seconds() {
        Some(ttl_seconds) => {
            document_ttl_credit_per_byte(u64::from(ttl_seconds) * 1000, fee_version)?
        }
        None => fee_version.storage.storage_disk_usage_credit_per_byte,
    };
    let serialized = document.serialize(document_type, contract, platform_version)?;
    let document_bytes = serialized.len() as u64;
    let writes = document_writes(
        contract,
        document_type,
        document,
        &serialized,
        platform_version,
    )?;
    let known = written_when_values_known(&writes, document.id().as_slice());

    let mut elements = Vec::with_capacity(writes.len());
    let mut primary_bytes = Scenarios::default();
    let mut preallocated_bytes = Scenarios::default();
    let mut expiration_bytes = Scenarios::default();
    let mut storage_bytes = Scenarios::default();
    let mut ephemeral_bytes = Scenarios::default();
    for (write, known) in writes.iter().zip(known.iter().copied()) {
        let bytes = u64::from(new_element_bytes(
            write.key.len() as u32,
            write.element,
            NodeKind::of_tree(write.parent),
        ));
        let scenarios = Scenarios {
            new_values: bytes,
            known_values: if known { bytes } else { 0 },
        };
        if write.ephemeral {
            ephemeral_bytes.add(scenarios);
        } else {
            storage_bytes.add(scenarios);
            if write.referring_type.is_some() {
                preallocated_bytes.add(scenarios);
            } else if write.expiration {
                expiration_bytes.add(scenarios);
            } else if write.indexes.is_empty() {
                primary_bytes.add(scenarios);
            }
        }
        elements.push(ElementCost {
            role: write.role,
            path: readable_path(write),
            bytes,
            indexes: write.indexes.clone(),
            if_absent: write.if_absent,
            written_when_values_known: known,
            ephemeral: write.ephemeral,
            referring_type: write.referring_type.clone(),
            expiration: write.expiration,
            ranking_axis: write.ranking.as_ref().map(|row| row.axis),
        });
    }

    let indexes = index_costs(document_type, &elements);
    let storage_credits = storage_bytes.scale(credits_per_byte);
    let processing = processing_costs(
        contract,
        document_type,
        &writes,
        &known,
        ephemeral_bytes,
        document_bytes,
        assumptions,
        platform_version,
    )?;
    let contract_charges =
        contract_charges(contract, document_type, assumptions, platform_version)?;

    // A document with a `ttl` carries no flags: nothing of it is refunded.
    let refundable = if document_type.documents_ttl_seconds().is_some() {
        Scenarios::default()
    } else {
        refundable_bytes(&writes, &known, &elements)
    };
    let refund_same_epoch = refund(document_type, refundable, credits_per_byte, 0)?;
    let refund_after_one_year = refund(
        document_type,
        refundable,
        credits_per_byte,
        DEFAULT_EPOCHS_PER_ERA,
    )?;

    Ok(DocumentCreateCost {
        document_type: document_type.name().clone(),
        assumptions: assumptions.clone(),
        document_bytes,
        credits_per_byte,
        elements,
        primary_bytes,
        preallocated_bytes,
        expiration_bytes,
        indexes,
        storage_bytes,
        storage_credits,
        processing,
        contract_charges,
        refund_same_epoch,
        refund_after_one_year,
        fields: Vec::new(),
    })
}

/// Refuses a platform version whose insert methods are not the ones the
/// estimate mirrors (those of protocol version 14): another version of any
/// of them may write other elements.
fn check_mirrored_method_versions(platform_version: &PlatformVersion) -> Result<(), Error> {
    let document = &platform_version.drive.methods.document;
    for (method, known, received) in [
        (
            "add_document_for_contract_operations",
            1,
            document.insert.add_document_for_contract_operations,
        ),
        (
            "add_document_to_primary_storage",
            0,
            document.insert.add_document_to_primary_storage,
        ),
        (
            "add_indices_for_top_index_level_for_contract_operations",
            2,
            document
                .insert
                .add_indices_for_top_index_level_for_contract_operations,
        ),
        (
            "add_indices_for_index_level_for_contract_operations",
            2,
            document
                .insert
                .add_indices_for_index_level_for_contract_operations,
        ),
        (
            "add_reference_for_index_level_for_contract_operations",
            0,
            document
                .insert
                .add_reference_for_index_level_for_contract_operations,
        ),
        (
            "add_document_expiration_operations",
            0,
            document.expiration.add_document_expiration_operations,
        ),
    ] {
        if received != known {
            return Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: format!("document_create_cost ({method})"),
                known_versions: vec![known],
                received,
            }));
        }
    }
    Ok(())
}

/// What creating a document of `document_type` costs, for a document of
/// the sizes `choices` name (every other variable-size value at its middle
/// size, every optional value present).
pub fn document_type_create_cost(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    choices: &std::collections::BTreeMap<String, FieldChoice>,
    assumptions: &CostAssumptions,
    platform_version: &PlatformVersion,
) -> Result<DocumentCreateCost, Error> {
    let (document, fields) = sized_document(contract, document_type, choices, platform_version)?;
    let mut cost = document_create_cost(
        contract,
        document_type,
        &document,
        assumptions,
        platform_version,
    )?;
    cost.fields = fields;
    Ok(cost)
}

/// The storage fee of `bytes` a delete `epochs_later` epochs after the
/// create refunds: what the epochs still to come would have been paid.
#[cfg(feature = "fee-distribution")]
fn refund(
    document_type: DocumentTypeRef,
    bytes: Scenarios,
    credits_per_byte: Credits,
    epochs_later: u16,
) -> Result<Option<Scenarios>, Error> {
    // A document of a type whose owner can not delete it refunds nothing, unless a create
    // may consume it (`canBeDeleted: "onlyWhenConsumed"`), which refunds its owner as the
    // owner's delete would
    if !document_type.documents_can_be_deleted()
        && !document_type.documents_deleted_only_when_consumed()
    {
        return Ok(Some(Scenarios::default()));
    }
    let refund = |bytes: u64| -> Result<Credits, Error> {
        let (refund, _) = calculate_storage_fee_refund_amount_and_leftovers(
            bytes.saturating_mul(credits_per_byte),
            0,
            epochs_later,
            DEFAULT_EPOCHS_PER_ERA,
        )?;
        Ok(refund)
    };
    Ok(Some(Scenarios {
        new_values: refund(bytes.new_values)?,
        known_values: refund(bytes.known_values)?,
    }))
}

#[cfg(not(feature = "fee-distribution"))]
fn refund(
    _document_type: DocumentTypeRef,
    _bytes: Scenarios,
    _credits_per_byte: Credits,
    _epochs_later: u16,
) -> Result<Option<Scenarios>, Error> {
    Ok(None)
}

/// Which writes happen even when every value an earlier document can hold
/// is stored: the ones every insert makes; a document with a `ttl`'s entry
/// in the expirations tree (a later document expires at another time); and
/// a tree no earlier document can have made, with everything under it: the
/// value tree of a unique index with no null value, and a tree keyed by the
/// document's own id (`document_id`), such as a preallocated index's.
fn written_when_values_known(writes: &[Write], document_id: &[u8]) -> Vec<bool> {
    // A tree, named by where its path starts and its full path.
    let tree =
        |write: &Write, path: Vec<Vec<u8>>| (write.referring_type.clone(), write.expiration, path);
    let full_path = |write: &Write| {
        let mut full = write.path.clone();
        full.push(write.key.clone());
        full
    };
    let always_new: Vec<_> = writes
        .iter()
        .filter_map(|write| {
            let unique_value = write.role == LayoutRole::Terminal
                && !write.if_absent
                && matches!(write.element, PricedElement::Serialized { .. })
                && !write.indexes.is_empty();
            if unique_value {
                Some(tree(write, write.path.clone()))
            } else if write.if_absent && write.key == document_id {
                Some(tree(write, full_path(write)))
            } else {
                None
            }
        })
        .collect();
    writes
        .iter()
        .map(|write| {
            if !write.if_absent || write.expiration {
                return true;
            }
            let (area, expiration, full) = tree(write, full_path(write));
            always_new
                .iter()
                .any(|(new_area, new_expiration, new_path)| {
                    *new_area == area && *new_expiration == expiration && full.starts_with(new_path)
                })
        })
        .collect()
}

/// The storage bytes a delete refunds: those of the elements it removes that
/// carry the owner's flags. Not refunded: elements without flags (a ranked
/// tree's rows, trees an index writes without flags), ephemeral elements,
/// and preallocated trees, which a delete keeps.
fn refundable_bytes(writes: &[Write], known: &[bool], elements: &[ElementCost]) -> Scenarios {
    let mut total = Scenarios::default();
    for ((write, known), element) in writes.iter().zip(known).zip(elements) {
        if write.flagged && !write.ephemeral && write.referring_type.is_none() {
            total.add(Scenarios {
                new_values: element.bytes,
                known_values: if *known { element.bytes } else { 0 },
            });
        }
    }
    total
}

fn readable_key(key: &[u8]) -> String {
    match std::str::from_utf8(key) {
        Ok(text) if !text.is_empty() && text.chars().all(|c| !c.is_control()) => text.to_string(),
        _ => format!("0x{}", hex::encode(key)),
    }
}

fn readable_path(write: &Write) -> Vec<String> {
    write
        .referring_type
        .iter()
        .cloned()
        .chain(
            write
                .path
                .iter()
                .chain(std::iter::once(&write.key))
                .map(|key| readable_key(key)),
        )
        .collect()
}

/// Each index's shared and own bytes. A layer used by several indexes is
/// shared by them; the document pays for it once.
fn index_costs(document_type: DocumentTypeRef, elements: &[ElementCost]) -> Vec<IndexCost> {
    document_type
        .indexes()
        .keys()
        .map(|name| {
            let mut shared_with = Vec::new();
            let mut shared_bytes = Scenarios::default();
            let mut own_bytes = Scenarios::default();
            for element in elements.iter().filter(|element| {
                !element.ephemeral
                    && element.referring_type.is_none()
                    && element.indexes.contains(name)
            }) {
                let scenarios = Scenarios {
                    new_values: element.bytes,
                    known_values: if element.written_when_values_known {
                        element.bytes
                    } else {
                        0
                    },
                };
                if element.indexes.len() > 1 {
                    shared_bytes.add(scenarios);
                    for other in element.indexes.iter().filter(|other| *other != name) {
                        if !shared_with.contains(other) {
                            shared_with.push(other.clone());
                        }
                    }
                } else {
                    own_bytes.add(scenarios);
                }
            }
            shared_with.sort();
            IndexCost {
                name: name.clone(),
                shared_with,
                shared_bytes,
                own_bytes,
            }
        })
        .collect()
}

/// The nodes above a new node in a balanced tree of `entries` entries:
/// about the base-2 logarithm of the count.
fn path_height(entries: u64) -> u64 {
    u64::from(64 - entries.leading_zeros())
        .saturating_sub(1)
        .max(u64::from(entries > 0))
}

/// The writes that cost processing, given `written`, those that write an
/// element: those, and also every ranked row and counter. A known value's
/// ranked row adds no storage, but it still moves in its secondary tree (a
/// delete and an insert), which costs processing. A `summableOffCountIndex`
/// counter (the one sum item written) is rewritten in place on every create
/// of its group: no storage, but a put.
fn processed_writes(writes: &[Write], written: &[bool]) -> Vec<bool> {
    writes
        .iter()
        .zip(written)
        .map(|(write, written)| {
            *written
                || write.ranking.is_some()
                || matches!(write.element, PricedElement::SumItem { .. })
        })
        .collect()
}

/// The processing fee, part by part.
#[allow(clippy::too_many_arguments)]
fn processing_costs(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    writes: &[Write],
    known: &[bool],
    ephemeral_bytes: Scenarios,
    document_bytes: u64,
    assumptions: &CostAssumptions,
    platform_version: &PlatformVersion,
) -> Result<Vec<ProcessingCost>, Error> {
    let fee_version = &platform_version.fee_version;
    // The primary key tree and one tree per first index level.
    let type_entries = u64::from(!document_type.index_only())
        + document_type.index_structure().sub_levels().len() as u64;
    let exact = |credits: Credits| Scenarios {
        new_values: credits,
        known_values: credits,
    };
    let known_with_row_moves = processed_writes(writes, known);
    let mut parts = vec![
        ProcessingCost {
            code: "signature",
            text: format!(
                "verifying the transition's {:?} signature",
                assumptions.signature_key_type
            ),
            credits: exact(
                assumptions
                    .signature_key_type
                    .signature_verify_cost(platform_version)?,
            ),
            exact: true,
        },
        ProcessingCost {
            code: "identity",
            text: "fetching the signing key and the balance of the identity".to_string(),
            credits: exact(
                fee_version
                    .processing
                    .fetch_identity_revision_processing_cost
                    .saturating_add(
                        fee_version
                            .processing
                            .fetch_identity_cost_per_look_up_key_by_id,
                    ),
            ),
            exact: true,
        },
        ProcessingCost {
            code: "writes",
            text: format!(
                "writing the elements: seeks, hashing and rewriting the paths to them in trees \
                 of about {} entries",
                assumptions.existing_documents
            ),
            credits: Scenarios {
                new_values: write_processing(
                    writes,
                    &[],
                    type_entries,
                    assumptions,
                    platform_version,
                ),
                known_values: write_processing(
                    writes,
                    &known_with_row_moves,
                    type_entries,
                    assumptions,
                    platform_version,
                ),
            },
            exact: false,
        },
        ProcessingCost {
            code: "checks",
            text: "checking the document does not exist yet and bumping the identity's nonce \
                   for the contract"
                .to_string(),
            credits: exact(checks_processing(assumptions, platform_version)),
            exact: false,
        },
    ];
    let derived_index_reads =
        derived_index_reads_processing(contract, document_type, assumptions, platform_version)?;
    if derived_index_reads > 0 {
        parts.push(ProcessingCost {
            code: "derivedIndexReads",
            text: "reading the documents the derived index properties take their values from, \
                   when their references are validated"
                .to_string(),
            credits: exact(derived_index_reads),
            exact: false,
        });
    }
    if document_type.documents_ttl_seconds().is_some() {
        parts.push(ProcessingCost {
            code: "ttlCleanup",
            text: "the deletion of the document when its ttl runs out, prepaid".to_string(),
            credits: exact(document_expiration_cleanup_fee(
                document_type,
                document_bytes,
                fee_version,
            )?),
            exact: true,
        });
    }
    if ephemeral_bytes.new_values > 0 {
        parts.push(ProcessingCost {
            code: "timeWindowTtl",
            text: "time window entries that expire, priced as processing".to_string(),
            credits: ephemeral_bytes
                .scale(fee_version.storage.ttl_ephemeral_disk_usage_credit_per_byte),
            exact: true,
        });
    }
    if assumptions.user_fee_increase > 0 {
        let mut subtotal = Scenarios::default();
        for part in &parts {
            subtotal.add(part.credits);
        }
        let increase = u64::from(assumptions.user_fee_increase);
        parts.push(ProcessingCost {
            code: "feeIncrease",
            text: format!("the {increase}% fee increase the transition asks for"),
            credits: Scenarios {
                new_values: subtotal.new_values.saturating_mul(increase) / 100,
                known_values: subtotal.known_values.saturating_mul(increase) / 100,
            },
            exact: false,
        });
    }
    Ok(parts)
}

/// The heights of the trees above a document type tree: the contracts'
/// documents (keyed by contract, about a thousand), the contract's own tree
/// and its document types.
const TREES_ABOVE_THE_TYPE: [u64; 3] = [10, 1, 2];

/// The processing of the writes: every tree that gets a new element is
/// walked from its root to the new node and that path is rewritten and
/// rehashed, and so is the element of every tree above it, up to the
/// root; every insert-if-absent first reads the element. `known` (when not
/// empty) leaves out the writes a stored value already made.
///
/// How deep those paths are depends on how many entries each tree holds,
/// assumed from `existing_documents` stored documents with values of their
/// own: the document type tree holds its `type_entries` trees; its documents
/// tree, its first index levels, a ranked tree's rows and the expirations
/// tree hold one entry per document; a tree this insert creates holds none,
/// and any other tree (under one value) the one earlier document with it.
fn write_processing(
    writes: &[Write],
    known: &[bool],
    type_entries: u64,
    assumptions: &CostAssumptions,
    platform_version: &PlatformVersion,
) -> Credits {
    let fee = &platform_version.fee_version;
    let seek = fee.storage.storage_seek_cost;
    let per_written_byte = fee.storage.storage_processing_credit_per_byte;
    let per_loaded_byte = fee.storage.storage_load_credit_per_byte;
    let per_hash = fee
        .hashing
        .blake3_base
        .saturating_add(fee.hashing.blake3_per_block);

    // A tree, named by where its path starts (the created document's type,
    // a referring type, the expirations tree) and its path from there.
    type Tree = (Option<String>, bool, Vec<Vec<u8>>);
    let tree_of = |write: &Write, path: Vec<Vec<u8>>| -> Tree {
        (write.referring_type.clone(), write.expiration, path)
    };

    let written: Vec<&Write> = writes
        .iter()
        .enumerate()
        .filter(|(i, _)| known.is_empty() || known[*i])
        .map(|(_, write)| write)
        .collect();
    let created: Vec<Tree> = written
        .iter()
        .filter(|write| matches!(write.element, PricedElement::Tree { .. }))
        .map(|write| {
            let mut full = write.path.clone();
            full.push(write.key.clone());
            tree_of(write, full)
        })
        .collect();
    let entries_before = |tree: &Tree| -> u64 {
        let (_, expiration, path) = tree;
        let is_ranking_rows = path
            .last()
            .is_some_and(|key| key.len() == 2 && key[0] == 0xff);
        if created.contains(tree) {
            0
        } else if path.is_empty() {
            if *expiration {
                assumptions.existing_documents
            } else {
                type_entries
            }
        } else if (path.len() == 1 && !*expiration) || is_ranking_rows {
            assumptions.existing_documents
        } else {
            1
        }
    };

    let mut credits: Credits = 0;
    // Every insert-if-absent reads the element first, written or not.
    for write in writes.iter().filter(|write| write.if_absent) {
        let bytes = u64::from(new_element_bytes(
            write.key.len() as u32,
            write.element,
            NodeKind::of_tree(write.parent),
        ));
        credits = credits
            .saturating_add(seek)
            .saturating_add(bytes.saturating_mul(per_loaded_byte));
    }
    // Each new element: its put, its bytes and its hashes (value, key-value
    // and node hash).
    let mut touched: Vec<(Tree, u64)> = Vec::new();
    for write in &written {
        let bytes = u64::from(new_element_bytes(
            write.key.len() as u32,
            write.element,
            NodeKind::of_tree(write.parent),
        ));
        credits = credits
            .saturating_add(seek)
            .saturating_add(bytes.saturating_mul(per_written_byte))
            .saturating_add(4 * per_hash);
        // Every tree from the element's tree up to where its path starts
        // has its path rewritten once.
        for depth in (0..=write.path.len()).rev() {
            let tree = tree_of(write, write.path[..depth].to_vec());
            if !touched.iter().any(|(seen, _)| *seen == tree) {
                touched.push((tree, bytes));
            }
        }
    }
    let rewrite = |height: u64, node_bytes: u64| -> Credits {
        height.saturating_mul(
            seek.saturating_add(node_bytes.saturating_mul(per_written_byte + per_loaded_byte))
                .saturating_add(2 * per_hash),
        )
    };
    for (tree, node_bytes) in &touched {
        credits = credits.saturating_add(rewrite(path_height(entries_before(tree)), *node_bytes));
    }
    if !written.is_empty() {
        for height in TREES_ABOVE_THE_TYPE {
            credits = credits.saturating_add(rewrite(height, 150));
        }
    }
    credits
}

/// The reads of the documents a type's derived index properties take their values from
/// (protocol version 14): each referenced document is read once on a create, when its
/// reference is validated, which also gives Drive the values to key the index entries by: a
/// seek down a tree of about as many entries as the assumptions give and a load of the
/// referenced document. Zero for a type without derived index properties.
fn derived_index_reads_processing(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    assumptions: &CostAssumptions,
    platform_version: &PlatformVersion,
) -> Result<Credits, Error> {
    let fee = &platform_version.fee_version;
    let height = path_height(assumptions.existing_documents).max(1);
    let mut referenced_types = BTreeMap::new();
    for derived in document_type.derived_index_properties().values() {
        referenced_types.insert(
            derived.reference_property.as_str(),
            derived.referenced_document_type_name.as_str(),
        );
    }
    let mut credits: Credits = 0;
    for referenced_type_name in referenced_types.into_values() {
        let referenced_type = contract.document_type_for_name(referenced_type_name)?;
        let read = height
            .saturating_mul(fee.storage.storage_seek_cost)
            .saturating_add(
                (referenced_type.estimated_size(platform_version)? as u64)
                    .saturating_mul(fee.storage.storage_load_credit_per_byte),
            );
        credits = credits.saturating_add(read);
    }
    Ok(credits)
}

/// The small reads and writes around the insert: the query that checks the
/// document id is free and the nonce the identity keeps for the contract.
fn checks_processing(assumptions: &CostAssumptions, platform_version: &PlatformVersion) -> Credits {
    let fee = &platform_version.fee_version;
    let height = path_height(assumptions.existing_documents).max(1);
    let per_hash = fee.hashing.blake3_base + fee.hashing.blake3_per_block;
    let lookup =
        height * (fee.storage.storage_seek_cost + 100 * fee.storage.storage_load_credit_per_byte);
    let nonce = fee.storage.storage_seek_cost * 4
        + 60 * fee.storage.storage_processing_credit_per_byte
        + 6 * per_hash;
    lookup + nonce
}

/// The charges the contract adds to a create.
fn contract_charges(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    assumptions: &CostAssumptions,
    platform_version: &PlatformVersion,
) -> Result<Vec<ContractCharge>, Error> {
    let mut charges = Vec::new();
    if let Some(fees) = document_type.action_fees() {
        if let Some(declared) = fees.document_creation_action_fee() {
            let pricing = fees.pricing();
            charges.push(ContractCharge::ActionFee {
                declared,
                pricing,
                charged: declared.charged(pricing, assumptions.fee_multiplier_permille)?,
            });
        }
    }
    if let Some(token_cost) = document_type.document_creation_token_cost() {
        charges.push(ContractCharge::TokenCost(token_cost));
    }
    if let Some(index) = document_type.find_contested_index() {
        charges.push(ContractCharge::ContestFund {
            index: index.name.clone(),
            credits: required_vote_resolution_fund_to_join(
                contract.id_ref(),
                document_type.name(),
                assumptions.contenders,
                platform_version,
            ),
        });
    }
    Ok(charges)
}

fn scenarios(value: Scenarios) -> Value {
    map(vec![
        ("newValues", number(value.new_values)),
        ("knownValues", number(value.known_values)),
    ])
}

fn action_fee(fee: DocumentActionFee) -> Value {
    map(vec![
        ("owner", number(fee.owner)),
        ("moderators", number(fee.moderators)),
    ])
}

impl ContractCharge {
    fn to_value(&self) -> Value {
        match self {
            ContractCharge::ActionFee {
                declared,
                pricing,
                charged,
            } => map(vec![
                ("kind", text("actionFee")),
                (
                    "pricing",
                    text(match pricing {
                        ActionFeePricing::FeeMultiplier => "feeMultiplier",
                        ActionFeePricing::Fixed => "fixed",
                    }),
                ),
                ("declared", action_fee(*declared)),
                ("charged", action_fee(*charged)),
            ]),
            ContractCharge::TokenCost(cost) => {
                let mut entries = vec![
                    ("kind", text("tokenCost")),
                    (
                        "tokenPosition",
                        number(u64::from(cost.token_contract_position)),
                    ),
                    ("amount", number(cost.token_amount)),
                    (
                        "effect",
                        text(match cost.effect {
                            DocumentActionTokenEffect::TransferTokenToContractOwner => {
                                "transferToContractOwner"
                            }
                            DocumentActionTokenEffect::BurnToken => "burn",
                        }),
                    ),
                    (
                        "gasFeesPaidBy",
                        text(match cost.gas_fees_paid_by {
                            GasFeesPaidBy::DocumentOwner => "documentOwner",
                            GasFeesPaidBy::ContractOwner => "contractOwner",
                            GasFeesPaidBy::PreferContractOwner => "preferContractOwner",
                        }),
                    ),
                    ("optional", Value::Bool(cost.optional)),
                ];
                if let Some(contract_id) = cost.contract_id {
                    entries.push((
                        "tokenContractId",
                        text(
                            &contract_id
                                .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                        ),
                    ));
                }
                map(entries)
            }
            ContractCharge::ContestFund { index, credits } => map(vec![
                ("kind", text("contestFund")),
                ("index", text(index)),
                ("credits", number(*credits)),
            ]),
        }
    }
}

impl DocumentCreateCost {
    /// The estimate as a plain value, amounts in credits, a key left out
    /// when it has no value:
    /// `{ documentType, assumptions, documentBytes, creditsPerByte,
    /// creditsPerDash, storage: { bytes, credits, primaryBytes }, indexes,
    /// elements, processing, processingCredits, contractCharges,
    /// refund: { sameEpoch, afterOneYear }, totalCredits }`; every amount
    /// that depends on the scenario is `{ newValues, knownValues }`.
    pub fn to_value(&self) -> Value {
        map(vec![
            ("documentType", text(&self.document_type)),
            (
                "assumptions",
                map(vec![
                    (
                        "existingDocuments",
                        number(self.assumptions.existing_documents),
                    ),
                    (
                        "signatureKeyType",
                        text(&format!("{:?}", self.assumptions.signature_key_type)),
                    ),
                    (
                        "userFeeIncrease",
                        number(u64::from(self.assumptions.user_fee_increase)),
                    ),
                    (
                        "feeMultiplierPermille",
                        number(self.assumptions.fee_multiplier_permille),
                    ),
                    ("contenders", number(u64::from(self.assumptions.contenders))),
                ]),
            ),
            ("documentBytes", number(self.document_bytes)),
            ("creditsPerByte", number(self.credits_per_byte)),
            ("creditsPerDash", number(CREDITS_PER_DASH)),
            (
                "storage",
                map(vec![
                    ("bytes", scenarios(self.storage_bytes)),
                    ("credits", scenarios(self.storage_credits)),
                    ("primaryBytes", scenarios(self.primary_bytes)),
                    ("preallocatedBytes", scenarios(self.preallocated_bytes)),
                    ("expirationBytes", scenarios(self.expiration_bytes)),
                ]),
            ),
            (
                "indexes",
                Value::Array(
                    self.indexes
                        .iter()
                        .map(|index| {
                            map(vec![
                                ("name", text(&index.name)),
                                ("sharedWith", texts(&index.shared_with)),
                                ("sharedBytes", scenarios(index.shared_bytes)),
                                ("ownBytes", scenarios(index.own_bytes)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "elements",
                Value::Array(
                    self.elements
                        .iter()
                        .map(|element| {
                            map(vec![
                                ("role", text(element.role.name())),
                                ("path", texts(&element.path)),
                                ("bytes", number(element.bytes)),
                                ("indexes", texts(&element.indexes)),
                                ("ifAbsent", Value::Bool(element.if_absent)),
                                (
                                    "writtenWhenValuesKnown",
                                    Value::Bool(element.written_when_values_known),
                                ),
                                ("ephemeral", Value::Bool(element.ephemeral)),
                                ("expiration", Value::Bool(element.expiration)),
                                (
                                    "rankingAxis",
                                    element
                                        .ranking_axis
                                        .map(|axis| {
                                            text(match axis {
                                                IndexAxis::Count => "count",
                                                IndexAxis::Sum => "sum",
                                                IndexAxis::Avg => "avg",
                                            })
                                        })
                                        .unwrap_or(Value::Null),
                                ),
                                (
                                    "referringType",
                                    element
                                        .referring_type
                                        .as_deref()
                                        .map(text)
                                        .unwrap_or(Value::Null),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "processing",
                Value::Array(
                    self.processing
                        .iter()
                        .map(|part| {
                            map(vec![
                                ("code", text(part.code)),
                                ("text", text(&part.text)),
                                ("credits", scenarios(part.credits)),
                                ("exact", Value::Bool(part.exact)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("processingCredits", scenarios(self.processing_credits())),
            (
                "contractCharges",
                Value::Array(
                    self.contract_charges
                        .iter()
                        .map(ContractCharge::to_value)
                        .collect(),
                ),
            ),
            (
                "refund",
                map(vec![
                    (
                        "sameEpoch",
                        self.refund_same_epoch.map(scenarios).unwrap_or(Value::Null),
                    ),
                    (
                        "afterOneYear",
                        self.refund_after_one_year
                            .map(scenarios)
                            .unwrap_or(Value::Null),
                    ),
                ]),
            ),
            ("totalCredits", scenarios(self.total_credits())),
            (
                "fields",
                Value::Array(
                    self.fields
                        .iter()
                        .map(|field| {
                            let optional_number = |value: Option<u32>| {
                                value.map(|v| number(u64::from(v))).unwrap_or(Value::Null)
                            };
                            map(vec![
                                ("path", text(&field.path)),
                                ("kind", text(field.kind)),
                                ("optional", Value::Bool(field.optional)),
                                ("present", Value::Bool(field.present)),
                                ("length", optional_number(field.length)),
                                ("minLength", optional_number(field.min_length)),
                                ("maxLength", optional_number(field.max_length)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

#[cfg(all(test, feature = "server"))]
mod tests;
