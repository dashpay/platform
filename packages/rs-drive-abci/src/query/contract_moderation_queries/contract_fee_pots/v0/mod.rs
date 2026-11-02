use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::identifier_from_request;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_fee_pots_request::GetContractFeePotsRequestV0;
use dapi_grpc::platform::v0::get_contract_fee_pots_response::{
    get_contract_fee_pots_response_v0, ContractFeePot as ContractFeePotProto,
    ContractFeePotLastClaim as ContractFeePotLastClaimProto,
    ContractFeePots as ContractFeePotsProto, GetContractFeePotsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::fee_pots::types::ContractFeePotState;
use drive::util::grove_operations::GroveDBToUse;

/// Both pots, in the order every reader of the proof names them
const BOTH_POTS: [ContractFeePot; 2] = [ContractFeePot::Owner, ContractFeePot::Moderators];

impl<C> Platform<C> {
    /// Returns the two fee pots of a contract: what each holds and its last claim, which is
    /// the epoch and the block time it was last paid out in and the identity that claimed. A
    /// pot that never received a fee holds nothing, and one that was never paid out has no
    /// last claim. The proved form proves both pots and both last claims, present or absent.
    ///
    /// The contract has to exist: its last claims live under it, and a proof of them
    /// under a contract that is not there would prove nothing a client can use.
    pub(super) fn query_contract_fee_pots_v0(
        &self,
        GetContractFeePotsRequestV0 { contract_id, prove }: GetContractFeePotsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractFeePotsResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));

        if self
            .drive
            .get_contract_with_fetch_info(contract_id.to_buffer(), false, None, platform_version)?
            .is_none()
        {
            return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                format!("contract {} not found", contract_id),
            )));
        }

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_contract_fee_pots(
                contract_id,
                &BOTH_POTS,
                None,
                platform_version
            ));

            GetContractFeePotsResponseV0 {
                result: Some(get_contract_fee_pots_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let owner = check_validation_result_with_data!(self.drive.fetch_contract_fee_pot(
                contract_id,
                ContractFeePot::Owner,
                None,
                platform_version
            ));
            let moderators = check_validation_result_with_data!(self.drive.fetch_contract_fee_pot(
                contract_id,
                ContractFeePot::Moderators,
                None,
                platform_version
            ));

            GetContractFeePotsResponseV0 {
                result: Some(get_contract_fee_pots_response_v0::Result::Pots(
                    ContractFeePotsProto {
                        owner: Some(pot_to_response(owner)),
                        moderators: Some(pot_to_response(moderators)),
                    },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

fn pot_to_response(pot: ContractFeePotState) -> ContractFeePotProto {
    ContractFeePotProto {
        credits: pot.credits,
        last_claim: pot
            .last_claim
            .map(|last_claim| ContractFeePotLastClaimProto {
                epoch: u32::from(last_claim.epoch_index),
                time_ms: last_claim.time_ms,
                claimant_id: last_claim.claimant_id.to_vec(),
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::contract_moderation_queries::tests::store_contract;
    use crate::query::tests::setup_platform;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::action_fees::ContractFeePotLastClaim;
    use dpp::identifier::Identifier;
    use drive::drive::contract::fee_pots::types::ContractFeePots;
    use drive::drive::Drive;
    use drive::util::batch::drive_op_batch::ContractFeePotOperationType;
    use drive::util::batch::DriveOperation;

    fn request(contract_id: Vec<u8>, prove: bool) -> GetContractFeePotsRequestV0 {
        GetContractFeePotsRequestV0 { contract_id, prove }
    }

    fn apply(
        platform: &TempPlatform<MockCoreRPCLike>,
        operations: Vec<ContractFeePotOperationType>,
        platform_version: &PlatformVersion,
    ) {
        platform
            .drive
            .apply_drive_operations(
                operations
                    .into_iter()
                    .map(DriveOperation::ContractFeePotOperation)
                    .collect(),
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to write the fee pots");
    }

    fn pots_of(
        result: QueryValidationResult<GetContractFeePotsResponseV0>,
    ) -> ContractFeePotsProto {
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        match result.data.expect("expected a response").result {
            Some(get_contract_fee_pots_response_v0::Result::Pots(pots)) => pots,
            other => panic!("expected the pots, got {other:?}"),
        }
    }

    #[test]
    fn should_refuse_a_malformed_contract_id_and_an_unknown_contract() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_fee_pots_v0(request(vec![0; 8], false), &state, version)
            .expect("expected query to succeed");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(message)] if message.contains("contract_id")
            ),
            "{:?}",
            result.errors
        );

        for prove in [false, true] {
            let result = platform
                .query_contract_fee_pots_v0(request(vec![9; 32], prove), &state, version)
                .expect("expected query to succeed");
            assert!(
                matches!(result.errors.as_slice(), [QueryError::NotFound(_)]),
                "{:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_return_empty_pots_that_were_never_claimed_before_the_first_fee() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        // Any contract has pots, moderated or not: the owner pot needs no moderation.
        let contract = store_contract(&platform, false, false, version);

        let pots = pots_of(
            platform
                .query_contract_fee_pots_v0(request(contract.id().to_vec(), false), &state, version)
                .expect("expected query to succeed"),
        );

        let empty = ContractFeePotProto {
            credits: 0,
            last_claim: None,
        };
        assert_eq!(pots.owner, Some(empty.clone()));
        assert_eq!(pots.moderators, Some(empty));
    }

    #[test]
    fn should_return_each_pot_with_the_epoch_it_was_last_claimed_in() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract(&platform, true, false, version);
        apply(
            &platform,
            vec![
                ContractFeePotOperationType::AddToPot {
                    contract_id: contract.id(),
                    pot: ContractFeePot::Owner,
                    amount: 700,
                },
                ContractFeePotOperationType::AddToPot {
                    contract_id: contract.id(),
                    pot: ContractFeePot::Moderators,
                    amount: 300,
                },
                // Epoch 0 is an epoch like any other: it must not read as "never claimed".
                ContractFeePotOperationType::SetLastClaim {
                    contract_id: contract.id(),
                    pot: ContractFeePot::Moderators,
                    last_claim: ContractFeePotLastClaim {
                        epoch_index: 0,
                        time_ms: 1_700_000_000_000,
                        claimant_id: Identifier::from([9; 32]),
                    },
                },
            ],
            version,
        );

        let pots = pots_of(
            platform
                .query_contract_fee_pots_v0(request(contract.id().to_vec(), false), &state, version)
                .expect("expected query to succeed"),
        );

        assert_eq!(
            pots.owner,
            Some(ContractFeePotProto {
                credits: 700,
                last_claim: None,
            })
        );
        assert_eq!(
            pots.moderators,
            Some(ContractFeePotProto {
                credits: 300,
                last_claim: Some(ContractFeePotLastClaimProto {
                    epoch: 0,
                    time_ms: 1_700_000_000_000,
                    claimant_id: vec![9; 32],
                }),
            })
        );
    }

    #[test]
    fn should_prove_what_the_unproved_form_returns() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract(&platform, true, false, version);
        apply(
            &platform,
            vec![
                ContractFeePotOperationType::AddToPot {
                    contract_id: contract.id(),
                    pot: ContractFeePot::Owner,
                    amount: 42,
                },
                ContractFeePotOperationType::SetLastClaim {
                    contract_id: contract.id(),
                    pot: ContractFeePot::Owner,
                    last_claim: ContractFeePotLastClaim {
                        epoch_index: 6,
                        time_ms: 1_700_000_006_000,
                        claimant_id: contract.owner_id(),
                    },
                },
            ],
            version,
        );

        let result = platform
            .query_contract_fee_pots_v0(request(contract.id().to_vec(), true), &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let Some(get_contract_fee_pots_response_v0::Result::Proof(proof)) =
            result.data.expect("expected a response").result
        else {
            panic!("expected a proof");
        };

        let (_, proved): (_, ContractFeePots) = Drive::verify_contract_fee_pots(
            &proof.grovedb_proof,
            Identifier::from(contract.id()),
            &BOTH_POTS,
            false,
            version,
        )
        .expect("expected the proof to verify");
        assert_eq!(proved.owner.credits, 42);
        assert_eq!(
            proved.owner.last_claim,
            Some(ContractFeePotLastClaim {
                epoch_index: 6,
                time_ms: 1_700_000_006_000,
                claimant_id: contract.owner_id(),
            })
        );
        assert_eq!(proved.moderators.credits, 0);
        assert_eq!(proved.moderators.last_claim, None);
    }
}
