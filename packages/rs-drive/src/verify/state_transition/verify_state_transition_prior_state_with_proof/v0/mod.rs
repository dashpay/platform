use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

impl Drive {
    /// Credit transfers, top-ups and credit withdrawals of an identity.
    pub(super) fn state_transition_prior_state_is_readable_v0(
        state_transition: &StateTransition,
    ) -> bool {
        matches!(
            state_transition,
            StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityTopUp(_)
                | StateTransition::IdentityCreditWithdrawal(_)
        )
    }

    pub(super) fn verify_state_transition_prior_state_with_proof_v0(
        state_transition: &StateTransition,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(RootHash, StateTransitionProofResult)>, Error> {
        let partial_identity = |id, balance, revision| PartialIdentity {
            id,
            loaded_public_keys: Default::default(),
            balance,
            revision,
            not_found_public_keys: Default::default(),
        };

        match state_transition {
            StateTransition::IdentityCreditTransfer(transfer) => {
                // The same two subsets the proof of the state after the transfer holds
                let (root_hash_sender, sender_balance) =
                    Drive::verify_identity_balance_for_identity_id(
                        proof,
                        transfer.identity_id().into_buffer(),
                        true,
                        platform_version,
                    )?;
                let (root_hash_recipient, recipient_balance) =
                    Drive::verify_identity_balance_for_identity_id(
                        proof,
                        transfer.recipient_id().into_buffer(),
                        true,
                        platform_version,
                    )?;
                if root_hash_sender != root_hash_recipient {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "proof is expected to have same root hash for all subsets (identity transfer)"
                            .to_string(),
                    )));
                }
                Ok(Some((
                    root_hash_sender,
                    StateTransitionProofResult::VerifiedBalanceTransfer(
                        partial_identity(transfer.identity_id(), sender_balance, None),
                        partial_identity(transfer.recipient_id(), recipient_balance, None),
                    ),
                )))
            }
            StateTransition::IdentityTopUp(top_up) => {
                let (root_hash, balance_and_revision) =
                    Drive::verify_identity_balance_and_revision_for_identity_id(
                        proof,
                        top_up.identity_id().into_buffer(),
                        false,
                        platform_version,
                    )?;
                Ok(Some((
                    root_hash,
                    StateTransitionProofResult::VerifiedPartialIdentity(partial_identity(
                        *top_up.identity_id(),
                        balance_and_revision.map(|(balance, _)| balance),
                        balance_and_revision.map(|(_, revision)| revision),
                    )),
                )))
            }
            StateTransition::IdentityCreditWithdrawal(withdrawal) => {
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    proof,
                    withdrawal.identity_id().into_buffer(),
                    false,
                    platform_version,
                )?;
                Ok(Some((
                    root_hash,
                    StateTransitionProofResult::VerifiedPartialIdentity(partial_identity(
                        withdrawal.identity_id(),
                        balance,
                        None,
                    )),
                )))
            }
            // Every other kind is checked against what it leaves, which the state before it
            // does not hold, or proves keys whose number has no bound
            _ => Ok(None),
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::data_contract_update_transition::{
        DataContractUpdateTransition, DataContractUpdateTransitionV0,
    };
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::TryIntoPlatformVersioned;

    fn contract_update() -> StateTransition {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let data_contract: DataContractInSerializationFormat = data_contract
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize the contract");
        DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
            identity_contract_nonce: 1,
            data_contract,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        })
        .into()
    }

    #[test]
    fn should_read_the_prior_state_of_identity_credit_moves_only() {
        let readable: [StateTransition; 3] = [
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0::default())
                .into(),
            IdentityTopUpTransition::V0(IdentityTopUpTransitionV0::default()).into(),
            IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1::default())
                .into(),
        ];
        for state_transition in &readable {
            assert!(Drive::state_transition_prior_state_is_readable_v0(
                state_transition
            ));
        }

        // A contract update's reader compares the proved contract with the updated one, and an
        // identity update proves the whole key set
        let not_readable: [StateTransition; 2] = [
            contract_update(),
            IdentityUpdateTransition::V0(IdentityUpdateTransitionV0::default()).into(),
        ];
        for state_transition in &not_readable {
            assert!(!Drive::state_transition_prior_state_is_readable_v0(
                state_transition
            ));
            // Answered without reading the proof
            assert!(Drive::verify_state_transition_prior_state_with_proof_v0(
                state_transition,
                &[],
                PlatformVersion::latest(),
            )
            .expect("expected no error")
            .is_none());
        }
    }

    #[test]
    fn should_read_a_balance_that_does_not_exist_yet_as_absent() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        // The recipient is created in the same block as the transfer to it, so the block
        // before holds only the sender
        let sender = Identity::random_identity(3, Some(1), platform_version).unwrap();
        let recipient = Identity::random_identity(3, Some(2), platform_version).unwrap();
        drive
            .add_new_identity(
                sender.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .unwrap();

        let transfer: StateTransition =
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
                identity_id: sender.id(),
                recipient_id: recipient.id(),
                amount: 500_000,
                nonce: 1,
                ..Default::default()
            })
            .into();
        let proof = drive
            .prove_state_transition(&transfer, None, platform_version)
            .unwrap()
            .into_data()
            .unwrap();

        let (root_hash, state) = Drive::verify_state_transition_prior_state_with_proof_v0(
            &transfer,
            &proof,
            platform_version,
        )
        .expect("expected the proof to verify")
        .expect("expected a transfer to be read");

        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap()
        );
        let StateTransitionProofResult::VerifiedBalanceTransfer(sender_state, recipient_state) =
            state
        else {
            panic!("expected the balances of a transfer");
        };
        assert_eq!(sender_state.balance, Some(sender.balance()));
        assert_eq!(recipient_state.balance, None);
    }
}
