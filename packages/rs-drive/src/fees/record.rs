//! A record of the fee calculations a thread makes, for a caller that holds the costs of
//! one execution against another's.
//!
//! Nothing is recorded on a thread until [`record_fee_calculations`] is called on it, and
//! then only for the length of the closure it is given and only on that thread. A fee
//! calculation on a thread that records nothing reads one thread-local where it begins,
//! and once more where it charges itself a read, and does no more for the record. (A
//! build for tests also counts it, see [`every_thread`].)

use crate::error::Error;
use crate::fees::op::{BaseOp, LowLevelDriveOperation};
use dpp::block::epoch::EpochIndex;
use dpp::fee::fee_result::FeeResult;
use enum_map::EnumMap;
use grovedb_costs::OperationCost;
use std::cell::RefCell;

/// An operation as a fee calculation was given it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordedOperation {
    /// What GroveDB charged for a call or for the calls of a batch.
    Cost(OperationCost),
    /// The same for a batch whose added bytes are priced by a rule of their own, with the
    /// rule as its `Debug` text.
    EphemeralCost(OperationCost, String),
    /// A fee that was calculated before.
    Fee(FeeResult),
    /// Any other operation, as its `Debug` text.
    Other(String),
}

impl RecordedOperation {
    fn of(operation: &LowLevelDriveOperation) -> Self {
        match operation {
            LowLevelDriveOperation::CalculatedCostOperation(cost) => Self::Cost(cost.clone()),
            LowLevelDriveOperation::CalculatedEphemeralCostOperation(cost, pricing) => {
                Self::EphemeralCost(cost.clone(), format!("{pricing:?}"))
            }
            LowLevelDriveOperation::PreCalculatedFeeResult(fee) => Self::Fee(fee.clone()),
            other => Self::Other(format!("{other:?}")),
        }
    }
}

/// One call of [`Drive::calculate_fee`](crate::drive::Drive::calculate_fee): what it was
/// given and what it returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeCalculation {
    /// The transaction the thread said it was executing ([`transaction`]), by its place
    /// in the list it executes. `None` for a calculation outside any.
    pub transaction: Option<usize>,
    /// The base operations, as their `Debug` text.
    pub base_operations: Option<String>,
    /// The operations, in the order they were given. `None` where the call was given no
    /// list.
    pub operations: Option<Vec<RecordedOperation>>,
    /// The operations the calculation charged of its own accord, for what it read to
    /// price the others: the reads of the records of earlier epochs.
    pub reads: Vec<RecordedOperation>,
    /// The epoch the fee was calculated for.
    pub epoch: EpochIndex,
    /// The fee, or the text of the error.
    pub outcome: Result<FeeResult, String>,
}

/// What a thread that records has recorded, and what it is at.
#[derive(Default)]
struct Recording {
    calculations: Vec<FeeCalculation>,
    /// The transaction the thread is executing.
    transaction: Option<usize>,
    /// The reads of the calculation that is being made.
    reads: Vec<RecordedOperation>,
}

thread_local! {
    static RECORD: RefCell<Option<Recording>> = const { RefCell::new(None) };
}

/// Puts back what the thread recorded into before, also where the closure panics.
struct Restore(Option<Recording>);

impl Drop for Restore {
    fn drop(&mut self) {
        let before = self.0.take();
        RECORD.with(|record| *record.borrow_mut() = before);
    }
}

/// Runs `run` and returns, with what it returns, every fee calculation it made on this
/// thread, in order. A calculation that `run` makes on another thread is not in the list.
/// Inside `run` a second call records for itself, and the calculations of its closure are
/// not in the list of the first.
pub fn record_fee_calculations<R>(run: impl FnOnce() -> R) -> (R, Vec<FeeCalculation>) {
    let before = RECORD.with(|record| record.borrow_mut().replace(Recording::default()));
    let restore = Restore(before);
    let returned = run();
    let recorded = RECORD.with(|record| record.borrow_mut().take());
    drop(restore);
    (
        returned,
        recorded
            .map(|recording| recording.calculations)
            .unwrap_or_default(),
    )
}

/// Says which transaction this thread executes from now on, by its place in the list it
/// executes, or that it executes none. The calculations it records carry the place. It
/// does nothing on a thread that records nothing.
pub fn transaction(place: Option<usize>) {
    RECORD.with(|record| {
        if let Some(recording) = record.borrow_mut().as_mut() {
            recording.transaction = place;
        }
    });
}

/// What a fee calculation on this thread was given, where the thread records.
pub(super) struct Given {
    base_operations: Option<String>,
    operations: Option<Vec<RecordedOperation>>,
}

/// `None` where this thread records nothing.
pub(super) fn given(
    base_operations: Option<&EnumMap<BaseOp, u64>>,
    operations: Option<&[LowLevelDriveOperation]>,
) -> Option<Given> {
    RECORD.with(|record| {
        let mut record = record.borrow_mut();
        let recording = record.as_mut()?;
        recording.reads.clear();
        Some(Given {
            base_operations: base_operations.map(|base| format!("{base:?}")),
            operations: operations
                .map(|operations| operations.iter().map(RecordedOperation::of).collect()),
        })
    })
}

/// Records the operations a fee calculation charges of its own accord, where this thread
/// records.
pub(super) fn reads(operations: &[LowLevelDriveOperation]) {
    RECORD.with(|record| {
        if let Some(recording) = record.borrow_mut().as_mut() {
            recording
                .reads
                .extend(operations.iter().map(RecordedOperation::of));
        }
    });
}

pub(super) fn returned(given: Given, epoch: EpochIndex, outcome: &Result<FeeResult, Error>) {
    RECORD.with(|record| {
        if let Some(recording) = record.borrow_mut().as_mut() {
            let calculation = FeeCalculation {
                transaction: recording.transaction,
                base_operations: given.base_operations,
                operations: given.operations,
                reads: std::mem::take(&mut recording.reads),
                epoch,
                outcome: match outcome {
                    Ok(fee) => Ok(fee.clone()),
                    Err(error) => Err(error.to_string()),
                },
            };
            recording.calculations.push(calculation);
        }
    });
}

/// The count of the fee calculations a Drive makes on every thread, for a test that holds
/// what one thread recorded against it.
///
/// The count is of the Drive, whoever calculates: a test that lets another thread
/// calculate fees on the same Drive beside what it counts, as `CheckTx` does beside a
/// block, counts those too.
#[cfg(any(test, feature = "fixtures-and-mocks"))]
pub mod every_thread {
    use crate::drive::Drive;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// A count that is open: the address of the Drive it counts, and what it counted.
    struct Open {
        drive: usize,
        counted: u64,
    }

    /// The counts that are open, by a number of their own, and the last number given.
    struct Counts {
        last: u64,
        open: BTreeMap<u64, Open>,
    }

    static COUNTS: Mutex<Counts> = Mutex::new(Counts {
        last: 0,
        open: BTreeMap::new(),
    });

    fn counts() -> std::sync::MutexGuard<'static, Counts> {
        COUNTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn address(drive: &Drive) -> usize {
        std::ptr::from_ref(drive) as usize
    }

    /// One open count. It is closed when it is dropped, also where what it counts panics.
    struct Count(u64);

    impl Drop for Count {
        fn drop(&mut self) {
            counts().open.remove(&self.0);
        }
    }

    /// Runs `run` and returns, with what it returns, the number of fee calculations that
    /// `drive` made while it ran, on whatever thread. Counts of one Drive that are open at
    /// once each count every calculation made while they are open.
    pub fn count_fee_calculations<R>(drive: &Drive, run: impl FnOnce() -> R) -> (R, u64) {
        let count = {
            let mut counts = counts();
            counts.last += 1;
            let number = counts.last;
            counts.open.insert(
                number,
                Open {
                    drive: address(drive),
                    counted: 0,
                },
            );
            Count(number)
        };
        let returned = run();
        let counted = counts()
            .open
            .get(&count.0)
            .map(|open| open.counted)
            .unwrap_or_default();
        (returned, counted)
    }

    pub(in crate::fees) fn calculated(drive: &Drive) {
        let address = address(drive);
        for open in counts().open.values_mut() {
            if open.drive == address {
                open.counted += 1;
            }
        }
    }

    /// The number of counts of `drive` that are open.
    #[cfg(test)]
    pub(in crate::fees) fn open_counts(drive: &Drive) -> usize {
        let address = address(drive);
        counts()
            .open
            .values()
            .filter(|open| open.drive == address)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::op::LowLevelDriveOperation::{
        CalculatedCostOperation, PreCalculatedFeeResult,
    };
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    fn a_cost(seek_count: u32) -> OperationCost {
        OperationCost {
            seek_count,
            storage_loaded_bytes: 7,
            hash_node_calls: 2,
            ..Default::default()
        }
    }

    fn a_fee(drive: &crate::drive::Drive, seek_count: u32) -> Result<FeeResult, Error> {
        drive.calculate_fee(
            None,
            Some(vec![
                CalculatedCostOperation(a_cost(seek_count)),
                PreCalculatedFeeResult(FeeResult {
                    processing_fee: 5,
                    ..Default::default()
                }),
            ]),
            &Epoch::new(3).expect("an epoch"),
            PlatformVersion::latest(),
        )
    }

    #[test]
    fn a_calculation_is_recorded_with_what_it_was_given_and_what_it_returned() {
        let drive = setup_drive(None);
        let (fee, recorded) = record_fee_calculations(|| a_fee(&drive, 4));
        let fee = fee.expect("a fee");
        assert_eq!(
            recorded,
            vec![FeeCalculation {
                transaction: None,
                base_operations: None,
                reads: vec![],
                operations: Some(vec![
                    RecordedOperation::Cost(a_cost(4)),
                    RecordedOperation::Fee(FeeResult {
                        processing_fee: 5,
                        ..Default::default()
                    }),
                ]),
                epoch: 3,
                outcome: Ok(fee),
            }]
        );
    }

    #[test]
    fn nothing_is_recorded_outside_the_closure_or_on_another_thread() {
        let drive = setup_drive(None);
        a_fee(&drive, 1).expect("a fee");
        let ((), recorded) = record_fee_calculations(|| {
            std::thread::scope(|scope| {
                scope
                    .spawn(|| a_fee(&drive, 2).expect("a fee"))
                    .join()
                    .expect("the other thread");
            });
        });
        assert!(recorded.is_empty());
        a_fee(&drive, 3).expect("a fee");
        let (_, after) = record_fee_calculations(|| a_fee(&drive, 4));
        assert_eq!(after.len(), 1);
    }

    #[test]
    fn a_closure_inside_records_for_itself() {
        let drive = setup_drive(None);
        let (inside, outside) = record_fee_calculations(|| {
            a_fee(&drive, 1).expect("a fee");
            let (_, inside) = record_fee_calculations(|| a_fee(&drive, 2));
            a_fee(&drive, 3).expect("a fee");
            inside
        });
        let seeks = |recorded: &[FeeCalculation]| -> Vec<u32> {
            recorded
                .iter()
                .map(
                    |calculation| match &calculation.operations.as_ref().expect("a list")[0] {
                        RecordedOperation::Cost(cost) => cost.seek_count,
                        other => panic!("not a cost: {other:?}"),
                    },
                )
                .collect()
        };
        assert_eq!(seeks(&inside), vec![2]);
        assert_eq!(seeks(&outside), vec![1, 3]);
    }

    #[test]
    fn a_closure_that_panics_leaves_the_thread_as_it_was() {
        let drive = setup_drive(None);
        let ((), outside) = record_fee_calculations(|| {
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                record_fee_calculations(|| panic!("inside"))
            }));
            assert!(caught.is_err());
            a_fee(&drive, 9).expect("a fee");
        });
        assert_eq!(outside.len(), 1);
        a_fee(&drive, 9).expect("a fee");
        assert!(RECORD.with(|record| record.borrow().is_none()));
    }

    #[test]
    fn an_error_is_recorded_as_its_text() {
        let drive = setup_drive(None);
        let overflowing = || {
            drive.calculate_fee(
                None,
                Some(vec![
                    PreCalculatedFeeResult(FeeResult {
                        processing_fee: u64::MAX,
                        ..Default::default()
                    }),
                    PreCalculatedFeeResult(FeeResult {
                        processing_fee: 1,
                        ..Default::default()
                    }),
                ]),
                &Epoch::new(0).expect("an epoch"),
                PlatformVersion::latest(),
            )
        };
        let (fee, recorded) = record_fee_calculations(overflowing);
        let error = fee.expect_err("an overflow");
        assert_eq!(recorded[0].outcome, Err(error.to_string()));
    }

    #[test]
    fn a_calculation_carries_the_transaction_the_thread_says_it_executes() {
        let drive = setup_drive(None);
        // Outside a recording the saying is nothing.
        transaction(Some(9));
        let ((), recorded) = record_fee_calculations(|| {
            a_fee(&drive, 1).expect("a fee");
            transaction(Some(0));
            a_fee(&drive, 2).expect("a fee");
            a_fee(&drive, 3).expect("a fee");
            transaction(Some(1));
            a_fee(&drive, 4).expect("a fee");
            transaction(None);
            a_fee(&drive, 5).expect("a fee");
        });
        let places: Vec<Option<usize>> = recorded
            .iter()
            .map(|calculation| calculation.transaction)
            .collect();
        assert_eq!(places, vec![None, Some(0), Some(0), Some(1), None]);
        // A recording begins outside any transaction, whatever the one before it was at.
        let ((), first) = record_fee_calculations(|| {
            transaction(Some(4));
            a_fee(&drive, 1).expect("a fee");
        });
        let (_, second) = record_fee_calculations(|| a_fee(&drive, 1));
        assert_eq!(
            (first[0].transaction, second[0].transaction),
            (Some(4), None)
        );
    }

    #[test]
    fn the_operations_other_than_costs_are_recorded_as_what_they_are() {
        use crate::fees::op::EphemeralPricing;
        use crate::fees::op::LowLevelDriveOperation::{
            CalculatedEphemeralCostOperation, RepaidIdentityDebt,
        };
        let operations = [
            CalculatedEphemeralCostOperation(a_cost(1), EphemeralPricing::TimeRangeTtl),
            RepaidIdentityDebt(7),
        ];
        assert_eq!(
            operations
                .iter()
                .map(RecordedOperation::of)
                .collect::<Vec<_>>(),
            vec![
                RecordedOperation::EphemeralCost(a_cost(1), "TimeRangeTtl".to_string()),
                RecordedOperation::Other("RepaidIdentityDebt(7)".to_string()),
            ]
        );
    }

    #[test]
    fn the_reads_a_calculation_charges_itself_are_recorded_with_it() {
        let drive = setup_drive(None);
        let ((), recorded) = record_fee_calculations(|| {
            // As the calculation does between what it is given and what it returns.
            let first = given(None, Some(&[])).expect("recording");
            reads(&[CalculatedCostOperation(a_cost(8))]);
            returned(first, 0, &Ok(FeeResult::default()));
            a_fee(&drive, 1).expect("a fee");
        });
        assert_eq!(recorded[0].reads, vec![RecordedOperation::Cost(a_cost(8))]);
        assert!(recorded[1].reads.is_empty());
        // On a thread that records nothing they go nowhere.
        reads(&[CalculatedCostOperation(a_cost(8))]);
        assert!(given(None, None).is_none());
    }

    #[test]
    fn counts_of_one_drive_that_are_open_at_once_each_count_and_each_close() {
        use super::every_thread::{count_fee_calculations, open_counts};
        let drive = setup_drive(None);
        let other = setup_drive(None);
        let ((inner, on_another_thread), outer) = count_fee_calculations(&drive, || {
            a_fee(&drive, 1).expect("a fee");
            a_fee(&other, 1).expect("a fee");
            let (on_another_thread, inner) = count_fee_calculations(&drive, || {
                a_fee(&drive, 2).expect("a fee");
                std::thread::scope(|scope| {
                    scope
                        .spawn(|| count_fee_calculations(&drive, || a_fee(&drive, 3)).1)
                        .join()
                        .expect("the other thread")
                })
            });
            a_fee(&drive, 4).expect("a fee");
            (inner, on_another_thread)
        });
        assert_eq!((outer, inner, on_another_thread), (4, 2, 1));
        assert_eq!(open_counts(&drive), 0);

        // A count whose closure panics is closed too.
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            count_fee_calculations(&drive, || panic!("inside"))
        }));
        assert!(caught.is_err());
        assert_eq!(open_counts(&drive), 0);
        assert_eq!(count_fee_calculations(&drive, || a_fee(&drive, 5)).1, 1);
    }
}
