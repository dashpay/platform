/// A deterministic count of smart-contract work.
///
/// One unit is the weight the active metering generation assigns to one admitted guest
/// operation or one slice of host work performed on the guest's behalf (host entry, byte
/// copying, memory growth, native operation cost). The weights themselves live with the
/// metering generation, not here. A unit is never wall-clock time: two nodes that run the same
/// invocation under the same protocol version count exactly the same number of units, whatever
/// their hardware, cache state or compiler backend.
///
/// Consumption is reported in this unit by the runtime and priced in credits by the fee
/// schedule (`FeeVersion::dashvm`), see `dpp::fee::smart_contract_computation`.
pub type ComputationUnits = u64;

/// The consensus limits on smart-contract computation.
///
/// Both limits are counted in [`ComputationUnits`] by one contract-only counter. That counter is
/// separate from every native budget: the proposer's wall-clock timer, the per-block withdrawal
/// and shielded caps and the Tenderdash block gas limit all keep their existing meaning and none
/// of them is derived from these numbers.
///
/// # Per invocation
///
/// `max_computation_units_per_invocation` bounds one outer contract invocation: a direct call,
/// a predicate evaluated for a native transition, or one scheduled attempt. Everything the
/// invocation causes is charged to that one counter, including module initialisation, nested
/// contract calls, predicates those calls trigger and the host work they request. Nothing is
/// counted twice: a nested call shares its caller's counter rather than opening a second one.
/// The runtime receives this value (or a smaller bound the caller declares and can afford) as
/// the budget of the invocation, meters guest and host work against it, and reports the units
/// actually consumed. A failed invocation still consumed its units and is charged for them.
///
/// # Per block
///
/// `max_computation_units_per_block` bounds the sum of all contract invocations in one block,
/// ordinary and scheduled together. The block loop reserves an invocation's admitted bound
/// against the block before it runs and settles the actual consumption afterwards, so an
/// invocation that would not fit is never started; it is delayed by the proposer and rejected by
/// a validator, never failed part-way through. The ledger that does this bookkeeping lives in
/// `drive-abci` (`BlockComputationBudget`).
///
/// # Credits and gas
///
/// Units become credits through the protocol-versioned price in the fee schedule
/// (`FeeVersion::dashvm.credits_per_computation_unit`, checked multiplication). The resulting
/// charge enters the processing fee of the invocation's `FeeResult`, which is what
/// Tenderdash's `gas_used` and `gas_wanted` already report. Gas therefore stays denominated in
/// credits; no unit equivalence between computation units and Tenderdash gas is introduced.
///
/// # Versioning
///
/// `None` on every protocol version that predates smart contracts: no code path meters, prices
/// or budgets contract computation there. The numbers on the first version that carries
/// `Some` are provisional until measured; see the `SYSTEM_LIMITS_V*` that sets them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SmartContractComputationLimits {
    /// Most computation units one outer contract invocation may consume, counting every nested
    /// call, predicate, module initialisation and host entry it causes.
    pub max_computation_units_per_invocation: ComputationUnits,
    /// Most computation units all contract invocations in one block may consume together,
    /// ordinary and scheduled.
    pub max_computation_units_per_block: ComputationUnits,
}

impl SmartContractComputationLimits {
    /// Whether the limits can be enforced at all: both are non-zero (a zero limit would reject
    /// every invocation) and one invocation fits in a block (otherwise the per-invocation limit
    /// could never be reached and the per-block reservation would refuse a maximal invocation).
    /// This is the only invariant that must hold whatever the measured numbers turn out to be;
    /// the table that sets the numbers asserts it at compile time.
    pub const fn is_well_formed(&self) -> bool {
        self.max_computation_units_per_invocation > 0
            && self.max_computation_units_per_block >= self.max_computation_units_per_invocation
    }
}
