use crate::config::QuorumLikeConfig;
use crate::platform_types::core_quorum_set::v0::quorums::{QuorumVerificationData, Quorums};
use dashcore_rpc::json::QuorumType;
use dpp::dashcore::QuorumSigningRequestId;
use std::vec::IntoIter;

/// Previously obtained quorums and heights. Required for signature verification
#[derive(Debug, Clone)]
pub(super) struct PreviousQuorumsV0 {
    pub(super) quorums: Quorums,

    /// The core height at which these quorums were last active
    pub(super) active_core_height: u32,

    /// The core height when the quorums were changed
    pub(super) updated_at_core_height: u32,

    /// The core height the previous chain lock validating quorums became active
    pub(super) previous_change_height: Option<u32>,
}

/// Quorums with keys for signature verification
#[derive(Debug, Clone)]
pub struct CoreQuorumSetV0 {
    /// Quorum configuration
    pub(super) config: QuorumConfig,

    /// Current quorums
    pub(super) current_quorums: Quorums,

    /// The slightly old quorums used for validating ch ain locks (or instant locks), it's important to keep
    /// these because validation of signatures happens for the quorums that are 8 blocks before the
    /// height written in the chain lock. The same for instant locks
    pub(super) previous: Option<PreviousQuorumsV0>,
}

/// The trait defines methods for the signature verification quorums structure v0
pub trait CoreQuorumSetV0Methods {
    /// Config
    fn config(&self) -> &QuorumConfig;

    /// Set current quorum keys
    fn set_current_quorums(&mut self, quorums: Quorums);

    /// Current quorum
    fn current_quorums(&self) -> &Quorums;

    /// Last quorum keys mutable
    fn current_quorums_mut(&mut self) -> &mut Quorums;

    /// Has previous quorums?
    fn has_previous_quorums(&self) -> bool;

    /// Set last quorums keys and update previous quorums
    fn replace_quorums(
        &mut self,
        quorums: Quorums,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    );

    /// Update previous quorums
    fn update_previous_quorums(
        &mut self,
        previous_quorums: Quorums,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    );

    /// Select quorums for signature verification
    fn select_quorums(
        &self,
        signing_height: u32,
        verification_height: u32,
        request_id: QuorumSigningRequestId,
    ) -> QuorumsVerificationDataIterator;
}

/// Iterator over selected quorum sets and specific quorums based on request_id and quorum configuration
pub struct QuorumsVerificationDataIterator<'q> {
    /// Quorum configuration
    config: &'q QuorumConfig,
    /// Request ID to chose right quorum
    request_id: QuorumSigningRequestId,
    /// Appropriate quorum sets
    quorum_sets: IntoIter<&'q Quorums>,
    /// Should we expect signature verification to be successful
    should_be_verifiable: bool,
}

impl<'p> Iterator for QuorumsVerificationDataIterator<'p> {
    type Item = QuorumVerificationData<'p>;

    fn next(&mut self) -> Option<Self::Item> {
        let quorum_set = self.quorum_sets.next();

        let Some(quorum_set) = quorum_set else {
            return None;
        };

        if self.config.rotation {
            quorum_set.find_rotating_quorum_verification_data(
                self.config.active_signers,
                self.request_id.as_ref(),
            )
        } else {
            quorum_set.find_classic_quorum_verification_data(
                self.config.quorum_type,
                self.request_id.as_ref(),
            )
        }
    }
}

impl<'q> QuorumsVerificationDataIterator<'q> {
    /// Number of quorum sets
    pub fn len(&self) -> usize {
        self.quorum_sets.len()
    }

    /// Does the iterator have any quorum sets
    pub fn is_empty(&self) -> bool {
        self.quorum_sets.len() == 0
    }

    /// Should we expect signature verification to be successful
    pub fn should_be_verifiable(&self) -> bool {
        self.should_be_verifiable
    }
}

/// Quorum configuration
#[derive(Debug, Clone)]
pub struct QuorumConfig {
    /// Type
    pub quorum_type: QuorumType,
    /// Active quorum signers count
    pub active_signers: u16,
    /// Is it a DIP24 rotating quorum or classic
    pub rotation: bool,
    /// DKG interval
    pub window: u32,
}

impl CoreQuorumSetV0Methods for CoreQuorumSetV0 {
    fn config(&self) -> &QuorumConfig {
        &self.config
    }

    fn set_current_quorums(&mut self, quorums: Quorums) {
        self.current_quorums = quorums;
    }

    fn current_quorums(&self) -> &Quorums {
        &self.current_quorums
    }

    fn current_quorums_mut(&mut self) -> &mut Quorums {
        &mut self.current_quorums
    }

    fn has_previous_quorums(&self) -> bool {
        self.previous.is_some()
    }

    fn replace_quorums(
        &mut self,
        quorums: Quorums,
        last_active_height: u32,
        updated_at_core_height: u32,
    ) {
        let previous_quorums = std::mem::replace(&mut self.current_quorums, quorums);

        self.update_previous_quorums(previous_quorums, last_active_height, updated_at_core_height);
    }

    fn update_previous_quorums(
        &mut self,
        previous_quorums: Quorums,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        self.previous = Some(PreviousQuorumsV0 {
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
        request_id: QuorumSigningRequestId,
    ) -> QuorumsVerificationDataIterator {
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
                quorums.push(&self.current_quorums);
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
                quorums.push(&self.current_quorums);
                quorums.push(&previous.quorums);
            }
        } else {
            quorums.push(&self.current_quorums);
        }

        QuorumsVerificationDataIterator {
            config: &self.config,
            request_id,
            quorum_sets: quorums.into_iter(),
            should_be_verifiable,
        }
    }
}

impl CoreQuorumSetV0 {
    /// New empty quorum set based on quorum configuration
    pub fn new(config: &impl QuorumLikeConfig) -> Self {
        CoreQuorumSetV0 {
            config: QuorumConfig {
                quorum_type: config.quorum_type(),
                active_signers: config.quorum_active_signers(),
                rotation: config.quorum_rotation(),
                window: config.quorum_window(),
            },
            current_quorums: Quorums::default(),
            previous: None,
        }
    }
}
