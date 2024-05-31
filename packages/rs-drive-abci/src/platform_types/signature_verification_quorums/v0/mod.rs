use bincode::{Decode, Encode};
use dpp::dashcore::QuorumHash;
use std::collections::BTreeMap;

pub use dpp::bls_signatures::PublicKey as ThresholdBlsPublicKey;

/// Quorum key per hash
pub type QuorumKeys = BTreeMap<QuorumHash, ThresholdBlsPublicKey>;

/// Previously obtained quorums and heights. Required for signature verification
#[derive(Debug, Clone, Encode, Decode, Default)]
pub struct PreviousQuorums {
    #[bincode(with_serde)]
    quorums: QuorumKeys,

    /// The core height at which these quorums were last active
    active_core_height: u32,

    /// The core height when the quorums were changed
    updated_at_core_height: u32,

    /// The core height the previous chain lock validating quorums became active
    previous_change_height: Option<u32>,
}

/// Quorums with keys for signature verification
#[derive(Debug, Clone, Encode, Decode, Default)]
pub struct SignatureVerificationQuorumsV0 {
    /// Current quorums
    #[bincode(with_serde)]
    last_quorums: QuorumKeys,

    /// The slightly old quorums used for validating chain locks (or instant locks), it's important to keep
    /// these because validation of signatures happens for the quorums that are 8 blocks before the
    /// height written in the chain lock. The same for instant locks
    previous: Option<PreviousQuorums>,
}

/// The trait defines methods for the signature verification quorums structure v0
pub trait SignatureVerificationQuorumsV0Methods {
    /// Set last quorum keys
    fn set_last_quorums(&mut self, quorums: QuorumKeys);

    /// Last quorum keys
    fn last_quorums(&self) -> &QuorumKeys;

    /// Last quorum keys mutable
    fn last_quorums_mut(&mut self) -> &mut QuorumKeys;

    /// Previous quorums
    fn previous_quorums(&self) -> Option<&PreviousQuorums>;

    /// Set last quorums keys and update previous quorums
    fn rotate_quorums(
        &mut self,
        quorums: QuorumKeys,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    );

    /// Update previous quorums
    fn update_previous_quorums(
        &mut self,
        previous_quorums: QuorumKeys,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    );

    /// Select quorum sets for signature verification
    fn select_quorums(
        &self,
        signing_height: u32,
        verification_height: u32,
    ) -> SelectedVerificationQuorumSets;
}

pub struct SelectedVerificationQuorumSets<'q> {
    pub quorum_sets: Vec<&'q QuorumKeys>,
    pub should_be_verifiable: bool,
}

impl<'q> Iterator for SelectedVerificationQuorumSets<'q> {
    type Item = &'q QuorumKeys;

    fn next(&mut self) -> Option<Self::Item> {
        self.quorum_sets.pop()
    }
}

impl<'q> SignatureVerificationQuorumsV0Methods for SignatureVerificationQuorumsV0 {
    fn set_last_quorums(&mut self, quorums: QuorumKeys) {
        self.last_quorums = quorums;
    }

    fn last_quorums(&self) -> &QuorumKeys {
        &self.last_quorums
    }

    fn last_quorums_mut(&mut self) -> &mut QuorumKeys {
        &mut self.last_quorums
    }

    fn previous_quorums(&self) -> Option<&PreviousQuorums> {
        self.previous.as_ref()
    }

    fn rotate_quorums(
        &mut self,
        quorums: QuorumKeys,
        last_active_height: u32,
        updated_at_core_height: u32,
    ) {
        let previous_quorums = std::mem::replace(&mut self.last_quorums, quorums);

        self.update_previous_quorums(previous_quorums, last_active_height, updated_at_core_height);
    }

    fn update_previous_quorums(
        &mut self,
        previous_quorums: QuorumKeys,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        self.previous = Some(PreviousQuorums {
            quorums: previous_quorums,
            active_core_height: last_active_core_height,
            updated_at_core_height,
            previous_change_height: self
                .previous
                .as_ref()
                .map(|previous| previous.updated_at_core_height),
        });
    }

    fn select_quorums(
        &self,
        signing_height: u32,
        verification_height: u32,
    ) -> SelectedVerificationQuorumSets {
        let mut quorums = Vec::new();
        let mut should_be_verifiable = false;

        if let Some(previous) = &self.previous {
            let previous_quorum_height = previous.active_core_height;
            let change_quorum_height = previous.updated_at_core_height;
            let previous_quorums_change_height = previous.previous_change_height;

            if signing_height > 8 && verification_height >= change_quorum_height {
                // in this case we are sure that we should be targeting the current quorum
                // We updated core chain lock height from 100 to 105, new chain lock comes in for block 114
                //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 106 (new chain lock verification height 114 - 8)
                // We are sure that we should use current quorums
                // If we have
                //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 105 (new chain lock verification height 113 - 8)
                // We should also use current quorums, this is because at 105 we are sure new chain lock validating quorums are active
                quorums.push(&self.last_quorums);
                should_be_verifiable = true;
            } else if signing_height > 8 && verification_height <= previous_quorum_height {
                should_be_verifiable = previous_quorums_change_height
                    .map(|previous_quorums_change_height| {
                        verification_height > previous_quorums_change_height
                    })
                    .unwrap_or(false);
                // In this case the quorums were changed recently meaning that we should use the previous quorums to verify the chain lock
                // We updated core chain lock height from 100 to 105, new chain lock comes in for block 106
                // -------- 98 (new chain lock verification height 106 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                // We are sure that we should use previous quorums
                // If we have
                // -------- 100 (new chain lock verification height 108 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                // We should also use previous quorums, this is because at 100 we are sure the old quorum set was active
                quorums.push(&previous.quorums);
            } else {
                should_be_verifiable = previous_quorums_change_height
                    .map(|previous_quorums_change_height| {
                        verification_height > previous_quorums_change_height
                    })
                    .unwrap_or(false);
                // we are in between, so we don't actually know if it was the old one or the new one to be used.
                //  ------- 100 (previous_quorum_height) ------ 104 (new chain lock verification height 112 - 8) -------105 (change_quorum_height)
                // we should just try both, starting with the current quorums
                quorums.push(&self.last_quorums);
                quorums.push(&previous.quorums);
            }
        } else {
            quorums.push(&self.last_quorums);
        }

        SelectedVerificationQuorumSets {
            quorum_sets: quorums,
            should_be_verifiable,
        }
    }
}
