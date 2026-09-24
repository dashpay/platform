use super::ContestWindows;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::fee::fee_result::FeeResult;
use dpp::moderation_charter::charter_election_target;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_charter_election_windows_v0(
        &self,
        contested_document_resource_vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<FeeResult>, Option<ContestWindows>), Error> {
        let Some(target_contract_id) = charter_election_target(
            &contested_document_resource_vote_poll.contract.as_ref().id(),
            &contested_document_resource_vote_poll.document_type_name,
            &contested_document_resource_vote_poll.index_values,
        ) else {
            return Ok((None, None));
        };

        // Billed with the fee this read returns, never the fee a cached contract carries,
        // which depends on how the cache was filled
        let (fee_result, target) = self.get_contract_with_fetch_info_and_fee(
            target_contract_id.to_buffer(),
            Some(epoch),
            false,
            transaction,
            platform_version,
        )?;

        let windows = target.as_ref().and_then(|target| {
            target
                .contract
                .config()
                .moderation()
                .and_then(|moderation| moderation.moderators.elected())
                .map(ContestWindows::of_elected_moderators)
        });

        Ok((fee_result, windows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::setup_contract;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::moderation::{
        ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
        ModerationAbility,
    };
    use dpp::data_contract::DataContract;
    use dpp::moderation_charter::{
        ELECTED_CHARTER_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
    };
    use dpp::platform_value::{Identifier, Value};
    use std::collections::{BTreeMap, BTreeSet};

    const TARGET_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/family/family-contract.json";

    fn elected(join_window: u32, vote_window: u32) -> ContractModerationConfig {
        ContractModerationConfig {
            banlist: true,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                join_window,
                vote_window,
                challenge_cool_down: Some(1_209_600),
                election_delay: None,
                max_added_moderators: 0,
                moderated_document_types: BTreeMap::from([(
                    "person".to_string(),
                    BTreeSet::from([ModerationAbility::Ban]),
                )]),
                interim: InterimModerators::ContractOwner,
                owner_protected: false,
            })),
        }
    }

    /// A contest on the charter contract's `electedCharter` for `target`, with the charter
    /// contract resolved from `resolved_as` (only its id is read).
    fn charter_contest(
        resolved_as: &DataContract,
        document_type_name: &str,
        index_values: Vec<Value>,
    ) -> ContestedDocumentResourceVotePollWithContractInfo {
        ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(resolved_as.clone()),
            document_type_name: document_type_name.to_string(),
            index_name: "byTargetContract".to_string(),
            index_values,
        }
    }

    #[test]
    fn should_read_the_windows_of_an_elected_target_and_bill_the_read() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let target = setup_contract(
            &drive,
            TARGET_CONTRACT_PATH,
            Some([0x7A; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(
                    contract
                        .config()
                        .clone()
                        .with_moderation(Some(elected(86_400, 2_419_200))),
                );
            }),
            None,
            Some(platform_version),
        );
        let mut charters = target.clone();
        charters.set_id(MODERATION_CHARTERS_CONTRACT_ID);

        let (fee, windows) = drive
            .fetch_charter_election_windows(
                &charter_contest(
                    &charters,
                    ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                    vec![Value::Identifier(target.id().to_buffer())],
                ),
                &Epoch::new(0).expect("epoch"),
                None,
                platform_version,
            )
            .expect("expected to read the windows");

        assert_eq!(
            windows,
            Some(ContestWindows {
                join_window_ms: 86_400_000,
                poll_duration_ms: 86_400_000 + 2_419_200_000,
            })
        );
        assert!(
            fee.expect("the read of the target is billed")
                .processing_fee
                > 0
        );
    }

    #[test]
    fn should_answer_every_other_contest_without_a_read() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let target = setup_contract(
            &drive,
            TARGET_CONTRACT_PATH,
            Some([0x7B; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(
                    contract
                        .config()
                        .clone()
                        .with_moderation(Some(elected(86_400, 86_400))),
                );
            }),
            None,
            Some(platform_version),
        );
        let target_value = Value::Identifier(target.id().to_buffer());
        let mut charters = target.clone();
        charters.set_id(MODERATION_CHARTERS_CONTRACT_ID);

        // Another contract's contest keyed by the same identifier, another type of the charter
        // contract, and an electedCharter key that is not one identifier
        for contest in [
            charter_contest(
                &target,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                vec![target_value.clone()],
            ),
            charter_contest(&charters, "submittedCharter", vec![target_value.clone()]),
            charter_contest(
                &charters,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                vec![target_value.clone(), target_value.clone()],
            ),
            charter_contest(
                &charters,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                vec![Value::U64(7)],
            ),
        ] {
            assert_eq!(
                drive
                    .fetch_charter_election_windows(
                        &contest,
                        &Epoch::new(0).expect("epoch"),
                        None,
                        platform_version,
                    )
                    .expect("expected an answer"),
                (None, None)
            );
        }
    }

    #[test]
    fn should_leave_a_missing_or_unelected_target_on_the_generic_windows() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let unmoderated = setup_contract(
            &drive,
            TARGET_CONTRACT_PATH,
            Some([0x7C; 32]),
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        let mut charters = unmoderated.clone();
        charters.set_id(MODERATION_CHARTERS_CONTRACT_ID);

        for target_id in [unmoderated.id(), Identifier::new([0x7D; 32])] {
            let (fee, windows) = drive
                .fetch_charter_election_windows(
                    &charter_contest(
                        &charters,
                        ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                        vec![Value::Identifier(target_id.to_buffer())],
                    ),
                    &Epoch::new(0).expect("epoch"),
                    None,
                    platform_version,
                )
                .expect("a missing or unelected target is not an error");
            assert_eq!(windows, None);
            assert!(fee.is_some(), "the read is billed either way");
        }
    }

    #[test]
    fn should_read_nothing_before_protocol_version_14() {
        let drive = setup_drive_with_initial_state_structure(None);
        let target = setup_contract(
            &drive,
            TARGET_CONTRACT_PATH,
            Some([0x7E; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(
                    contract
                        .config()
                        .clone()
                        .with_moderation(Some(elected(86_400, 86_400))),
                );
            }),
            None,
            None,
        );
        let mut charters = target.clone();
        charters.set_id(MODERATION_CHARTERS_CONTRACT_ID);
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");

        assert_eq!(
            drive
                .fetch_charter_election_windows(
                    &charter_contest(
                        &charters,
                        ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                        vec![Value::Identifier(target.id().to_buffer())],
                    ),
                    &Epoch::new(0).expect("epoch"),
                    None,
                    platform_version,
                )
                .expect("expected an answer"),
            (None, None)
        );
    }
}
