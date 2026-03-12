use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{IdentityOperation, PrefundedSpecializedBalanceOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use crate::util::batch::drive_op_batch::PrefundedSpecializedBalanceOperationType;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

impl DriveHighLevelOperationConverter for MasternodeVoteTransitionAction {
    fn into_high_level_drive_operations<'a>(
        mut self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .masternode_vote_transition
        {
            0 => {
                let voter_identity_id = self.voter_identity_id();
                let pro_tx_hash = self.pro_tx_hash();
                let nonce = self.nonce();
                let strength = self.vote_strength();
                let previous_resource_vote_choice_to_remove =
                    self.take_previous_resource_vote_choice_to_remove();
                let vote = self.vote_owned();
                let prefunded_specialized_balance_id = vote.specialized_balance_id()?.ok_or(Error::Protocol(Box::new(ProtocolError::VoteError("vote does not have a specialized balance from where it can use to pay for processing (this should have been caught during validation)".to_string()))))?;

                let drive_operations = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: voter_identity_id.into_buffer(),
                        nonce,
                    }),
                    IdentityOperation(IdentityOperationType::MasternodeCastVote {
                        // Votes are cast based on masternode pro_tx_hash, and not the voter identity id
                        voter_pro_tx_hash: pro_tx_hash.to_buffer(),
                        strength,
                        vote,
                        previous_resource_vote_choice_to_remove,
                    }),
                    // Casting a vote has a fixed cost
                    PrefundedSpecializedBalanceOperation(
                        PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                            prefunded_specialized_balance_id,
                            remove_balance: platform_version
                                .fee_version
                                .vote_resolution_fund_fees
                                .contested_document_single_vote_cost,
                        },
                    ),
                ];
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "MasternodeVoteTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
    use crate::drive::votes::resolved::votes::resolved_resource_vote::v0::ResolvedResourceVoteV0;
    use crate::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
    use crate::drive::votes::resolved::votes::ResolvedVote;
    use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
    use crate::util::batch::drive_op_batch::PrefundedSpecializedBalanceOperationType;
    use crate::util::batch::DriveOperation;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

    fn make_resolved_vote() -> ResolvedVote {
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = dpns.data_contract_owned();
        let contract_info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(data_contract),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![],
        };
        ResolvedVote::ResolvedResourceVote(ResolvedResourceVote::V0(ResolvedResourceVoteV0 {
            resolved_vote_poll: ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                contract_info,
            ),
            resource_vote_choice: ResourceVoteChoice::Abstain,
        }))
    }

    fn make_action() -> MasternodeVoteTransitionAction {
        MasternodeVoteTransitionAction::V0(MasternodeVoteTransitionActionV0 {
            pro_tx_hash: Identifier::from([0xAA; 32]),
            voter_identity_id: Identifier::from([0xBB; 32]),
            voting_address: [0xCC; 20],
            vote_strength: 4,
            vote: make_resolved_vote(),
            previous_resource_vote_choice_to_remove: Some((ResourceVoteChoice::Lock, 2)),
            nonce: 77,
        })
    }

    #[test]
    fn test_produces_three_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // UpdateIdentityNonce + MasternodeCastVote + DeductFromPrefundedBalance
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_update_identity_nonce() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xBB; 32]); // voter_identity_id
                assert_eq!(*nonce, 77);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_is_masternode_cast_vote() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            DriveOperation::IdentityOperation(IdentityOperationType::MasternodeCastVote {
                voter_pro_tx_hash,
                strength,
                vote,
                previous_resource_vote_choice_to_remove,
            }) => {
                assert_eq!(*voter_pro_tx_hash, [0xAA; 32]); // pro_tx_hash
                assert_eq!(*strength, 4);
                assert!(previous_resource_vote_choice_to_remove.is_some());
                let (choice, count) = previous_resource_vote_choice_to_remove.as_ref().unwrap();
                assert_eq!(*choice, ResourceVoteChoice::Lock);
                assert_eq!(*count, 2);

                // Verify the vote contents: should be Abstain choice
                match vote {
                    ResolvedVote::ResolvedResourceVote(resolved) => match resolved {
                        ResolvedResourceVote::V0(v0) => {
                            assert_eq!(v0.resource_vote_choice, ResourceVoteChoice::Abstain);
                        }
                    },
                }
            }
            other => panic!("expected MasternodeCastVote, got {:?}", other),
        }
    }

    #[test]
    fn test_third_op_is_deduct_from_prefunded_balance() {
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        // Build the vote and compute its expected balance ID before it is consumed
        let vote = make_resolved_vote();
        let expected_balance_id = vote
            .specialized_balance_id()
            .expect("expected specialized balance id")
            .expect("expected Some balance id");

        let action = MasternodeVoteTransitionAction::V0(MasternodeVoteTransitionActionV0 {
            pro_tx_hash: Identifier::from([0xAA; 32]),
            voter_identity_id: Identifier::from([0xBB; 32]),
            voting_address: [0xCC; 20],
            vote_strength: 4,
            vote,
            previous_resource_vote_choice_to_remove: Some((ResourceVoteChoice::Lock, 2)),
            nonce: 77,
        });

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[2] {
            DriveOperation::PrefundedSpecializedBalanceOperation(
                PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                    prefunded_specialized_balance_id,
                    remove_balance,
                },
            ) => {
                assert_eq!(
                    *prefunded_specialized_balance_id,
                    expected_balance_id,
                    "prefunded_specialized_balance_id should match the vote poll's specialized balance id"
                );
                assert_eq!(
                    *remove_balance,
                    platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_single_vote_cost
                );
            }
            other => panic!("expected DeductFromPrefundedBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_no_previous_vote_choice() {
        let v0 = MasternodeVoteTransitionActionV0 {
            pro_tx_hash: Identifier::from([0xAA; 32]),
            voter_identity_id: Identifier::from([0xBB; 32]),
            voting_address: [0xCC; 20],
            vote_strength: 1,
            vote: make_resolved_vote(),
            previous_resource_vote_choice_to_remove: None,
            nonce: 5,
        };
        let action = MasternodeVoteTransitionAction::V0(v0);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should still produce 3 ops
        assert_eq!(ops.len(), 3);

        // Verify nonce op uses the correct voter_identity_id and nonce
        match &ops[0] {
            DriveOperation::IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xBB; 32]); // voter_identity_id
                assert_eq!(*nonce, 5);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }

        match &ops[1] {
            DriveOperation::IdentityOperation(IdentityOperationType::MasternodeCastVote {
                voter_pro_tx_hash,
                strength,
                vote,
                previous_resource_vote_choice_to_remove,
            }) => {
                assert_eq!(*voter_pro_tx_hash, [0xAA; 32]); // pro_tx_hash
                assert_eq!(*strength, 1);
                assert!(previous_resource_vote_choice_to_remove.is_none());

                // Verify vote choice is Abstain (from make_resolved_vote)
                match vote {
                    ResolvedVote::ResolvedResourceVote(resolved) => match resolved {
                        ResolvedResourceVote::V0(v0) => {
                            assert_eq!(v0.resource_vote_choice, ResourceVoteChoice::Abstain);
                        }
                    },
                }
            }
            other => panic!("expected MasternodeCastVote, got {:?}", other),
        }
    }
}
