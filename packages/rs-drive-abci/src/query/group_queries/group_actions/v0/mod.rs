use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_group_actions_request::GetGroupActionsRequestV0;
use dapi_grpc::platform::v0::get_group_actions_response::get_group_actions_response_v0::{emergency_action_event, group_action_event, token_event, BurnEvent, DestroyFrozenFundsEvent, EmergencyActionEvent, FreezeEvent, GroupActionEntry, GroupActionEvent, GroupActions, MintEvent, TokenConfigUpdateEvent, TokenEvent as TokenEventResponse, UnfreezeEvent, UpdateDirectPurchasePriceEvent};
use dapi_grpc::platform::v0::get_group_actions_response::{
    get_group_actions_response_v0, GetGroupActionsResponseV0,
};
use dapi_grpc::platform::v0::get_group_actions_response::get_group_actions_response_v0::update_direct_purchase_price_event::{Price, PriceForQuantity, PricingSchedule};
use dpp::check_validation_result_with_data;
use dpp::data_contract::GroupContractPosition;
use dpp::group::action_event;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_group_actions_v0(
        &self,
        GetGroupActionsRequestV0 {
            contract_id,
            group_contract_position,
            status,
            start_at_action_id,
            count,
            prove,
        }: GetGroupActionsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetGroupActionsResponseV0>, Error> {
        let config = &self.config.drive;
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        if group_contract_position > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidParameter(format!(
                    "group contract position {} can not be over u16::MAX",
                    group_contract_position
                )),
            )));
        }

        let limit = count
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.default_query_limit
                {
                    None
                } else {
                    Some(limit_value as u16)
                }
            })
            .ok_or(drive::error::Error::Query(QuerySyntaxError::InvalidLimit(
                format!("limit greater than max limit {}", config.max_query_limit),
            )))?;

        let maybe_start_at_action_id = match start_at_action_id {
            None => None,
            Some(start_at_action_id) => {
                let start_at_action_id_identifier: Identifier =
                    check_validation_result_with_data!(start_at_action_id
                        .start_action_id
                        .try_into()
                        .map_err(|_| {
                            QueryError::InvalidArgument(
                                "start at action id must be a valid identifier (32 bytes long)"
                                    .to_string(),
                            )
                        }));
                Some((
                    start_at_action_id_identifier,
                    start_at_action_id.start_action_id_included,
                ))
            }
        };

        let group_status: GroupActionStatus =
            check_validation_result_with_data!(status.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "group action status must be Active or Closed".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_action_infos(
                contract_id,
                group_contract_position as GroupContractPosition,
                group_status,
                maybe_start_at_action_id,
                Some(limit),
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let group_actions = self
                .drive
                .fetch_action_infos(
                    contract_id,
                    group_contract_position as GroupContractPosition,
                    group_status,
                    maybe_start_at_action_id,
                    Some(limit),
                    None,
                    platform_version,
                )?
                .into_iter()
                .filter_map(|(action_id, group_action)| {
                    // Convert the fetched GroupAction into a GroupActionEntry
                    Some(GroupActionEntry {
                        action_id: action_id.to_vec(),
                        event: Some(GroupActionEvent {
                            event_type: Some(match group_action {
                                GroupAction::V0(group_action_v0) => match group_action_v0.event {
                                    action_event::GroupActionEvent::TokenEvent(token_event) => match token_event {
                                        TokenEvent::Mint(amount, recipient_id, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::Mint(MintEvent {
                                                    amount,
                                                    recipient_id: recipient_id.to_vec(),
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::Burn(amount, burn_from_id, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::Burn(BurnEvent {
                                                    amount,
                                                    burn_from_id: burn_from_id.to_vec(),
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::Freeze(frozen_id, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::Freeze(FreezeEvent {
                                                    frozen_id: frozen_id.to_vec(),
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::Unfreeze(frozen_id, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::Unfreeze(UnfreezeEvent {
                                                    frozen_id: frozen_id.to_vec(),
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::DestroyFrozenFunds(frozen_id, amount, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::DestroyFrozenFunds(
                                                    DestroyFrozenFundsEvent {
                                                        frozen_id: frozen_id.to_vec(),
                                                        amount,
                                                        public_note,
                                                    },
                                                )),
                                            })
                                        }
                                        TokenEvent::EmergencyAction(action, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::EmergencyAction(EmergencyActionEvent {
                                                    action_type: match action {
                                                        TokenEmergencyAction::Pause => emergency_action_event::ActionType::Pause.into(),
                                                        TokenEmergencyAction::Resume => emergency_action_event::ActionType::Resume.into(),
                                                    },
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::ConfigUpdate(token_configuration_change_item, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::TokenConfigUpdate(TokenConfigUpdateEvent {
                                                    token_config_update_item: token_configuration_change_item.serialize_consume_to_bytes_with_platform_version(platform_version).ok()?,
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::ChangePriceForDirectPurchase(pricing_schedule, public_note) => {
                                            group_action_event::EventType::TokenEvent(TokenEventResponse {
                                                r#type: Some(token_event::Type::UpdatePrice(UpdateDirectPurchasePriceEvent {
                                                    price: pricing_schedule.map(|pricing_schedule| {
                                                        match pricing_schedule {
                                                            TokenPricingSchedule::SinglePrice(price) => {
                                                                Price::FixedPrice(price)
                                                            }
                                                            TokenPricingSchedule::SetPrices(prices) => {
                                                                let schedule = PricingSchedule {
                                                                    price_for_quantity: prices
                                                                        .into_iter()
                                                                        .map(|(quantity, price)| PriceForQuantity { quantity, price })
                                                                        .collect(),
                                                                };
                                                                Price::VariablePrice(schedule)
                                                            }
                                                        }
                                                    }),
                                                    public_note,
                                                })),
                                            })
                                        }
                                        TokenEvent::Transfer(..) | TokenEvent::DirectPurchase(..) | TokenEvent::Claim(..) => {
                                            return None;
                                        },
                                    },
                                },
                            }),
                        }),
                    })
                })
                .collect();
            GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::GroupActions(
                    GroupActions { group_actions },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dapi_grpc::platform::v0::get_group_actions_request::StartAtActionId;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::group::action_event::GroupActionEvent as DppGroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::identifier::Identifier;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use dpp::tokens::token_event::TokenEvent;
    use std::collections::BTreeMap;

    #[test]
    fn test_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 8],
            group_contract_position: 0,
            status: 0, // Active
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_when_prove_is_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 8],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_group_contract_position_exceeds_u16_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32 + 1,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }

    #[test]
    fn test_invalid_limit_zero() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: Some(0),
            prove: false,
        };

        let result = platform.query_group_actions_v0(request, &state, version);

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Drive(drive::error::Error::Query(
                    QuerySyntaxError::InvalidLimit(_)
                )))
            ),
            "expected InvalidLimit error for zero count"
        );
    }

    #[test]
    fn test_invalid_limit_exceeds_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: Some(u16::MAX as u32 + 1),
            prove: false,
        };

        let result = platform.query_group_actions_v0(request, &state, version);

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Drive(drive::error::Error::Query(
                    QuerySyntaxError::InvalidLimit(_)
                )))
            ),
            "expected InvalidLimit error for count exceeding max"
        );
    }

    #[test]
    fn test_invalid_start_action_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: Some(StartAtActionId {
                start_action_id: vec![0; 8], // invalid: must be 32 bytes
                start_action_id_included: true,
            }),
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("start at action id must be a valid identifier")
        ));
    }

    #[test]
    fn test_invalid_status() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 99, // invalid status
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("group action status must be Active or Closed")
        ));
    }

    #[test]
    fn test_query_group_actions_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0, // Active
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::GroupActions(
                    GroupActions { group_actions }
                )),
                metadata: Some(_),
            }) if group_actions.is_empty()
        ));
    }

    #[test]
    fn test_query_group_actions_with_prove_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_actions_with_closed_status() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 1, // Closed
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::GroupActions(
                    GroupActions { group_actions }
                )),
                metadata: Some(_),
            }) if group_actions.is_empty()
        ));
    }

    #[test]
    fn test_query_group_actions_with_valid_start_at_action_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: Some(StartAtActionId {
                start_action_id: vec![0; 32],
                start_action_id_included: true,
            }),
            count: Some(10),
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::GroupActions(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_empty_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_query_group_actions_with_valid_count() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionsRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: Some(5),
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
    }

    /// Helper to set up a platform with a group and add a specific token event action.
    fn setup_group_and_add_action(
        event: TokenEvent,
        action_id_byte: u8,
    ) -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        std::sync::Arc<crate::platform_types::platform_state::PlatformState>,
        &'static dpp::version::PlatformVersion,
        Identifier,
    ) {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let contract_id = Identifier::from([1u8; 32]);
        let member_id = Identifier::from([2u8; 32]);
        let action_id = Identifier::from([action_id_byte; 32]);

        let mut members = BTreeMap::new();
        members.insert(member_id, 5u32);

        let group = Group::V0(GroupV0 {
            members,
            required_power: 10,
        });

        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        platform
            .drive
            .add_new_groups(
                contract_id,
                &groups,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to add groups");

        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: member_id,
            token_contract_position: 0,
            event: DppGroupActionEvent::TokenEvent(event),
        });

        platform
            .drive
            .add_group_action(
                contract_id,
                0,
                Some(action),
                false,
                action_id,
                member_id,
                5,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to add group action");

        (platform, state, version, contract_id)
    }

    #[test]
    fn test_query_group_actions_with_mint_event() {
        let recipient_id = Identifier::from([4u8; 32]);
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::Mint(1000, recipient_id, Some("test mint".to_string())),
            3,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::Mint(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_burn_event() {
        let burn_from_id = Identifier::from([5u8; 32]);
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::Burn(500, burn_from_id, Some("test burn".to_string())),
            4,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::Burn(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_freeze_event() {
        let frozen_id = Identifier::from([6u8; 32]);
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::Freeze(frozen_id, Some("freeze".to_string())),
            5,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::Freeze(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_unfreeze_event() {
        let frozen_id = Identifier::from([7u8; 32]);
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::Unfreeze(frozen_id, Some("unfreeze".to_string())),
            6,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::Unfreeze(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_destroy_frozen_funds_event() {
        let frozen_id = Identifier::from([8u8; 32]);
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::DestroyFrozenFunds(frozen_id, 100, Some("destroy".to_string())),
            7,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::DestroyFrozenFunds(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_emergency_action_pause() {
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::EmergencyAction(TokenEmergencyAction::Pause, Some("pause".to_string())),
            8,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::EmergencyAction(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_emergency_action_resume() {
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::EmergencyAction(TokenEmergencyAction::Resume, Some("resume".to_string())),
            9,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::EmergencyAction(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_change_price_no_schedule() {
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::ChangePriceForDirectPurchase(None, Some("remove price".to_string())),
            10,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::UpdatePrice(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_change_price_single_price() {
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::ChangePriceForDirectPurchase(
                Some(dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(42)),
                Some("set fixed price".to_string()),
            ),
            11,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::UpdatePrice(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_with_change_price_variable_prices() {
        let mut prices = BTreeMap::new();
        prices.insert(100u64, 50u64);
        prices.insert(1000u64, 40u64);

        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::ChangePriceForDirectPurchase(
                Some(dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SetPrices(prices)),
                Some("set variable price".to_string()),
            ),
            12,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(t.r#type.as_ref(), Some(token_event::Type::UpdatePrice(_)))
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_actions_prove_with_populated_data() {
        let recipient_id = Identifier::from([4u8; 32]);
        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::Mint(1000, recipient_id, Some("test mint".to_string())),
            3,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_actions_with_config_update_event() {
        use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

        let (platform, state, version, contract_id) = setup_group_and_add_action(
            TokenEvent::ConfigUpdate(
                TokenConfigurationChangeItem::MaxSupply(Some(1_000_000)),
                Some("update max supply".to_string()),
            ),
            13,
        );

        let request = GetGroupActionsRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            start_at_action_id: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_actions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionsResponseV0 {
                result:
                    Some(get_group_actions_response_v0::Result::GroupActions(GroupActions {
                        group_actions,
                    })),
                metadata: Some(_),
            }) => {
                assert_eq!(group_actions.len(), 1);
                let event = group_actions[0]
                    .event
                    .as_ref()
                    .expect("expected event")
                    .event_type
                    .as_ref();
                assert!(matches!(
                    event,
                    Some(group_action_event::EventType::TokenEvent(t))
                        if matches!(
                            t.r#type.as_ref(),
                            Some(token_event::Type::TokenConfigUpdate(_))
                        )
                ));
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }
}
