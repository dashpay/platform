use crate::error::Error;
use arc_swap::ArcSwap;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::{PlatformVersion, ProtocolVersion};
use std::collections::BTreeMap;
use std::sync::Arc;

/// How many distinct protocol versions the cache keeps materializations for.
///
/// Live reads only ever ask for two: the committed protocol version — used by `check_tx`, which
/// validates against the last committed state on its own connection and thread pool, and by
/// ordinary block execution — and, on the first block of an upgrade, the candidate version the
/// block is being executed at. Anything older is dead weight, so inserting a materialization
/// for a third version drops the lowest one.
const MAX_MEMOIZED_PROTOCOL_VERSIONS: usize = 2;

/// Materializations held by [`SystemDataContracts`], keyed so that entries for one protocol
/// version can never be reached by a read pinned to another.
type Materializations = BTreeMap<(ProtocolVersion, SystemDataContract), Arc<DataContract>>;

/// Memoized materializations of the compiled-in system data contracts.
///
/// `load_system_data_contract(variant, platform_version)` is a deterministic pure function of
/// the variant and the protocol version, and its result changes across protocol versions (the
/// DPNS `domain` document type gains its history flags at protocol version 13, for instance).
/// This cache only avoids repeating that work — schema compilation and validation — so it holds
/// no authoritative state: any entry may be dropped and rebuilt with a bit-identical result.
///
/// There is no "current" contract. Every read is pinned to the protocol version the caller is
/// executing under, which it already carries in its [`PlatformVersion`]. That matters because
/// materializations are loaded **speculatively**: the first block of a protocol change is
/// executed for a candidate block that may be rejected, and the in-memory cache has no part in
/// the grovedb rollback that follows. Keying every entry by its protocol version keeps the
/// candidate's materializations on keys no committed-version read can reach, so a rejected
/// candidate is inert rather than something that has to be undone.
///
/// Reads are lock-free and writes replace the whole map. The set of materializations only
/// changes when a protocol version is first read from — a handful of times per protocol
/// upgrade — while reads happen on every block and every contract fetch, so cloning a map of
/// at most a dozen `Arc` pointers on that rare path is cheaper than making every reader
/// contend on a lock word.
pub struct SystemDataContracts {
    materialized: ArcSwap<Materializations>,
}

impl Default for SystemDataContracts {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemDataContracts {
    /// Creates an empty cache. Contracts are materialized on first use.
    pub fn new() -> Self {
        SystemDataContracts {
            materialized: ArcSwap::from_pointee(Materializations::new()),
        }
    }

    /// Returns `system_contract` materialized for `platform_version`, reusing the memoized
    /// materialization when one is present.
    ///
    /// The contract is materialized at the protocol version the caller is executing at, which
    /// is what the state holds: whenever a system contract's materialization changes at a
    /// protocol version, the first block of that change rewrites the persisted contract (see
    /// `perform_events_on_first_block_of_protocol_change`).
    ///
    /// # Errors
    /// Propagates any error from `load_system_data_contract`, notably when a contract's schema
    /// is not expressible under `platform_version` — which is the case for contracts asked for
    /// below their activation version.
    pub fn load(
        &self,
        system_contract: SystemDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        let key = (platform_version.protocol_version, system_contract);

        let materialized = self.materialized.load();
        if let Some(contract) = materialized.get(&key) {
            return Ok(Arc::clone(contract));
        }
        drop(materialized);

        let contract = Arc::new(load_system_data_contract(
            system_contract,
            platform_version,
        )?);

        // Copy-on-write publish. The closure is pure and idempotent, so `rcu` re-running it
        // under a concurrent publish is harmless; and because materialization is a pure
        // function of the key, a racing thread that wins the swap has stored an identical
        // contract, which is why returning our own is equivalent.
        self.materialized.rcu(|materialized| {
            let mut next = Materializations::clone(materialized);
            next.insert(key, Arc::clone(&contract));
            Self::drop_stale_protocol_versions(&mut next);
            next
        });

        Ok(contract)
    }

    /// Returns the withdrawals contract materialized for `platform_version`.
    pub fn load_withdrawals(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::Withdrawals, platform_version)
    }

    /// Returns the token history contract materialized for `platform_version`.
    pub fn load_token_history(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::TokenHistory, platform_version)
    }

    /// Returns the DPNS contract materialized for `platform_version`.
    pub fn load_dpns(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::DPNS, platform_version)
    }

    /// Returns the Dashpay contract materialized for `platform_version`.
    pub fn load_dashpay(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::Dashpay, platform_version)
    }

    /// Returns the masternode reward shares contract materialized for `platform_version`.
    pub fn load_masternode_reward_shares(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::MasternodeRewards, platform_version)
    }

    /// Returns the keyword search contract materialized for `platform_version`.
    pub fn load_keyword_search(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::KeywordSearch, platform_version)
    }

    /// Returns the document history contract materialized for `platform_version`.
    pub fn load_document_history(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContract>, Error> {
        self.load(SystemDataContract::DocumentHistory, platform_version)
    }

    /// Returns the system contract whose deterministic identifier matches `id`, materialized
    /// for `platform_version`.
    ///
    /// Returns `None` for user contracts, for system contracts this cache does not materialize
    /// (`WalletUtils`, which lives only in grovedb), and for system contracts that are not yet
    /// active at `platform_version`: before activation the contract does not exist in the
    /// state, so the lookup must fall through to the billed grovedb fetch and report it absent
    /// exactly like a non-upgraded node would.
    pub fn find_by_id(
        &self,
        id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, Error> {
        // Linear scan over each contract's static id. The set is small and fixed, which makes
        // this cheaper than building and holding a map.
        let Some(&system_contract) = SystemDataContract::ALL
            .iter()
            .find(|system_contract| system_contract.id() == id)
        else {
            return Ok(None);
        };

        // Which contracts this cache may answer for, and from which protocol version each one
        // exists in the state. Exhaustive so that a new system contract cannot be added without
        // deciding both. Before its activation a contract is absent from the state, so the
        // lookup must fall through to the billed grovedb fetch and report it missing, exactly
        // as a node that has not upgraded does — answering early would turn a billed "not
        // found" into a free "found".
        let activated_at_protocol_version: ProtocolVersion = match system_contract {
            // Registered in the genesis state.
            SystemDataContract::Withdrawals
            | SystemDataContract::MasternodeRewards
            | SystemDataContract::DPNS
            | SystemDataContract::Dashpay => 1,
            // Written to state by the transition to protocol version 9.
            SystemDataContract::TokenHistory | SystemDataContract::KeywordSearch => 9,
            // Written to state by the transition to protocol version 13.
            SystemDataContract::DocumentHistory => 13,
            // Never served from this cache: `WalletUtils` is only ever read from grovedb, and
            // the reserved `FeatureFlags` slot has no implementation.
            SystemDataContract::WalletUtils | SystemDataContract::FeatureFlags => return Ok(None),
        };

        if activated_at_protocol_version > platform_version.protocol_version {
            return Ok(None);
        }

        self.load(system_contract, platform_version).map(Some)
    }

    /// Drops materializations for protocol versions below `protocol_version`.
    ///
    /// Call this when a block commits, passing the committed protocol version: from that point
    /// every live read asks for it or higher — `check_tx` validates against the committed
    /// state, and the next block executes at the committed version until another upgrade
    /// proposes a candidate — so everything below is garbage. Materializations *above* it are kept: those belong to an
    /// upgrade candidate that was proposed and will be proposed again.
    ///
    /// Nothing depends on this being called. Entries are reproducible, so skipping it costs
    /// memory bounded by [`MAX_MEMOIZED_PROTOCOL_VERSIONS`], never correctness.
    pub fn drop_versions_below(&self, protocol_version: ProtocolVersion) {
        // Checked before publishing so that ordinary blocks, which have nothing to drop, do no
        // work beyond a lock-free read.
        if self
            .materialized
            .load()
            .keys()
            .all(|(memoized, _)| *memoized >= protocol_version)
        {
            return;
        }

        self.materialized.rcu(|materialized| {
            let mut next = Materializations::clone(materialized);
            next.retain(|(memoized, _), _| *memoized >= protocol_version);
            next
        });
    }

    /// Keeps materializations for at most [`MAX_MEMOIZED_PROTOCOL_VERSIONS`] protocol versions
    /// by dropping the lowest ones.
    ///
    /// Dropping is always safe: every entry is reproducible from its key alone, so a discarded
    /// materialization is rebuilt identically the next time it is read.
    fn drop_stale_protocol_versions(materialized: &mut Materializations) {
        // Keys are ordered by protocol version first, so equal versions form runs that `dedup`
        // collapses into the ascending list of distinct versions held.
        let mut protocol_versions: Vec<ProtocolVersion> = materialized
            .keys()
            .map(|(protocol_version, _)| *protocol_version)
            .collect();
        protocol_versions.dedup();

        let Some(lowest_kept) = protocol_versions
            .len()
            .checked_sub(MAX_MEMOIZED_PROTOCOL_VERSIONS)
            .and_then(|first_kept| protocol_versions.get(first_kept))
            .copied()
        else {
            return;
        };

        materialized.retain(|(protocol_version, _), _| *protocol_version >= lowest_kept);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    fn platform_version(protocol_version: ProtocolVersion) -> &'static PlatformVersion {
        PlatformVersion::get(protocol_version).expect("expected a supported platform version")
    }

    fn dpns_history_flags(contract: &DataContract) -> (bool, bool, bool) {
        let domain = contract
            .document_type_for_name("domain")
            .expect("DPNS must define the domain document type");

        (
            domain.documents_keep_transfer_history(),
            domain.documents_keep_purchase_history(),
            domain.documents_keep_pricing_history(),
        )
    }

    #[test]
    fn reloading_v13_must_not_make_dpns_v2_visible_to_v12_reads() {
        let contracts = SystemDataContracts::new();

        assert_eq!(
            dpns_history_flags(
                &contracts
                    .find_by_id(SystemDataContract::DPNS.id(), platform_version(12))
                    .expect("expected the v12 DPNS lookup to succeed")
                    .expect("DPNS must be active at protocol v12")
            ),
            (false, false, false)
        );

        // Stand in for the speculative load performed on the first block of a candidate
        // protocol change, which happens before that block is known to commit.
        contracts
            .load_dpns(platform_version(13))
            .expect("speculatively materialize protocol v13 DPNS");

        assert_eq!(
            dpns_history_flags(
                &contracts
                    .find_by_id(SystemDataContract::DPNS.id(), platform_version(12))
                    .expect("expected the v12 DPNS lookup to succeed")
                    .expect("DPNS must remain available at protocol v12")
            ),
            (false, false, false),
            "a speculative v13 materialization must preserve explicit v12 reads"
        );
        assert_eq!(
            dpns_history_flags(
                &contracts
                    .load_dpns(platform_version(12))
                    .expect("expected the v12 DPNS accessor to succeed")
            ),
            (false, false, false),
            "a speculative v13 materialization must preserve v12 accessor reads"
        );
        assert_eq!(
            dpns_history_flags(
                &contracts
                    .find_by_id(SystemDataContract::DPNS.id(), platform_version(13))
                    .expect("expected the v13 DPNS lookup to succeed")
                    .expect("DPNS must be active at protocol v13")
            ),
            (true, true, true)
        );
    }

    #[test]
    fn same_feature_version_must_preserve_distinct_protocol_materializations() {
        let contracts = SystemDataContracts::new();
        let platform_version_8 = platform_version(8);
        let platform_version_9 = platform_version(9);
        let expected_v8 =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version_8)
                .expect("load protocol v8 withdrawals");
        let expected_v9 =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version_9)
                .expect("load protocol v9 withdrawals");
        assert_ne!(
            expected_v8, expected_v9,
            "the regression requires distinct materialized contracts"
        );

        contracts
            .load_withdrawals(platform_version_8)
            .expect("materialize protocol v8 withdrawals");
        contracts
            .load_withdrawals(platform_version_9)
            .expect("speculatively materialize protocol v9 withdrawals");

        assert_eq!(
            contracts
                .find_by_id(SystemDataContract::Withdrawals.id(), platform_version_8)
                .expect("expected the v8 withdrawals lookup to succeed")
                .expect("withdrawals must be active at protocol v8")
                .as_ref(),
            &expected_v8,
            "a v9 materialization must preserve the protocol v8 one"
        );
        assert_eq!(
            contracts
                .find_by_id(SystemDataContract::Withdrawals.id(), platform_version_9)
                .expect("expected the v9 withdrawals lookup to succeed")
                .expect("withdrawals must be active at protocol v9")
                .as_ref(),
            &expected_v9
        );
    }

    #[test]
    fn document_history_cache_respects_its_activation_version() {
        let contracts = SystemDataContracts::new();

        assert!(contracts
            .find_by_id(
                SystemDataContract::DocumentHistory.id(),
                platform_version(12)
            )
            .expect("expected the pre-activation lookup to succeed")
            .is_none());
        assert!(contracts
            .find_by_id(
                SystemDataContract::DocumentHistory.id(),
                platform_version(13)
            )
            .expect("expected the v13 lookup to succeed")
            .is_some());
    }

    fn memoized_protocol_versions(contracts: &SystemDataContracts) -> Vec<ProtocolVersion> {
        let materialized = contracts.materialized.load();
        let mut protocol_versions: Vec<ProtocolVersion> = materialized
            .keys()
            .map(|(protocol_version, _)| *protocol_version)
            .collect();
        protocol_versions.dedup();
        protocol_versions
    }

    /// A chain replayed from genesis crosses every protocol upgrade, so the cache must not
    /// accumulate a materialization set per version it has ever executed at.
    #[test]
    fn committing_a_block_releases_the_outgoing_protocol_version() {
        let contracts = SystemDataContracts::new();

        for committed_protocol_version in 9..=14 {
            // The block that switches protocol version executes at the candidate while
            // `check_tx` still validates against the committed one, so both are live at once.
            contracts
                .load_dpns(platform_version(committed_protocol_version))
                .expect("materialize DPNS for the block being executed");
            assert!(
                memoized_protocol_versions(&contracts).len() <= 2,
                "at most the committed and candidate versions may be live"
            );

            contracts.drop_versions_below(committed_protocol_version);
            assert_eq!(
                memoized_protocol_versions(&contracts),
                vec![committed_protocol_version],
                "committing the switch must release the outgoing version"
            );
        }
    }

    /// The commit-time release is an optimisation, not a guarantee the bound relies on.
    #[test]
    fn memoization_is_bounded_to_the_live_protocol_versions() {
        let contracts = SystemDataContracts::new();

        for protocol_version in [9, 10, 11, 12, 13, 14] {
            contracts
                .load_dpns(platform_version(protocol_version))
                .expect("materialize DPNS");
        }

        assert_eq!(
            memoized_protocol_versions(&contracts),
            vec![13, 14],
            "only the most recent protocol versions stay memoized"
        );

        // An evicted protocol version is rebuilt identically, so eviction is not observable.
        assert_eq!(
            dpns_history_flags(
                &contracts
                    .load_dpns(platform_version(9))
                    .expect("re-materialize protocol v9 DPNS")
            ),
            (false, false, false)
        );
    }
}
