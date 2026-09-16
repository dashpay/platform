//! Verdict of the last gap-limit identity scan — what it probed, what it
//! could not answer, and how a later scan's verdict folds over it.
//!
//! Carried on [`PlatformWalletChangeSet::identity_scan_state`] and restored
//! through [`IdentityManagerStartState::scan_states`]; the startup sequence
//! reads it to decide whether the warm-launch shortcut may skip discovery.
//!
//! [`PlatformWalletChangeSet::identity_scan_state`]: crate::changeset::PlatformWalletChangeSet::identity_scan_state
//! [`IdentityManagerStartState::scan_states`]: crate::changeset::IdentityManagerStartState::scan_states

/// Whether the last gap-limit identity scan for this wallet answered every
/// index it probed.
///
/// A scan has three endings, and only two of them are visible in what it
/// returns. It can find identities, it can prove there are none, or it can
/// find *some* while one of its probes goes unanswered — and that third
/// ending returns `Ok` with the identities it did find, because discarding
/// them would be worse. `ScanTally::is_trustworthy` is
/// `identities_seen > 0 || failed_probes == 0`, so a scan that saw index 0
/// and got no answer at index 1 is reported as a success.
///
/// That is survivable only if something scans again. Nothing did: the
/// warm-launch shortcut skips discovery whenever any identity is on file, and
/// the fact that the scan behind that identity was partial existed nowhere
/// once the process exited. An identity at the unanswered index then stayed
/// invisible for the life of the installation, along with all of its contacts
/// — a silent, permanent gap whose only symptom is a missing identity and
/// DPNS name after a restore.
///
/// This is that missing fact. `complete` is stored rather than derived from
/// `failed_indices` because the two ways a scan can end early are different:
/// unanswered probes leave indices behind, while a scan abandoned at the
/// startup budget leaves none and is no more complete for it.
///
/// Carried as `Option<IdentityScanStateEntry>` — at most one scan verdict per
/// persist round. A newer verdict is folded over the older one rather than
/// replacing it outright; see [`IdentityScanStateEntry::superseding`] for why
/// replacing loses gaps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentityScanStateEntry {
    /// Every index the scan probed was answered, and nothing an earlier scan
    /// left unanswered is still outstanding. Only a `true` here may let a
    /// later launch skip discovery.
    pub complete: bool,
    /// The lowest index the scan probed. Together with
    /// [`Self::probed_through`] this is the scan's coverage — what it is
    /// entitled to have an opinion about, and the reason a suffix scan cannot
    /// clear a gap below where it started.
    pub probed_from: u32,
    /// One past the highest index the scan probed.
    pub probed_through: u32,
    /// Indices whose probe never got an answer, ascending — this scan's own,
    /// plus any an earlier scan left that this one did not cover. Empty for a
    /// scan that was cut off before it could fail anything.
    pub failed_indices: Vec<u32>,
    /// A scan ended without naming where its gap was, and nothing has covered
    /// that region since.
    ///
    /// A scan abandoned mid-await answered no index and failed none, so
    /// [`Self::failed_indices`] cannot speak for it: what it never reached has
    /// no name. Only a scan that starts at index 0 and answers everything it
    /// probed covers a region nobody can point at, so the fact rides the state
    /// until one does.
    ///
    /// Stored rather than read back off `failed_indices` because a fold mixes
    /// the two kinds of gap. An unlocated gap followed by a suffix scan with
    /// unanswered probes of its own produces a state with a non-empty failed
    /// list, at which point the derived reading says "located" and a later
    /// suffix scan covering those names hands the shortcut back over the
    /// original gap.
    pub unlocated_gap: bool,
}

impl IdentityScanStateEntry {
    /// A scan that answered every index in `probed_from..probed_through`.
    pub fn completed(probed_from: u32, probed_through: u32) -> Self {
        Self {
            complete: true,
            probed_from,
            probed_through,
            failed_indices: Vec::new(),
            unlocated_gap: false,
        }
    }

    /// A scan that left at least one index unanswered, or was abandoned
    /// before it could finish.
    pub fn incomplete(probed_from: u32, probed_through: u32, failed_indices: Vec<u32>) -> Self {
        Self {
            complete: false,
            // A scan that named an unanswered index located its gap; one that
            // named none was cut off before it could, and its gap has no name.
            unlocated_gap: failed_indices.is_empty(),
            probed_from,
            probed_through,
            failed_indices,
        }
    }

    /// Fold this scan's verdict over `previous`, the one already on record.
    ///
    /// A scan answers the range it walked and nothing else, so an index
    /// `previous` recorded as unanswered is still unanswered unless this scan
    /// covered it. Replacing the verdict outright is what let a clean suffix
    /// scan erase a gap it never probed: discovery resumes one past the
    /// highest registered identity by default, so a wallet with identities at
    /// 0 and 2 and no answer at 1 resumes at 3, answers everything from there
    /// cleanly, and publishes `complete` — after which the warm-launch
    /// shortcut reports a settled identity set while the identity at index 1
    /// and all of its contacts stay missing. That is the same
    /// Ready-over-an-unprobed-gap failure the verdict exists to prevent,
    /// reached from the other side.
    ///
    /// A gap this scan covered and answered is cleared; one it re-probed and
    /// still could not answer is already among its own `failed_indices`. A gap
    /// nobody could name is carried in [`Self::unlocated_gap`], which only a
    /// clean scan starting at index 0 clears — a from-zero scan cut short
    /// covered no more than the window it walked, so the unknown region above
    /// it is still unknown. The folds in between may add and clear named gaps
    /// freely without touching it. The result is complete only when this scan
    /// was clean AND it left nothing carried over of either kind.
    pub fn superseding(mut self, previous: &Self) -> Self {
        let covered = self.probed_from..self.probed_through;
        for index in &previous.failed_indices {
            if !covered.contains(index) && !self.failed_indices.contains(index) {
                self.failed_indices.push(*index);
            }
        }
        self.failed_indices.sort_unstable();

        // An unlocated gap is carried as a fact rather than re-derived from
        // `failed_indices`, which cannot hold a gap that has no name. Only a
        // scan that starts at the bottom of the index space and answers
        // everything it probed can be said to have covered it: one that starts
        // there and is itself cut short walked only as far as it got, and the
        // region above that is the same one nobody could point at. So nothing
        // narrower and nothing unfinished supersedes it — until one does it
        // survives every fold in between, including the ones that put named
        // gaps of their own into the list.
        let supersedes_unlocated_gap = self.complete && self.probed_from == 0;
        self.unlocated_gap =
            self.unlocated_gap || (previous.unlocated_gap && !supersedes_unlocated_gap);

        self.complete = self.complete && self.failed_indices.is_empty() && !self.unlocated_gap;
        self
    }
}
