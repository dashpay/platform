use crate::consensus::basic::document::{
    DuplicatePrivateDocumentEntryError, InvalidPrivateDocumentEntryCountError,
    InvalidPrivateDocumentEntrySizeError,
};
use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl StateTransitionStructureValidation for PrivateDocumentCreateFromShieldedPoolTransitionV0 {
    /// Checks what needs no state: from 1 to `max_private_document_entries_per_transition`
    /// entries, each as long as the first and from 1 to `max_private_document_entry_size`
    /// bytes, no two the same, then the bundle's actions, value balance, proof and anchor. Whether the entries
    /// take the type's `entrySize` needs the contract, so the transform checks it.
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let limits = &platform_version.system_limits;
        // Both limits are `None` before protocol version 15, where the transition is refused
        // before its structure is validated; refusing every entry there keeps it so.
        let max_count = limits
            .max_private_document_entries_per_transition
            .unwrap_or(0);
        let max_size = limits.max_private_document_entry_size.unwrap_or(0);

        if self.entries.is_empty() || self.entries.len() > max_count as usize {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidPrivateDocumentEntryCountError::new(self.entries.len() as u64, max_count)
                    .into(),
            );
        }

        let first_entry_size = self.entries[0].len();
        for (entry_index, entry) in self.entries.iter().enumerate() {
            let size = entry.len();
            if size == 0 || size != first_entry_size || size > max_size as usize {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidPrivateDocumentEntrySizeError::new(
                        // At most max_count (a u16) entries reach this loop.
                        entry_index as u16,
                        size.min(u32::MAX as usize) as u32,
                        first_entry_size.min(u32::MAX as usize) as u32,
                        max_size,
                    )
                    .into(),
                );
            }
        }

        let mut first_index_of_entry = BTreeMap::new();
        for (entry_index, entry) in self.entries.iter().enumerate() {
            if let Some(first_index) =
                first_index_of_entry.insert(entry.as_slice(), entry_index as u16)
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DuplicatePrivateDocumentEntryError::new(first_index, entry_index as u16).into(),
                );
            }
        }

        let result = validate_actions_count(&self.actions, limits.max_shielded_transition_actions);
        if !result.is_valid() {
            return result;
        }

        let result = validate_encrypted_note_sizes(&self.actions);
        if !result.is_valid() {
            return result;
        }

        if self.value_balance == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "private document create value balance must be greater than zero"
                            .to_string(),
                    ),
                )
                .into(),
            );
        }

        if self.value_balance > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "private document create value balance exceeds maximum allowed value"
                            .to_string(),
                    ),
                )
                .into(),
            );
        }

        let result = validate_proof_not_empty(&self.proof);
        if !result.is_valid() {
            return result;
        }

        validate_anchor_not_zero(&self.anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::shielded::SerializedAction;
    use assert_matches::assert_matches;
    use platform_value::{BinaryData, Identifier};

    fn action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn valid() -> PrivateDocumentCreateFromShieldedPoolTransitionV0 {
        PrivateDocumentCreateFromShieldedPoolTransitionV0 {
            data_contract_id: Identifier::from([1u8; 32]),
            document_type_name: "privateContactRequest".to_string(),
            entries: vec![
                BinaryData::new(vec![7u8; 585]),
                BinaryData::new(vec![8u8; 585]),
            ],
            actions: vec![action()],
            value_balance: 1_000,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
        }
    }

    fn entry_count_error(
        t: &PrivateDocumentCreateFromShieldedPoolTransitionV0,
    ) -> Option<(u64, u16)> {
        match t
            .validate_structure(PlatformVersion::latest())
            .errors
            .as_slice()
        {
            [ConsensusError::BasicError(BasicError::InvalidPrivateDocumentEntryCountError(e))] => {
                Some((e.count(), e.max_count()))
            }
            _ => None,
        }
    }

    fn entry_size_error(
        t: &PrivateDocumentCreateFromShieldedPoolTransitionV0,
    ) -> Option<(u16, u32)> {
        match t
            .validate_structure(PlatformVersion::latest())
            .errors
            .as_slice()
        {
            [ConsensusError::BasicError(BasicError::InvalidPrivateDocumentEntrySizeError(e))] => {
                Some((e.entry_index(), e.size()))
            }
            _ => None,
        }
    }

    #[test]
    fn should_accept_a_well_formed_transition() {
        let result = valid().validate_structure(PlatformVersion::latest());
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_accept_the_most_entries_of_the_largest_size() {
        let limits = &PlatformVersion::latest().system_limits;
        let mut t = valid();
        t.entries = (0..limits.max_private_document_entries_per_transition.unwrap())
            .map(|i| {
                BinaryData::new(vec![
                    i as u8;
                    limits.max_private_document_entry_size.unwrap() as usize
                ])
            })
            .collect();
        let result = t.validate_structure(PlatformVersion::latest());
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_refuse_no_entry_and_one_entry_too_many() {
        let max = PlatformVersion::latest()
            .system_limits
            .max_private_document_entries_per_transition
            .unwrap();
        let mut t = valid();
        t.entries = vec![];
        assert_eq!(entry_count_error(&t), Some((0, max)));
        t.entries = (0..=max)
            .map(|i| BinaryData::new(vec![i as u8; 585]))
            .collect();
        assert_eq!(entry_count_error(&t), Some((max as u64 + 1, max)));
    }

    #[test]
    fn should_refuse_an_empty_entry_an_entry_over_the_cap_and_entries_of_two_sizes() {
        let max_size = PlatformVersion::latest()
            .system_limits
            .max_private_document_entry_size
            .unwrap() as usize;
        let mut t = valid();
        t.entries = vec![BinaryData::new(vec![])];
        assert_eq!(entry_size_error(&t), Some((0, 0)));
        t.entries = vec![BinaryData::new(vec![1u8; max_size + 1])];
        assert_eq!(entry_size_error(&t), Some((0, max_size as u32 + 1)));
        t.entries = vec![
            BinaryData::new(vec![1u8; 585]),
            BinaryData::new(vec![2u8; 585]),
            BinaryData::new(vec![1u8; 584]),
        ];
        assert_eq!(entry_size_error(&t), Some((2, 584)));
    }

    #[test]
    fn should_refuse_two_entries_that_are_the_same_bytes() {
        let mut t = valid();
        t.entries = vec![
            BinaryData::new(vec![1u8; 585]),
            BinaryData::new(vec![2u8; 585]),
            BinaryData::new(vec![1u8; 585]),
        ];
        match t
            .validate_structure(PlatformVersion::latest())
            .errors
            .as_slice()
        {
            [ConsensusError::BasicError(BasicError::DuplicatePrivateDocumentEntryError(e))] => {
                assert_eq!((e.first_index(), e.entry_index()), (0, 2));
            }
            errors => panic!("expected a duplicate entry error, got {errors:?}"),
        }
    }

    #[test]
    fn should_refuse_every_entry_before_protocol_version_15() {
        let t = valid();
        let result = t.validate_structure(PlatformVersion::get(14).expect("protocol version 14"));
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidPrivateDocumentEntryCountError(_)
            )]
        );
    }

    #[test]
    fn should_refuse_no_actions_a_zero_or_oversized_value_balance_an_empty_proof_and_a_zero_anchor()
    {
        let mut t = valid();
        t.actions = vec![];
        assert_matches!(
            t.validate_structure(PlatformVersion::latest())
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
        for value_balance in [0, i64::MAX as u64 + 1] {
            let mut t = valid();
            t.value_balance = value_balance;
            assert_matches!(
                t.validate_structure(PlatformVersion::latest())
                    .errors
                    .as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ShieldedInvalidValueBalanceError(_)
                )]
            );
        }
        let mut t = valid();
        t.proof = vec![];
        assert_matches!(
            t.validate_structure(PlatformVersion::latest())
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
        let mut t = valid();
        t.anchor = [0u8; 32];
        assert_matches!(
            t.validate_structure(PlatformVersion::latest())
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }
}
