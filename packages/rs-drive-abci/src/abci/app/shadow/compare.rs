//! What one engine made of a block, as a line, and the differences between two lines.

use dpp::dashcore::hashes::sha256;
use dpp::fee::fee_result::FeeResult;
use drive::fees::record::{FeeCalculation, RecordedOperation};
use drive::grovedb_costs::storage_cost::removal::StorageRemovedBytes;
use drive::grovedb_costs::OperationCost;
use prost::Message;
use serde::Serialize;
use std::borrow::Borrow;
use std::fmt::Write;
use tenderdash_abci::proto::abci as proto;

/// The sha256 of `bytes`, in hex.
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(sha256::Hash::hash(bytes).to_byte_array())
}

/// What one engine made of a block, as one line of JSON. The fields are written in the
/// order they stand in here, and a reader of the lines goes by their names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BlockLine {
    /// The height of the block.
    pub height: u64,
    /// The round it was committed in.
    pub round: u32,
    /// The core chain locked height of its header.
    pub core_height: u32,
    /// The application version of its header.
    pub protocol_version: u64,
    /// The app hash its execution gave, in hex.
    pub app_hash: String,
    /// GroveDB's root hash after the commit, in hex. `None` before the commit.
    pub grove_root: Option<String>,
    /// Whether GroveDB's root after the commit is the app hash. `None` before the commit.
    pub root_is_app_hash: Option<bool>,
    /// The code and `gas_used` of each transaction's result: the fee the result states.
    pub tx: Vec<(u32, i64)>,
    /// The sha256 of the whole answer to `ProcessProposal`.
    pub proposal_sha256: String,
    /// The sha256 of the answer to `FinalizeBlock`. `None` before the commit.
    pub finalize_sha256: Option<String>,
    /// The number of fee calculations of the block's execution. `None`, with the two
    /// fields after it, where the calculations of this execution are not known.
    pub fee_calcs: Option<usize>,
    /// The number of operations they were given, with the reads they charged themselves.
    pub fee_ops: Option<usize>,
    /// The sha256 of [`fees_text`] of them.
    pub fee_ops_sha256: Option<String>,
}

impl BlockLine {
    /// The line of a block that was executed and is not committed.
    pub(super) fn executed(
        request: &proto::RequestFinalizeBlock,
        answer: &proto::ResponseProcessProposal,
        fees: Option<&[FeeCalculation]>,
    ) -> Self {
        let header = request
            .block
            .as_ref()
            .and_then(|block| block.header.as_ref());
        Self {
            height: u64::try_from(request.height).unwrap_or_default(),
            round: u32::try_from(request.round).unwrap_or_default(),
            core_height: header
                .map(|header| header.core_chain_locked_height)
                .unwrap_or_default(),
            protocol_version: header
                .and_then(|header| header.version.as_ref())
                .map(|version| version.app)
                .unwrap_or_default(),
            app_hash: hex::encode(&answer.app_hash),
            grove_root: None,
            tx: answer
                .tx_results
                .iter()
                .map(|result| (result.code, result.gas_used))
                .collect(),
            proposal_sha256: sha256_hex(&answer.encode_to_vec()),
            finalize_sha256: None,
            root_is_app_hash: None,
            fee_calcs: fees.map(<[FeeCalculation]>::len),
            fee_ops: fees.map(|fees| {
                fees.iter()
                    .map(|calculation| {
                        calculation.operations.as_ref().map_or(0, Vec::len)
                            + calculation.reads.len()
                    })
                    .sum()
            }),
            fee_ops_sha256: fees.map(|fees| sha256_hex(fees_text(fees).as_bytes())),
        }
    }

    /// The line after the commit: with the answer to `FinalizeBlock` and GroveDB's root,
    /// where the root was read.
    pub(super) fn committed(
        mut self,
        answer: &proto::ResponseFinalizeBlock,
        grove_root: Option<&[u8]>,
    ) -> Self {
        let grove_root = grove_root.map(hex::encode);
        self.finalize_sha256 = Some(sha256_hex(&answer.encode_to_vec()));
        self.root_is_app_hash = grove_root.as_ref().map(|root| *root == self.app_hash);
        self.grove_root = grove_root;
        self
    }

    /// The line as JSON.
    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("a line of numbers and text is JSON")
    }
}

/// What a difference between two lines is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DivergenceKind {
    /// The state or an answer: an app hash, a transaction's code or fee, the rest of an
    /// answer, a root.
    State,
    /// The costs of the fee calculations alone.
    Cost,
    /// The stored records of the two databases.
    Store,
    /// The shadow returned an error or panicked.
    ShadowError,
}

impl DivergenceKind {
    /// The name the kind is counted under.
    pub fn name(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Cost => "cost",
            Self::Store => "store",
            Self::ShadowError => "shadow_error",
        }
    }
}

/// One field that two lines differ in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// The height of the block.
    pub height: u64,
    /// The name of the field.
    pub field: &'static str,
    /// The primary's value.
    pub this: String,
    /// The shadow's value.
    pub other: String,
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DIFFERS height={} field={} this={} other={}",
            self.height, self.field, self.this, self.other
        )
    }
}

impl Difference {
    /// What the difference is about.
    pub fn kind(&self) -> DivergenceKind {
        match self.field {
            "fee_calcs" | "fee_ops" | "fee_ops_sha256" => DivergenceKind::Cost,
            "stored_records" => DivergenceKind::Store,
            "shadow_error" => DivergenceKind::ShadowError,
            _ => DivergenceKind::State,
        }
    }
}

/// The fields `this`, the primary's line, and `other`, the shadow's, differ in. The
/// three of the fee calculations are not held against each other where one line has none.
pub(super) fn differences(this: &BlockLine, other: &BlockLine) -> Vec<Difference> {
    fn text<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).expect("a field of numbers and text is JSON")
    }
    let mut found = Vec::new();
    macro_rules! field {
        ($name:ident) => {
            if this.$name != other.$name {
                found.push(Difference {
                    height: this.height,
                    field: stringify!($name),
                    this: text(&this.$name),
                    other: text(&other.$name),
                });
            }
        };
    }
    field!(height);
    field!(round);
    field!(core_height);
    field!(protocol_version);
    field!(app_hash);
    field!(grove_root);
    field!(tx);
    field!(proposal_sha256);
    field!(finalize_sha256);
    field!(root_is_app_hash);
    // The fee calculations are compared where both executions' are known.
    if this.fee_ops_sha256.is_some() && other.fee_ops_sha256.is_some() {
        field!(fee_calcs);
        field!(fee_ops);
        field!(fee_ops_sha256);
    }
    found
}

/// What the two databases differ in after both committed a block: the answer to
/// `FinalizeBlock`, and, where both roots were read, GroveDB's root and whether the root
/// is the app hash. Each side is its answer, its root, and the app hash it committed, in
/// hex.
pub(super) fn after_the_commits(
    height: u64,
    this: (&proto::ResponseFinalizeBlock, Option<&[u8]>, &str),
    other: (&proto::ResponseFinalizeBlock, Option<&[u8]>, &str),
) -> Vec<Difference> {
    let quoted = |text: String| format!("\"{text}\"");
    let answer =
        |answer: &proto::ResponseFinalizeBlock| quoted(sha256_hex(&answer.encode_to_vec()));
    let mut fields = vec![("finalize_sha256", answer(this.0), answer(other.0))];
    if let (Some(this_root), Some(other_root)) = (this.1, other.1) {
        let (this_root, other_root) = (hex::encode(this_root), hex::encode(other_root));
        fields.push((
            "root_is_app_hash",
            (this_root == this.2).to_string(),
            (other_root == other.2).to_string(),
        ));
        fields.push(("grove_root", quoted(this_root), quoted(other_root)));
    }
    fields
        .into_iter()
        .filter(|(_, this, other)| this != other)
        .map(|(field, this, other)| Difference {
            height,
            field,
            this,
            other,
        })
        .collect()
}

/// The most severe kind among `differences`: the state before the costs.
pub(super) fn kind_of(differences: &[Difference]) -> Option<DivergenceKind> {
    differences.iter().map(Difference::kind).min()
}

fn sorted<K: Borrow<u16>, V: Borrow<W>, W: Copy + Into<u64>>(
    pairs: impl Iterator<Item = (K, V)>,
) -> String {
    let mut pairs: Vec<(u16, u64)> = pairs
        .map(|(key, value)| (*key.borrow(), (*value.borrow()).into()))
        .collect();
    pairs.sort_unstable();
    let pairs: Vec<String> = pairs
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect();
    pairs.join(",")
}

fn cost_text(cost: &OperationCost) -> String {
    let removed = match &cost.storage_cost.removed_bytes {
        StorageRemovedBytes::NoStorageRemoval => "none".to_string(),
        StorageRemovedBytes::BasicStorageRemoval(bytes) => format!("basic({bytes})"),
        StorageRemovedBytes::SectionedStorageRemoval(sections) => {
            let sections: Vec<String> = sections
                .iter()
                .map(|(identifier, epochs)| {
                    format!(
                        "{}:{{{}}}",
                        hex::encode(identifier),
                        sorted::<_, _, u32>(epochs.iter())
                    )
                })
                .collect();
            format!("sectioned({})", sections.join(";"))
        }
    };
    format!(
        "seeks={} added={} replaced={} removed={} loaded={} hashes={} sinsemilla={}",
        cost.seek_count,
        cost.storage_cost.added_bytes,
        cost.storage_cost.replaced_bytes,
        removed,
        cost.storage_loaded_bytes,
        cost.hash_node_calls,
        cost.sinsemilla_hash_calls,
    )
}

fn fee_text(fee: &FeeResult) -> String {
    let refunds: Vec<String> = fee
        .fee_refunds
        .0
        .iter()
        .map(|(identifier, epochs)| {
            format!(
                "{}:{{{}}}",
                hex::encode(identifier),
                sorted::<_, _, u64>(epochs.iter())
            )
        })
        .collect();
    format!(
        "storage={} processing={} removed_from_system={} refunds=({}) lifetime={:?}",
        fee.storage_fee,
        fee.processing_fee,
        fee.removed_bytes_from_system,
        refunds.join(";"),
        fee.lifetime_storage_fees,
    )
}

fn operation_text(operation: &RecordedOperation) -> String {
    match operation {
        RecordedOperation::Cost(cost) => format!("cost {}", cost_text(cost)),
        RecordedOperation::EphemeralCost(cost, pricing) => {
            format!("ephemeral cost {} priced {pricing:?}", cost_text(cost))
        }
        RecordedOperation::Fee(fee) => format!("fee {}", fee_text(fee)),
        RecordedOperation::Other(text) => format!("other {text:?}"),
    }
}

fn outcome_text(outcome: &Result<FeeResult, String>) -> String {
    match outcome {
        Ok(fee) => fee_text(fee),
        Err(error) => format!("error {error:?}"),
    }
}

fn calculation_text(calculation: &FeeCalculation) -> String {
    format!(
        "{} epoch={} base={:?} gives {}",
        match calculation.transaction {
            Some(place) => format!("transaction {place}"),
            None => "outside a transaction".to_string(),
        },
        calculation.epoch,
        calculation.base_operations,
        outcome_text(&calculation.outcome),
    )
}

/// The fee calculations of a block as text: a line for each calculation, one for each
/// operation it was given and one for each read it charged itself. Two lists give the
/// same text exactly where they are equal: a map is written in the order of its keys, and
/// a text that a calculation carries is written quoted.
pub fn fees_text(fees: &[FeeCalculation]) -> String {
    let mut text = String::new();
    for (number, calculation) in fees.iter().enumerate() {
        let _ = writeln!(
            text,
            "calculation {number} {}",
            calculation_text(calculation)
        );
        match &calculation.operations {
            None => text.push_str("  no operations given\n"),
            Some(operations) => {
                for (number, operation) in operations.iter().enumerate() {
                    let _ = writeln!(text, "  operation {number} {}", operation_text(operation));
                }
            }
        }
        for (number, read) in calculation.reads.iter().enumerate() {
            let _ = writeln!(text, "  read {number} {}", operation_text(read));
        }
    }
    text
}

/// The fee calculations of a block, taken from those of the proposal a node prepared
/// for it.
///
/// A node that prepares a proposal executes the transactions it is given and leaves some
/// out of the block. `kept` says of each transaction it executed, by its place, whether
/// the block has it. The calculations of a transaction that was left out are left out;
/// those of one that was kept carry its place in the block; those outside a transaction
/// stay. What is returned is what an execution of the block itself calculates, where the
/// two engines agree.
pub(super) fn of_the_block(prepared: &[FeeCalculation], kept: &[bool]) -> Vec<FeeCalculation> {
    // The place in the block of each transaction that was kept: how many were kept
    // before it.
    let mut kept_before = 0;
    let places: Vec<Option<usize>> = kept
        .iter()
        .map(|kept| {
            let place = kept.then_some(kept_before);
            kept_before += usize::from(*kept);
            place
        })
        .collect();
    let place_in_the_block = |place: usize| places.get(place).copied().flatten();
    prepared
        .iter()
        .filter_map(|calculation| match calculation.transaction {
            None => Some(calculation.clone()),
            Some(place) => place_in_the_block(place).map(|place| FeeCalculation {
                transaction: Some(place),
                ..calculation.clone()
            }),
        })
        .collect()
}

/// The first place two lists of fee calculations differ, as text with both sides. `None`
/// where they are equal.
pub(super) fn first_fee_difference(
    this: &[FeeCalculation],
    other: &[FeeCalculation],
) -> Option<String> {
    let number = (0..this.len().max(other.len())).find(|&n| this.get(n) != other.get(n))?;
    let (Some(this), Some(other)) = (this.get(number), other.get(number)) else {
        return Some(format!(
            "fee calculation {number} is made by one engine alone: this has {}, other has {}",
            this.len(),
            other.len()
        ));
    };
    // One that was given no operations is not one that was given none of them.
    if this.operations.is_some() != other.operations.is_some() {
        let given = |calculation: &FeeCalculation| match &calculation.operations {
            Some(operations) => format!("{} operations given", operations.len()),
            None => "no operations given".to_string(),
        };
        return Some(format!(
            "fee calculation {number}: this={} other={}",
            given(this),
            given(other)
        ));
    }
    // The operations a calculation was given, then the reads it charged itself.
    let all = |calculation: &FeeCalculation| -> Vec<(String, RecordedOperation)> {
        let given = calculation.operations.iter().flatten().enumerate();
        let reads = calculation.reads.iter().enumerate();
        given
            .map(|(n, operation)| (format!("operation {n}"), operation.clone()))
            .chain(reads.map(|(n, read)| (format!("read {n}"), read.clone())))
            .collect()
    };
    let (these, others) = (all(this), all(other));
    let place = (0..these.len().max(others.len())).find(|&n| these.get(n) != others.get(n));
    let side = |operations: &[(String, RecordedOperation)], n: usize| {
        operations
            .get(n)
            .map(|(_, operation)| operation_text(operation))
            .unwrap_or_else(|| "no such operation".to_string())
    };
    Some(match place {
        Some(n) => format!(
            "fee calculation {number} {}: this={} other={}",
            these
                .get(n)
                .or(others.get(n))
                .map(|(name, _)| name.as_str())
                .unwrap_or_default(),
            side(&these, n),
            side(&others, n)
        ),
        None => format!(
            "fee calculation {number}: this={} other={}",
            calculation_text(this),
            calculation_text(other)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::fee::fee_result::refunds::FeeRefunds;
    use drive::grovedb_costs::storage_cost::removal::StorageRemovalPerEpochByIdentifier;
    use drive::grovedb_costs::storage_cost::StorageCost;
    use tenderdash_abci::proto::types::{Block, Header};
    use tenderdash_abci::proto::version::Consensus;

    fn request(height: i64) -> proto::RequestFinalizeBlock {
        proto::RequestFinalizeBlock {
            height,
            round: 2,
            block: Some(Block {
                header: Some(Header {
                    version: Some(Consensus { block: 14, app: 9 }),
                    core_chain_locked_height: 77,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn answer() -> proto::ResponseProcessProposal {
        proto::ResponseProcessProposal {
            app_hash: vec![0xab; 32],
            tx_results: vec![
                proto::ExecTxResult {
                    code: 0,
                    gas_used: 1200,
                    ..Default::default()
                },
                proto::ExecTxResult {
                    code: 10603,
                    gas_used: 0,
                    info: "refused".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    fn cost(seek_count: u32) -> OperationCost {
        OperationCost {
            seek_count,
            storage_cost: StorageCost {
                added_bytes: 10,
                replaced_bytes: 20,
                removed_bytes: StorageRemovedBytes::BasicStorageRemoval(30),
            },
            storage_loaded_bytes: 40,
            hash_node_calls: 5,
            sinsemilla_hash_calls: 6,
        }
    }

    fn calculation(seek_count: u32) -> FeeCalculation {
        FeeCalculation {
            transaction: None,
            base_operations: None,
            operations: Some(vec![
                RecordedOperation::Cost(cost(seek_count)),
                RecordedOperation::Other("FunctionOperation".to_string()),
            ]),
            reads: vec![],
            epoch: 3,
            outcome: Ok(FeeResult {
                storage_fee: 7,
                processing_fee: u64::from(seek_count),
                ..Default::default()
            }),
        }
    }

    fn line() -> BlockLine {
        BlockLine::executed(
            &request(41),
            &answer(),
            Some(&[calculation(1), calculation(2)]),
        )
    }

    #[test]
    fn a_line_says_what_the_block_and_its_execution_held() {
        let line = line();
        assert_eq!(line.height, 41);
        assert_eq!(line.round, 2);
        assert_eq!(line.core_height, 77);
        assert_eq!(line.protocol_version, 9);
        assert_eq!(line.app_hash, "ab".repeat(32));
        assert_eq!(line.tx, vec![(0, 1200), (10603, 0)]);
        assert_eq!(line.proposal_sha256, sha256_hex(&answer().encode_to_vec()));
        assert_eq!((line.fee_calcs, line.fee_ops), (Some(2), Some(4)));
        assert_eq!(
            line.fee_ops_sha256,
            Some(sha256_hex(
                fees_text(&[calculation(1), calculation(2)]).as_bytes()
            ))
        );
        // A read a calculation charged itself is one of its operations.
        let mut with_a_read = calculation(1);
        with_a_read.reads = vec![RecordedOperation::Cost(cost(9))];
        assert_eq!(
            BlockLine::executed(&request(41), &answer(), Some(&[with_a_read])).fee_ops,
            Some(3)
        );
        // An execution whose calculations are not known says so.
        let unknown = BlockLine::executed(&request(41), &answer(), None);
        assert_eq!(
            (unknown.fee_calcs, unknown.fee_ops, &unknown.fee_ops_sha256),
            (None, None, &None)
        );
        assert!(unknown.json().contains("\"fee_calcs\":null"));
        assert_eq!(
            (
                &line.grove_root,
                &line.finalize_sha256,
                line.root_is_app_hash
            ),
            (&None, &None, None)
        );

        let finalized = proto::ResponseFinalizeBlock {
            retain_height: 3,
            ..Default::default()
        };
        let committed = line.clone().committed(&finalized, Some(&[0xab; 32]));
        assert_eq!(committed.grove_root, Some("ab".repeat(32)));
        assert_eq!(committed.root_is_app_hash, Some(true));
        assert_eq!(
            committed.finalize_sha256,
            Some(sha256_hex(&finalized.encode_to_vec()))
        );
        assert_eq!(
            line.committed(&finalized, Some(&[1; 32])).root_is_app_hash,
            Some(false)
        );
    }

    /// The names and the order of the fields are what a reader of the lines goes by.
    #[test]
    fn a_line_is_json_with_the_fields_in_their_order() {
        let json = line().json();
        let fields: Vec<&str> = [
            "height",
            "round",
            "core_height",
            "protocol_version",
            "app_hash",
            "grove_root",
            "root_is_app_hash",
            "tx",
            "proposal_sha256",
            "finalize_sha256",
            "fee_calcs",
            "fee_ops",
            "fee_ops_sha256",
        ]
        .to_vec();
        let places: Vec<usize> = fields
            .iter()
            .map(|field| json.find(&format!("\"{field}\":")).expect(field))
            .collect();
        assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{json}");
        assert!(
            json.starts_with("{\"height\":41,\"round\":2,\"core_height\":77,"),
            "{json}"
        );
        assert!(json.contains("\"tx\":[[0,1200],[10603,0]]"), "{json}");
        assert!(!json.contains('\n'));
    }

    #[test]
    fn two_equal_lines_differ_in_nothing() {
        assert_eq!(differences(&line(), &line()), vec![]);
        assert_eq!(kind_of(&[]), None);
    }

    /// Every field that is compared, changed alone, is the one field that differs, with
    /// both values, and is of the kind it belongs to.
    #[test]
    fn each_field_that_differs_is_named_with_both_values_and_its_kind() {
        let finalized = proto::ResponseFinalizeBlock {
            retain_height: 3,
            ..Default::default()
        };
        let this = line().committed(&finalized, Some(&[0xab; 32]));
        type Change = fn(&mut BlockLine);
        let changes: [(&str, Change, DivergenceKind); 13] = [
            ("height", |line| line.height += 1, DivergenceKind::State),
            ("round", |line| line.round += 1, DivergenceKind::State),
            (
                "core_height",
                |line| line.core_height += 1,
                DivergenceKind::State,
            ),
            (
                "protocol_version",
                |line| line.protocol_version += 1,
                DivergenceKind::State,
            ),
            (
                "app_hash",
                |line| line.app_hash.push('0'),
                DivergenceKind::State,
            ),
            (
                "grove_root",
                |line| line.grove_root = None,
                DivergenceKind::State,
            ),
            ("tx", |line| line.tx[0].1 += 1, DivergenceKind::State),
            (
                "proposal_sha256",
                |line| line.proposal_sha256.push('0'),
                DivergenceKind::State,
            ),
            (
                "finalize_sha256",
                |line| line.finalize_sha256 = None,
                DivergenceKind::State,
            ),
            (
                "root_is_app_hash",
                |line| line.root_is_app_hash = Some(false),
                DivergenceKind::State,
            ),
            (
                "fee_calcs",
                |line| line.fee_calcs = Some(9),
                DivergenceKind::Cost,
            ),
            (
                "fee_ops",
                |line| line.fee_ops = Some(9),
                DivergenceKind::Cost,
            ),
            (
                "fee_ops_sha256",
                |line| line.fee_ops_sha256 = Some("00".to_string()),
                DivergenceKind::Cost,
            ),
        ];
        for (field, change, kind) in changes {
            let mut other = this.clone();
            change(&mut other);
            let found = differences(&this, &other);
            assert_eq!(found.len(), 1, "{field}");
            assert_eq!(found[0].field, field);
            assert_eq!(found[0].kind(), kind, "{field}");
            assert_eq!(kind_of(&found), Some(kind));
            assert_ne!(found[0].this, found[0].other, "{field}");
        }
        let mut other = this.clone();
        other.tx[0].1 = 1201;
        assert_eq!(
            differences(&this, &other)[0].to_string(),
            "DIFFERS height=41 field=tx this=[[0,1200],[10603,0]] other=[[0,1201],[10603,0]]"
        );
    }

    /// Where the fee calculations of one execution are not known, the lines are not held
    /// against each other in them, and are in everything else.
    #[test]
    fn fee_calculations_that_are_not_known_are_not_a_difference() {
        let this = line();
        let mut other = BlockLine::executed(&request(41), &answer(), None);
        assert_eq!(differences(&this, &other), vec![]);
        assert_eq!(differences(&other, &this), vec![]);
        other.app_hash.push('0');
        assert_eq!(differences(&this, &other).len(), 1);
    }

    #[test]
    fn a_difference_in_state_goes_before_one_in_costs() {
        let this = line();
        let mut other = this.clone();
        other.fee_calcs = Some(9);
        other.app_hash.push('0');
        assert_eq!(
            kind_of(&differences(&this, &other)),
            Some(DivergenceKind::State)
        );
        for (field, kind) in [
            ("stored_records", DivergenceKind::Store),
            ("shadow_error", DivergenceKind::ShadowError),
            ("init_chain_sha256", DivergenceKind::State),
        ] {
            let difference = Difference {
                height: 1,
                field,
                this: String::new(),
                other: String::new(),
            };
            assert_eq!(difference.kind(), kind);
        }
        assert_eq!(
            [
                DivergenceKind::State,
                DivergenceKind::Cost,
                DivergenceKind::Store,
                DivergenceKind::ShadowError
            ]
            .map(DivergenceKind::name),
            ["state", "cost", "store", "shadow_error"]
        );
    }

    /// Two costs and two fees that hold the same maps, filled in another order, give one
    /// text: a map is written in the order of its keys.
    #[test]
    fn the_text_of_fees_does_not_depend_on_the_order_a_map_was_filled_in() {
        let sectioned = |epochs: [u16; 3]| {
            let mut sections = StorageRemovalPerEpochByIdentifier::default();
            for identifier in [[2u8; 32], [1u8; 32]] {
                let per_epoch = sections.entry(identifier).or_default();
                for epoch in epochs {
                    per_epoch.insert(epoch, u32::from(epoch) * 10);
                }
            }
            let mut cost = cost(1);
            cost.storage_cost.removed_bytes =
                StorageRemovedBytes::SectionedStorageRemoval(sections);
            cost
        };
        let refunds = |epochs: [u16; 3]| {
            let mut fee = FeeResult::default();
            let mut refunds = FeeRefunds::default();
            let per_epoch = refunds.0.entry([9u8; 32]).or_default();
            for epoch in epochs {
                per_epoch.insert(epoch, u64::from(epoch) * 100);
            }
            fee.fee_refunds = refunds;
            fee.lifetime_storage_fees.insert(4, 44);
            fee
        };
        let fees = |epochs: [u16; 3]| {
            vec![FeeCalculation {
                transaction: Some(2),
                reads: vec![RecordedOperation::Cost(cost(3))],
                base_operations: Some("{HashNode: 1}".to_string()),
                operations: Some(vec![
                    RecordedOperation::Cost(sectioned(epochs)),
                    RecordedOperation::EphemeralCost(cost(2), "TimeRangeTtl".to_string()),
                    RecordedOperation::Fee(refunds(epochs)),
                ]),
                epoch: 5,
                outcome: Ok(refunds(epochs)),
            }]
        };
        let text = fees_text(&fees([1, 2, 300]));
        assert_eq!(text, fees_text(&fees([300, 2, 1])));
        assert_eq!(
            text,
            format!(
                "calculation 0 transaction 2 epoch=5 base=Some(\"{{HashNode: 1}}\") gives storage=0 processing=0 \
                 removed_from_system=0 refunds=({nine}:{{1:100,2:200,300:30000}}) \
                 lifetime={{4: 44}}\n  operation 0 cost seeks=1 added=10 replaced=20 \
                 removed=sectioned({one}:{{1:10,2:20,300:3000}};{two}:{{1:10,2:20,300:3000}}) \
                 loaded=40 hashes=5 sinsemilla=6\n  operation 1 ephemeral cost seeks=2 added=10 \
                 replaced=20 removed=basic(30) loaded=40 hashes=5 sinsemilla=6 priced \
                 \"TimeRangeTtl\"\n  operation 2 fee storage=0 processing=0 removed_from_system=0 \
                 refunds=({nine}:{{1:100,2:200,300:30000}}) lifetime={{4: 44}}\n  read 0 cost \
                 seeks=3 added=10 replaced=20 removed=basic(30) loaded=40 hashes=5 sinsemilla=6\n",
                nine = "09".repeat(32),
                one = "01".repeat(32),
                two = "02".repeat(32),
            )
        );
    }

    #[test]
    fn the_text_of_fees_says_an_error_and_a_calculation_given_no_list() {
        let fees = [FeeCalculation {
            transaction: None,
            base_operations: None,
            operations: None,
            reads: vec![],
            epoch: 0,
            outcome: Err("overflow".to_string()),
        }];
        assert_eq!(
            fees_text(&fees),
            "calculation 0 outside a transaction epoch=0 base=None gives error \"overflow\"\n  no \
             operations given\n"
        );
        let mut removed_nothing = cost(1);
        removed_nothing.storage_cost.removed_bytes = StorageRemovedBytes::NoStorageRemoval;
        assert!(cost_text(&removed_nothing).contains("removed=none"));
    }

    #[test]
    fn the_first_fee_difference_names_the_calculation_and_the_operation() {
        let these = [calculation(1), calculation(2), calculation(3)];
        assert_eq!(first_fee_difference(&these, &these), None);

        let mut others = these.clone();
        let Some(RecordedOperation::Cost(cost)) = others[1]
            .operations
            .as_mut()
            .and_then(|operations| operations.first_mut())
        else {
            panic!("a cost");
        };
        cost.seek_count = 9;
        let said = first_fee_difference(&these, &others).expect("a difference");
        assert!(
            said.starts_with("fee calculation 1 operation 0: this=cost seeks=2 added=10 ")
                && said.contains(" other=cost seeks=9 added=10 "),
            "{said}"
        );

        // The operations are equal and what the calculation gave is not.
        let mut others = these.clone();
        others[2].outcome = Err("refused".to_string());
        let said = first_fee_difference(&these, &others).expect("a difference");
        assert!(
            said.starts_with(
                "fee calculation 2: this=outside a transaction epoch=3 base=None gives storage=7 \
                 processing=3 "
            ) && said
                .ends_with("other=outside a transaction epoch=3 base=None gives error \"refused\""),
            "{said}"
        );

        // One list has an operation the other lacks.
        let mut others = these.clone();
        others[0].operations.as_mut().expect("a list").pop();
        let said = first_fee_difference(&these, &others).expect("a difference");
        assert_eq!(
            said,
            "fee calculation 0 operation 1: this=other \"FunctionOperation\" other=no such operation"
        );

        // One list is longer.
        assert_eq!(
            first_fee_difference(&these, &these[..2]),
            Some(
                "fee calculation 2 is made by one engine alone: this has 3, other has 2"
                    .to_string()
            )
        );
    }

    /// A read that a calculation charged itself and a transaction it belongs to are part
    /// of what is compared: a difference in either is found and named.
    #[test]
    fn a_read_and_a_transaction_that_differ_are_named() {
        let mut these = [calculation(1), calculation(2)];
        these[1].reads = vec![RecordedOperation::Cost(cost(5))];
        let mut others = these.clone();
        others[1].reads = vec![RecordedOperation::Cost(cost(6))];
        let said = first_fee_difference(&these, &others).expect("a difference");
        assert!(
            said.starts_with("fee calculation 1 read 0: this=cost seeks=5 ")
                && said.contains(" other=cost seeks=6 "),
            "{said}"
        );
        assert_ne!(fees_text(&these), fees_text(&others));

        let mut others = these.clone();
        others[0].transaction = Some(4);
        let said = first_fee_difference(&these, &others).expect("a difference");
        assert!(
            said.starts_with("fee calculation 0: this=outside a transaction epoch=3 ")
                && said.contains(" other=transaction 4 epoch=3 "),
            "{said}"
        );
        assert_ne!(fees_text(&these), fees_text(&others));
    }

    /// From the calculations of a prepared proposal, those of the transactions that were
    /// left out go, those of the ones that were kept carry their place in the block, and
    /// those outside a transaction stay.
    #[test]
    fn the_calculations_of_a_block_are_those_of_the_transactions_it_kept() {
        let of = |transaction: Option<usize>, seek_count: u32| FeeCalculation {
            transaction,
            ..calculation(seek_count)
        };
        let prepared = [
            of(None, 1),
            of(Some(0), 2),
            of(Some(1), 3),
            of(Some(1), 4),
            of(Some(2), 5),
            of(Some(3), 6),
            of(None, 7),
        ];
        // The second and the fourth transaction were left out.
        assert_eq!(
            of_the_block(&prepared, &[true, false, true, false]),
            vec![of(None, 1), of(Some(0), 2), of(Some(1), 5), of(None, 7)]
        );
        assert_eq!(
            of_the_block(&prepared, &[true, true, true, true]),
            prepared.to_vec()
        );
        assert_eq!(
            of_the_block(&prepared, &[false, false, false, false]),
            vec![of(None, 1), of(None, 7)]
        );
        // A transaction the list does not speak of was not kept.
        assert_eq!(
            of_the_block(&prepared, &[false, true]),
            vec![of(None, 1), of(Some(0), 3), of(Some(0), 4), of(None, 7)]
        );
        // What a block's own execution calculates otherwise is not what was prepared: a
        // cost that moved from a kept transaction to one that was left out is a difference.
        let block = [of(None, 1), of(Some(0), 3), of(Some(1), 5), of(None, 7)];
        assert_ne!(
            of_the_block(&prepared, &[true, false, true, false]),
            block.to_vec()
        );
        assert_ne!(
            of_the_block(&prepared, &[true, false, true, false]),
            Vec::new()
        );
    }
}
