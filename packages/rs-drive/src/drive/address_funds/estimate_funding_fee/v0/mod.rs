use crate::drive::asset_lock::asset_lock_storage_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use crate::util::batch::drive_op_batch::finalize_task::DriveOperationFinalizationTasks;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::proof_depth::{single_key_proof_levels, SingleKeyProofLevels};
use dpp::address_funds::PlatformAddress;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::{EstimatedLayerInformation, PathQuery};
use std::collections::{BTreeMap, HashMap};

/// The outcome of a state-aware address funding fee estimation.
///
/// `fee_result` prices the GroveDB batch only (the three drive operations of
/// a 0-input / 1-output funding); validation-operation fees and
/// `user_fee_increase` are added by the caller.
#[derive(Debug, Clone)]
pub struct AddressFundingFeeEstimate {
    /// The estimated fee for the GroveDB batch.
    pub fee_result: FeeResult,
    /// Whether the recipient address already exists (the balance write is a
    /// replace) or is new (the write is an insert).
    pub address_exists: bool,
    /// Measured search-path levels for the recipient address in the clear
    /// address pool.
    pub address_layer_levels: u8,
    /// Measured search-path levels for the asset lock outpoint in the spent
    /// asset lock transactions tree.
    pub spent_asset_lock_layer_levels: u8,
}

impl Drive {
    /// Version 0 of the state-aware address funding fee estimation.
    ///
    /// See [`Drive::estimate_address_funding_fee`] for the contract.
    pub(super) fn estimate_address_funding_fee_v0(
        &self,
        recipient: &PlatformAddress,
        asset_lock_outpoint: Bytes36,
        lock_credits: Credits,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<AddressFundingFeeEstimate, Error> {
        // The estimation reads committed state several times (outpoint
        // fetch, two proofs, the stateful conversion) with no transaction. A
        // block committing in between could make those reads describe
        // different roots — the API promises pricing from one coherent
        // committed state, so accept an attempt only when the root hash is
        // byte-identical before and after all the reads, retrying otherwise.
        let (low_level_operations, layer_map, address_levels, outpoint_levels) =
            stable_committed_read(
                || {
                    self.grove
                        .root_hash(None, &platform_version.drive.grove_version)
                        .unwrap()
                        .map_err(Error::from)
                },
                || {
                    self.estimate_address_funding_fee_parts_v0(
                        recipient,
                        asset_lock_outpoint,
                        lock_credits,
                        block_info,
                        platform_version,
                    )
                },
            )?;

        let (grove_batch, mut cost_operations) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_with_leftovers(
                low_level_operations,
            );
        self.grove_batch_operations_costs(
            grove_batch,
            layer_map,
            false,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        let fee_result = Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok(AddressFundingFeeEstimate {
            fee_result,
            address_exists: address_levels.present,
            address_layer_levels: address_levels.levels,
            spent_asset_lock_layer_levels: outpoint_levels.levels,
        })
    }

    /// Builds the production operations (stateful) and the layer-info map
    /// (server models with measured counts) that
    /// [`Drive::estimate_address_funding_fee_v0`] prices.
    ///
    /// Split out so tests can inspect the exact operations and layer map.
    #[allow(clippy::type_complexity)]
    fn estimate_address_funding_fee_parts_v0(
        &self,
        recipient: &PlatformAddress,
        asset_lock_outpoint: Bytes36,
        lock_credits: Credits,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            Vec<LowLevelDriveOperation>,
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
            SingleKeyProofLevels,
            SingleKeyProofLevels,
        ),
        Error,
    > {
        // v0 scope: the estimate models a FRESH asset lock consumed in full
        // (0 address inputs / 1 remainder output). An outpoint already in the
        // state would execute through the partial-use path, which this
        // estimator does not model — fail closed.
        match self.fetch_asset_lock_outpoint_info(
            &asset_lock_outpoint,
            None,
            &platform_version.drive,
        )? {
            StoredAssetLockInfo::NotPresent => {}
            _ => {
                return Err(Error::Drive(DriveError::AssetLockOutpointAlreadyPresent(
                    "address funding fee estimation requires a fresh (unspent) asset lock outpoint",
                )));
            }
        }

        // Measure search-path levels from locally generated proofs against
        // committed state. The outer estimator accepts these reads only when
        // the GroveDB root hash is identical before and after the complete
        // state-dependent operation.
        let address_query = Drive::balance_for_clear_address_query(recipient);
        let address_proof = self.grove_get_proved_path_query(
            &address_query,
            None,
            &mut vec![],
            &platform_version.drive,
        )?;
        let clear_addresses_path = Self::clear_addresses_path();
        let clear_addresses_segments: Vec<&[u8]> = clear_addresses_path
            .iter()
            .map(|segment| segment.as_slice())
            .collect();
        let address_levels = single_key_proof_levels(
            &address_proof,
            &clear_addresses_segments,
            recipient.to_bytes().as_slice(),
        )?;

        let mut outpoint_query = PathQuery::new_single_key(
            vec![asset_lock_storage_path()[0].to_vec()],
            asset_lock_outpoint.to_vec(),
        );
        outpoint_query.query.limit = Some(1);
        let outpoint_proof = self.grove_get_proved_path_query(
            &outpoint_query,
            None,
            &mut vec![],
            &platform_version.drive,
        )?;
        let outpoint_levels = single_key_proof_levels(
            &outpoint_proof,
            &[asset_lock_storage_path()[0]],
            asset_lock_outpoint.as_slice(),
        )?;
        if outpoint_levels.present {
            // Not corruption: a block can commit between the fetch and the
            // proof, so the outpoint can legitimately appear in between.
            // Fail closed with what the second read actually observed.
            return Err(Error::Drive(DriveError::AssetLockOutpointAlreadyPresent(
                "the asset lock outpoint appeared in the state while the estimate was being \
                 computed",
            )));
        }

        // The exact production operations: build the real action and run it
        // through the production high-level converter, then convert to
        // low-level operations STATEFULLY (estimated layer info = None) so
        // the insert-vs-replace branch and the element bytes come from
        // committed state, exactly as during apply=true execution.
        let action = AddressFundingFromAssetLockTransitionAction::V0(
            AddressFundingFromAssetLockTransitionActionV0 {
                signable_bytes_hasher: SignableBytesHasher::Bytes(vec![]),
                asset_lock_value_to_be_consumed: AssetLockValue::new(
                    lock_credits,
                    vec![],
                    lock_credits,
                    vec![],
                    platform_version,
                )?,
                asset_lock_outpoint,
                inputs_with_remaining_balance: BTreeMap::new(),
                outputs: BTreeMap::from([(*recipient, None)]),
                input_contributions_total: 0,
                fee_strategy: vec![],
                user_fee_increase: 0,
            },
        );
        let high_level_operations =
            action.into_high_level_drive_operations(&block_info.epoch, platform_version)?;

        let mut stateful_marker: Option<HashMap<KeyInfoPath, EstimatedLayerInformation>> = None;
        let mut low_level_operations = vec![];
        for operation in high_level_operations {
            if operation.finalization_tasks(platform_version)?.is_some() {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "address funding fee estimation encountered an operation with finalization tasks",
                )));
            }
            low_level_operations.append(&mut operation.into_low_level_drive_operations(
                self,
                &mut stateful_marker,
                block_info,
                None,
                platform_version,
            )?);
        }

        // The server's own layer models, inserted in the same order the
        // converters would insert them in the apply=false path, so entries
        // shared between them (the root path) resolve to the same winner.
        let mut layer_map: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        Self::add_estimation_costs_for_total_system_credits_update(
            &mut layer_map,
            &platform_version.drive,
        )?;
        Self::add_estimation_costs_for_adding_asset_lock(&mut layer_map, &platform_version.drive)?;
        Self::add_estimation_costs_for_address_balance_update(
            &mut layer_map,
            &platform_version.drive,
        )?;
        // Replace ONLY the layer counts of the two data-dependent layers with
        // the measured levels; tree types and element sizes stay the server's
        // own models.
        set_measured_layer_count(
            &mut layer_map,
            KeyInfoPath::from_known_path(asset_lock_storage_path()),
            outpoint_levels.levels,
        )?;
        set_measured_layer_count(
            &mut layer_map,
            KeyInfoPath::from_known_owned_path(Self::clear_addresses_path()),
            address_levels.levels,
        )?;

        Ok((
            low_level_operations,
            layer_map,
            address_levels,
            outpoint_levels,
        ))
    }
}

/// How many times a multi-read committed-state operation may observe an
/// unstable root before giving up.
const SNAPSHOT_ATTEMPTS: usize = 3;

/// Runs `attempt` and accepts its output only when `root_sample` returns the
/// same value before and after it — i.e. no block committed underneath the
/// attempt's reads. An unstable attempt's output is discarded and the attempt
/// re-run, up to [`SNAPSHOT_ATTEMPTS`] times; persistent instability fails
/// with the retriable [`DriveError::CommittedStateChangedDuringOperation`].
/// Errors from either closure propagate immediately, without a retry.
fn stable_committed_read<T>(
    mut root_sample: impl FnMut() -> Result<[u8; 32], Error>,
    mut attempt: impl FnMut() -> Result<T, Error>,
) -> Result<T, Error> {
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let root_before = root_sample()?;
        let value = attempt()?;
        let root_after = root_sample()?;
        if root_before == root_after {
            return Ok(value);
        }
    }
    Err(Error::Drive(
        DriveError::CommittedStateChangedDuringOperation(
            "address funding fee estimation could not observe a stable committed state; retry",
        ),
    ))
}

fn set_measured_layer_count(
    layer_map: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    layer: KeyInfoPath,
    levels: u8,
) -> Result<(), Error> {
    let entry =
        layer_map
            .get_mut(&layer)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "estimated layer info is missing an expected layer",
            )))?;
    entry.estimated_layer_count = EstimatedLevel(levels as u32, false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::key_info::KeyInfo::KnownKey;
    use grovedb::batch::GroveOp;
    use grovedb::Element;

    fn address(n: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([n; 20])
    }

    fn outpoint(n: u8) -> Bytes36 {
        Bytes36([n; 36])
    }

    /// The production high-level operations for a 0-input / 1-output funding.
    fn funding_operations<'a>(
        recipient: &PlatformAddress,
        asset_lock_outpoint: Bytes36,
        lock_credits: Credits,
        platform_version: &PlatformVersion,
    ) -> Vec<DriveOperation<'a>> {
        let action = AddressFundingFromAssetLockTransitionAction::V0(
            AddressFundingFromAssetLockTransitionActionV0 {
                signable_bytes_hasher: SignableBytesHasher::Bytes(vec![]),
                asset_lock_value_to_be_consumed: AssetLockValue::new(
                    lock_credits,
                    vec![],
                    lock_credits,
                    vec![],
                    platform_version,
                )
                .expect("asset lock value"),
                asset_lock_outpoint,
                inputs_with_remaining_balance: BTreeMap::new(),
                outputs: BTreeMap::from([(*recipient, None)]),
                input_contributions_total: 0,
                fee_strategy: vec![],
                user_fee_increase: 0,
            },
        );
        action
            .into_high_level_drive_operations(&BlockInfo::default().epoch, platform_version)
            .expect("high level operations")
    }

    /// Applies a funding to committed state.
    fn seed_funding(
        drive: &Drive,
        recipient: &PlatformAddress,
        asset_lock_outpoint: Bytes36,
        lock_credits: Credits,
        platform_version: &PlatformVersion,
    ) {
        let operations = funding_operations(
            recipient,
            asset_lock_outpoint,
            lock_credits,
            platform_version,
        );
        drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("seed funding");
    }

    /// The real metered fee for a funding, measured inside a transaction that
    /// is dropped afterwards, so committed state is untouched.
    fn actual_fee_probe(
        drive: &Drive,
        recipient: &PlatformAddress,
        asset_lock_outpoint: Bytes36,
        lock_credits: Credits,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        let operations = funding_operations(
            recipient,
            asset_lock_outpoint,
            lock_credits,
            platform_version,
        );
        let transaction = drive.grove.start_transaction();
        let fee_result = drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("actual fee probe");
        drop(transaction);
        fee_result
    }

    fn root_hash(drive: &Drive, platform_version: &PlatformVersion) -> [u8; 32] {
        drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash")
    }

    const LOCK_CREDITS: Credits = 56_000_000;

    // ---------------------------------------------------------------
    // The committed-root stability contract, pinned deterministically
    // with scripted root samples — the drive-backed tests below only
    // ever exercise the quiescent first-attempt branch.
    // ---------------------------------------------------------------

    /// A scripted root sampler: returns the next hash from the list on each
    /// call, counting attempts as pairs of samples.
    fn scripted_roots(samples: Vec<[u8; 32]>) -> impl FnMut() -> Result<[u8; 32], Error> {
        let mut remaining = samples.into_iter();
        move || Ok(remaining.next().expect("script exhausted"))
    }

    #[test]
    fn test_stable_read_returns_the_first_stable_attempt() {
        let mut attempts = 0u32;
        let value = stable_committed_read(scripted_roots(vec![[1; 32], [1; 32]]), || {
            attempts += 1;
            Ok(attempts)
        })
        .expect("stable first attempt");
        assert_eq!(value, 1);
        assert_eq!(attempts, 1, "a stable attempt must not be re-run");
    }

    #[test]
    fn test_stable_read_discards_an_unstable_attempt_and_returns_a_later_stable_one() {
        // Attempt 1 sees roots 1→2 (unstable), attempt 2 sees 2→2 (stable).
        let mut attempts = 0u32;
        let value = stable_committed_read(
            scripted_roots(vec![[1; 32], [2; 32], [2; 32], [2; 32]]),
            || {
                attempts += 1;
                Ok(attempts)
            },
        )
        .expect("second attempt is stable");
        assert_eq!(
            value, 2,
            "the unstable attempt's value must be discarded, not returned"
        );
        assert_eq!(attempts, 2);
    }

    #[test]
    fn test_stable_read_fails_after_three_unstable_attempts() {
        let mut next_root = 0u8;
        let mut attempts = 0u32;
        let result = stable_committed_read(
            || {
                next_root += 1;
                Ok([next_root; 32])
            },
            || {
                attempts += 1;
                Ok(attempts)
            },
        );
        assert!(
            matches!(
                result,
                Err(Error::Drive(
                    DriveError::CommittedStateChangedDuringOperation(_)
                ))
            ),
            "persistent instability must fail with the retriable error, got {result:?}"
        );
        assert_eq!(attempts, 3, "exactly SNAPSHOT_ATTEMPTS attempts must run");
    }

    #[test]
    fn test_stable_read_propagates_attempt_errors_without_retry() {
        let mut attempts = 0u32;
        let result: Result<u32, Error> =
            stable_committed_read(scripted_roots(vec![[1; 32], [1; 32]]), || {
                attempts += 1;
                Err(Error::Drive(DriveError::CorruptedDriveState(
                    "boom".to_string(),
                )))
            });
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedDriveState(_)))
            ),
            "an attempt error must propagate as-is, got {result:?}"
        );
        assert_eq!(attempts, 1, "an errored attempt must not be retried");
    }

    /// The estimate must not write anything: the grove root hash is
    /// byte-identical before and after estimating for a new address, an
    /// existing address, and a rejected present outpoint.
    #[test]
    fn test_estimate_is_read_only_root_hash_unchanged() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        seed_funding(
            &drive,
            &address(1),
            outpoint(1),
            LOCK_CREDITS,
            platform_version,
        );
        seed_funding(
            &drive,
            &address(2),
            outpoint(2),
            LOCK_CREDITS,
            platform_version,
        );

        let before = root_hash(&drive, platform_version);

        drive
            .estimate_address_funding_fee(
                &address(200),
                outpoint(200),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect("estimate for new address");
        drive
            .estimate_address_funding_fee(
                &address(1),
                outpoint(201),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect("estimate for existing address");
        drive
            .estimate_address_funding_fee(
                &address(202),
                outpoint(1),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect_err("present outpoint must be rejected");

        let after = root_hash(&drive, platform_version);
        assert_eq!(
            before, after,
            "estimation must not change the grove root hash"
        );
    }

    /// Stateful op-building picks the branch from committed state: an insert
    /// (InsertOrReplace with a zero nonce) for a new address, and a replace
    /// (with the summed balance and the existing nonce) for an existing one —
    /// byte-identical to what apply=true execution would write.
    #[test]
    fn test_new_address_builds_insert_and_existing_builds_replace() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let existing = address(1);
        seed_funding(
            &drive,
            &existing,
            outpoint(1),
            LOCK_CREDITS,
            platform_version,
        );

        let clear_path = KeyInfoPath::from_known_owned_path(Drive::clear_addresses_path());

        let balance_write_for = |recipient: &PlatformAddress, op_n: u8| {
            let (low_level, _, _, _) = drive
                .estimate_address_funding_fee_parts_v0(
                    recipient,
                    outpoint(op_n),
                    LOCK_CREDITS,
                    &BlockInfo::default(),
                    platform_version,
                )
                .expect("estimate parts");
            low_level
                .into_iter()
                .find_map(|operation| match operation {
                    LowLevelDriveOperation::GroveOperation(op)
                        if op.path == clear_path
                            && op.key == Some(KnownKey(recipient.to_bytes())) =>
                    {
                        Some(op.op)
                    }
                    _ => None,
                })
                .expect("balance write for the recipient")
        };

        match balance_write_for(&address(200), 200) {
            GroveOp::InsertOrReplace {
                element: Element::ItemWithSumItem(nonce, sum, _),
            } => {
                assert_eq!(nonce, 0u32.to_be_bytes().to_vec(), "new address nonce");
                assert_eq!(sum, LOCK_CREDITS as i64, "new address balance");
            }
            other => panic!("expected an insert for a new address, got {other:?}"),
        }

        match balance_write_for(&existing, 201) {
            GroveOp::Replace {
                element: Element::ItemWithSumItem(_, sum, _),
            } => {
                assert_eq!(
                    sum,
                    (LOCK_CREDITS * 2) as i64,
                    "existing address balance must be summed from committed state"
                );
            }
            other => panic!("expected a replace for an existing address, got {other:?}"),
        }
    }

    /// The estimate brackets the real metered fee on the same state, for both
    /// the insert (new address) and the replace (existing address) branch, and
    /// orders them correctly: a replace carries no new storage bytes for the
    /// balance element, so both its estimate and its actual fee are lower.
    #[test]
    fn test_estimated_fee_brackets_actual_for_new_and_existing_address() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        for n in 1..=8u8 {
            seed_funding(
                &drive,
                &address(n),
                outpoint(n),
                LOCK_CREDITS,
                platform_version,
            );
        }

        // Observed samples on this test's state (protocol latest, 2026-08):
        // new address 13_175_300 vs 13_044_880 (+1.0%), existing address
        // 7_051_860 vs 6_802_640 (+3.7%). The band is regression headroom,
        // not a bound claim.
        let assert_brackets = |estimated: u64, actual: u64, what: &str| {
            assert!(
                estimated >= actual.saturating_mul(85) / 100
                    && estimated <= actual.saturating_mul(115) / 100,
                "{what}: estimated {estimated} not within [85%, 115%] of actual {actual}"
            );
        };

        let estimate_new = drive
            .estimate_address_funding_fee(
                &address(200),
                outpoint(200),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect("estimate new");
        let actual_new = actual_fee_probe(
            &drive,
            &address(200),
            outpoint(200),
            LOCK_CREDITS,
            platform_version,
        );
        assert!(!estimate_new.address_exists);
        assert_brackets(
            estimate_new.fee_result.total_base_fee(),
            actual_new.total_base_fee(),
            "new address",
        );

        let estimate_existing = drive
            .estimate_address_funding_fee(
                &address(1),
                outpoint(201),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect("estimate existing");
        let actual_existing = actual_fee_probe(
            &drive,
            &address(1),
            outpoint(201),
            LOCK_CREDITS,
            platform_version,
        );
        assert!(estimate_existing.address_exists);
        assert_brackets(
            estimate_existing.fee_result.total_base_fee(),
            actual_existing.total_base_fee(),
            "existing address",
        );

        assert!(
            actual_existing.storage_fee < actual_new.storage_fee,
            "replace must carry less storage fee than insert: {} vs {}",
            actual_existing.storage_fee,
            actual_new.storage_fee
        );
        assert!(
            estimate_existing.fee_result.storage_fee < estimate_new.fee_result.storage_fee,
            "estimated storage fee must order the same way: {} vs {}",
            estimate_existing.fee_result.storage_fee,
            estimate_new.fee_result.storage_fee
        );

        println!(
            "new address: estimated {} vs actual {} (storage {} vs {})",
            estimate_new.fee_result.total_base_fee(),
            actual_new.total_base_fee(),
            estimate_new.fee_result.storage_fee,
            actual_new.storage_fee,
        );
        println!(
            "existing address: estimated {} vs actual {} (storage {} vs {})",
            estimate_existing.fee_result.total_base_fee(),
            actual_existing.total_base_fee(),
            estimate_existing.fee_result.storage_fee,
            actual_existing.storage_fee,
        );
    }

    /// v0 scope pin: the estimate models a FRESH asset lock. An outpoint that
    /// is already in the spent-asset-lock tree — fully consumed (empty item)
    /// or partially consumed (serialized remainder) — is rejected with
    /// `AssetLockOutpointAlreadyPresent`, never silently priced.
    #[test]
    fn test_estimate_rejects_present_outpoint() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Fully consumed: a committed funding stores an empty item.
        seed_funding(
            &drive,
            &address(1),
            outpoint(1),
            LOCK_CREDITS,
            platform_version,
        );

        // Partially consumed: a used asset lock with a non-zero remainder
        // stores the serialized AssetLockValue.
        use crate::util::batch::DriveOperation::SystemOperation;
        use crate::util::batch::SystemOperationType;
        drive
            .apply_drive_operations(
                vec![SystemOperation(SystemOperationType::AddUsedAssetLock {
                    asset_lock_outpoint: outpoint(2),
                    asset_lock_value: AssetLockValue::new(
                        LOCK_CREDITS,
                        vec![0xAB; 25],
                        LOCK_CREDITS / 2,
                        vec![],
                        platform_version,
                    )
                    .expect("asset lock value"),
                })],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("seed partially consumed outpoint");

        for present in [outpoint(1), outpoint(2)] {
            let result = drive.estimate_address_funding_fee(
                &address(200),
                present,
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            );
            assert!(
                matches!(
                    result,
                    Err(Error::Drive(DriveError::AssetLockOutpointAlreadyPresent(_)))
                ),
                "present outpoint must be rejected, got {result:?}"
            );
        }
    }

    /// The measured layer levels grow with tree population and the estimate
    /// keeps bracketing the real metered fee as the trees deepen.
    #[test]
    fn test_estimate_tracks_actual_as_population_grows() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut seeded: u8 = 0;
        let mut last_levels = 0u8;
        for population in [0u8, 8, 40] {
            while seeded < population {
                seeded += 1;
                seed_funding(
                    &drive,
                    &address(seeded),
                    outpoint(seeded),
                    LOCK_CREDITS,
                    platform_version,
                );
            }

            let estimate = drive
                .estimate_address_funding_fee(
                    &address(200),
                    outpoint(200),
                    LOCK_CREDITS,
                    &BlockInfo::default(),
                    platform_version,
                )
                .expect("estimate");
            let actual = actual_fee_probe(
                &drive,
                &address(200),
                outpoint(200),
                LOCK_CREDITS,
                platform_version,
            );

            // Observed samples (protocol latest, 2026-08): population 0 →
            // 12_460_620 vs 12_551_520 (-0.7%), 8 → 13_175_300 vs 13_044_880
            // (+1.0%), 40 → 13_457_700 vs 13_330_880 (+1.0%). The band is
            // regression headroom, not a bound claim.
            let estimated = estimate.fee_result.total_base_fee();
            let actual_total = actual.total_base_fee();
            assert!(
                estimated >= actual_total.saturating_mul(85) / 100
                    && estimated <= actual_total.saturating_mul(115) / 100,
                "population {population}: estimated {estimated} not within [85%, 115%] of actual {actual_total}"
            );
            assert!(
                estimate.address_layer_levels >= last_levels,
                "measured levels must not shrink as the tree grows: {} then {}",
                last_levels,
                estimate.address_layer_levels
            );
            last_levels = estimate.address_layer_levels;

            println!(
                "population {population}: estimated {estimated} vs actual {actual_total}, \
                 address levels {}, outpoint levels {}",
                estimate.address_layer_levels, estimate.spent_asset_lock_layer_levels,
            );
        }
    }

    /// Protocol v11 regression: DRIVE_VERSION_V6 pins GROVE_V2, whose prove
    /// path emits the legacy `GroveDBProof::V0` envelope — the depth decoder
    /// must accept it, and the estimate must still bracket the real metered
    /// fee under that protocol version.
    #[test]
    fn test_estimate_works_under_protocol_v11_v0_proof_envelope() {
        let platform_version = PlatformVersion::get(11).expect("protocol v11");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        for n in 1..=4u8 {
            seed_funding(
                &drive,
                &address(n),
                outpoint(n),
                LOCK_CREDITS,
                platform_version,
            );
        }

        // Sanity for the regression itself: v11 must actually produce the
        // legacy V0 envelope, otherwise this test would not be exercising
        // the V0 decoding path.
        let probe_query = Drive::balance_for_clear_address_query(&address(200));
        let probe_proof = drive
            .grove_get_proved_path_query(&probe_query, None, &mut vec![], &platform_version.drive)
            .expect("prove under v11");
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<{ 256 * 1024 * 1024 }>();
        let (envelope, _): (grovedb::operations::proof::GroveDBProof, usize) =
            bincode::decode_from_slice(&probe_proof, config).expect("decode proof envelope");
        assert!(
            matches!(envelope, grovedb::operations::proof::GroveDBProof::V0(_)),
            "protocol v11 is expected to produce the legacy V0 proof envelope"
        );

        let estimate = drive
            .estimate_address_funding_fee(
                &address(200),
                outpoint(200),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect("the estimate must decode the V0 proof envelope under protocol v11");
        let actual = actual_fee_probe(
            &drive,
            &address(200),
            outpoint(200),
            LOCK_CREDITS,
            platform_version,
        );

        let estimated = estimate.fee_result.total_base_fee();
        let actual_total = actual.total_base_fee();
        assert!(
            estimated >= actual_total.saturating_mul(85) / 100
                && estimated <= actual_total.saturating_mul(115) / 100,
            "protocol v11: estimated {estimated} not within [85%, 115%] of actual {actual_total}"
        );
        assert!(!estimate.address_exists);
        assert!(estimate.address_layer_levels >= 1);
    }

    /// The layer map is the server's own model with ONLY the two
    /// data-dependent layer counts replaced by measured levels: tree types,
    /// element sizes, and every other layer stay byte-identical to the
    /// server's `add_estimation_costs_*` output.
    #[test]
    fn test_layer_map_keeps_server_shapes_and_measured_counts() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        for n in 1..=4u8 {
            seed_funding(
                &drive,
                &address(n),
                outpoint(n),
                LOCK_CREDITS,
                platform_version,
            );
        }

        let (_, layer_map, address_levels, outpoint_levels) = drive
            .estimate_address_funding_fee_parts_v0(
                &address(200),
                outpoint(200),
                LOCK_CREDITS,
                &BlockInfo::default(),
                platform_version,
            )
            .expect("estimate parts");

        let mut server_map: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
        Drive::add_estimation_costs_for_total_system_credits_update(
            &mut server_map,
            &platform_version.drive,
        )
        .expect("system credits estimation");
        Drive::add_estimation_costs_for_adding_asset_lock(&mut server_map, &platform_version.drive)
            .expect("asset lock estimation");
        Drive::add_estimation_costs_for_address_balance_update(
            &mut server_map,
            &platform_version.drive,
        )
        .expect("address balance estimation");

        assert_eq!(
            layer_map.len(),
            server_map.len(),
            "the engine must not add or drop layers"
        );

        let asset_lock_layer = KeyInfoPath::from_known_path(asset_lock_storage_path());
        let clear_addresses_layer =
            KeyInfoPath::from_known_owned_path(Drive::clear_addresses_path());

        for (layer, engine_info) in &layer_map {
            let server_info = server_map.get(layer).expect("layer known to the server");
            let is_patched = *layer == asset_lock_layer || *layer == clear_addresses_layer;
            if is_patched {
                let measured = if *layer == asset_lock_layer {
                    outpoint_levels.levels
                } else {
                    address_levels.levels
                };
                assert_eq!(
                    engine_info.estimated_layer_count,
                    EstimatedLevel(measured as u32, false),
                    "patched layer must carry the measured level count"
                );
                assert_eq!(
                    engine_info.tree_type, server_info.tree_type,
                    "patched layer must keep the server tree type"
                );
                assert_eq!(
                    engine_info.estimated_layer_sizes, server_info.estimated_layer_sizes,
                    "patched layer must keep the server element sizes"
                );
            } else {
                assert_eq!(
                    engine_info, server_info,
                    "unpatched layer must stay byte-identical to the server model"
                );
            }
        }
    }
}
