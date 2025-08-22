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

            GetGroupActionsResponseV0 {
                result: Some(get_group_actions_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
