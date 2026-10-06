//! Conversion between [`StateTransitionFilter`] and its wire message.

use super::{
    DocumentAction, DocumentActionMatch, DocumentFilter, Role, StateTransitionFilter,
    SubscriptionFilterError,
};
use crate::documents::document_query::{
    value_to_proto, where_clause_to_proto, where_operator_to_proto,
};
use crate::error::Error;
use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value, DocumentFieldValue as ProtoDocumentFieldValue,
    WhereClause as ProtoWhereClause, WhereOperator as ProtoWhereOperator,
};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::{
    self as proto, document_filter, state_transition_filter, SubscribeToStateTransitionsRequestV0,
    Version,
};
use dapi_grpc::platform::v0::SubscribeToStateTransitionsRequest;
use dpp::address_funds::PlatformAddress;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use drive::query::{ValueClause, WhereClause, WhereOperator};

/// Build a subscription request for `filters`, scanning from `from_block_height` (inclusive),
/// or from after the current tip when `None`.
///
/// Operands are encoded as given (see [`StateTransitionFilter::to_proto`]). A document filter
/// with clauses or a price should first go through
/// [`canonical_operands`](super::canonical_operands) with the contract it is on, as
/// `Sdk::subscribe_to_state_transitions` does: the wire carries only a few primitive types, so
/// an operand the matcher accepts in another representation than its field's (a `u128` for a
/// float field, `u8`s for a byte array) can arrive as one the node refuses.
pub fn subscribe_request(
    filters: &[StateTransitionFilter],
    from_block_height: Option<u64>,
) -> Result<SubscribeToStateTransitionsRequest, Error> {
    Ok(SubscribeToStateTransitionsRequest {
        version: Some(Version::V0(SubscribeToStateTransitionsRequestV0 {
            filters: filters
                .iter()
                .map(StateTransitionFilter::to_proto)
                .collect::<Result<_, _>>()?,
            from_block_height,
        })),
    })
}

impl StateTransitionFilter {
    /// The filter's wire message, with operands encoded as given: pass a document filter
    /// through [`canonical_operands`](super::canonical_operands) first, so that the node reads
    /// its operands as the schema types them (see [`subscribe_request`]).
    pub fn to_proto(&self) -> Result<proto::StateTransitionFilter, Error> {
        let filter = match self {
            StateTransitionFilter::Documents(filter) => {
                state_transition_filter::Filter::Documents(document_filter_to_proto(filter)?)
            }
            StateTransitionFilter::Addresses { addresses, role } => {
                state_transition_filter::Filter::Addresses(proto::AddressFilter {
                    addresses: addresses.iter().map(PlatformAddress::to_bytes).collect(),
                    role: role_to_proto(*role) as i32,
                })
            }
            StateTransitionFilter::Identities { identity_ids, role } => {
                state_transition_filter::Filter::Identities(proto::IdentityFilter {
                    identity_ids: identity_ids.iter().map(Identifier::to_vec).collect(),
                    role: role_to_proto(*role) as i32,
                })
            }
            StateTransitionFilter::Tokens {
                token_ids,
                identity_ids,
                role,
            } => state_transition_filter::Filter::Tokens(proto::TokenFilter {
                token_ids: token_ids.iter().map(Identifier::to_vec).collect(),
                identity_ids: identity_ids.iter().map(Identifier::to_vec).collect(),
                role: role_to_proto(*role) as i32,
            }),
            StateTransitionFilter::DataContracts { data_contract_ids } => {
                state_transition_filter::Filter::DataContracts(proto::DataContractFilter {
                    data_contract_ids: data_contract_ids.iter().map(Identifier::to_vec).collect(),
                })
            }
        };
        Ok(proto::StateTransitionFilter {
            filter: Some(filter),
        })
    }

    /// Decode the filter at position `index` of a request.
    pub fn from_proto(
        index: usize,
        filter: proto::StateTransitionFilter,
    ) -> Result<Self, SubscriptionFilterError> {
        let invalid = |message: String| SubscriptionFilterError::filter(index, message);
        let filter = filter
            .filter
            .ok_or_else(|| invalid("no filter kind is set".to_string()))?;
        Ok(match filter {
            state_transition_filter::Filter::Documents(filter) => StateTransitionFilter::Documents(
                document_filter_from_proto(filter).map_err(invalid)?,
            ),
            state_transition_filter::Filter::Addresses(filter) => {
                StateTransitionFilter::Addresses {
                    addresses: filter
                        .addresses
                        .iter()
                        .map(|bytes| {
                            PlatformAddress::from_bytes(bytes)
                                .map_err(|e| invalid(format!("invalid platform address: {e}")))
                        })
                        .collect::<Result<_, _>>()?,
                    role: role_from_proto(filter.role).map_err(invalid)?,
                }
            }
            state_transition_filter::Filter::Identities(filter) => {
                StateTransitionFilter::Identities {
                    identity_ids: identifiers(&filter.identity_ids).map_err(invalid)?,
                    role: role_from_proto(filter.role).map_err(invalid)?,
                }
            }
            state_transition_filter::Filter::Tokens(filter) => StateTransitionFilter::Tokens {
                token_ids: identifiers(&filter.token_ids).map_err(invalid)?,
                identity_ids: identifiers(&filter.identity_ids).map_err(invalid)?,
                role: role_from_proto(filter.role).map_err(invalid)?,
            },
            state_transition_filter::Filter::DataContracts(filter) => {
                StateTransitionFilter::DataContracts {
                    data_contract_ids: identifiers(&filter.data_contract_ids).map_err(invalid)?,
                }
            }
        })
    }
}

fn document_filter_to_proto(filter: &DocumentFilter) -> Result<proto::DocumentFilter, Error> {
    Ok(proto::DocumentFilter {
        data_contract_id: filter.data_contract_id.to_vec(),
        document_type_name: filter.document_type_name.clone(),
        actions: filter
            .actions
            .iter()
            .map(action_match_to_proto)
            .collect::<Result<_, _>>()?,
        batch_owner_id: filter.batch_owner_id.map(|id| id.to_vec()),
    })
}

/// A clause operand as sent in a subscription: 128-bit integers that fit 64 bits become 64-bit,
/// since the shared encoder sends 128-bit integers as text, which the node reads as text.
fn narrowed(value: &Value) -> Value {
    match value {
        Value::U128(n) => u64::try_from(*n).map_or(value.clone(), Value::U64),
        Value::I128(n) => i64::try_from(*n)
            .map(Value::I64)
            .or_else(|_| u64::try_from(*n).map(Value::U64))
            .unwrap_or_else(|_| value.clone()),
        Value::Array(values) => Value::Array(values.iter().map(narrowed).collect()),
        _ => value.clone(),
    }
}

fn narrowed_clause(clause: &WhereClause) -> WhereClause {
    WhereClause {
        field: clause.field.clone(),
        operator: clause.operator,
        value: narrowed(&clause.value),
    }
}

fn action_match_to_proto(
    action: &DocumentActionMatch,
) -> Result<document_filter::ActionMatch, Error> {
    let action_kind = action
        .action
        .ok_or_else(|| Error::Config("a document action match must name its action".to_string()))?;
    Ok(document_filter::ActionMatch {
        action: Some(action_to_proto(action_kind) as i32),
        new_document_where: action
            .new_document_where
            .iter()
            .map(narrowed_clause)
            .map(where_clause_to_proto)
            .collect::<Result<_, _>>()?,
        original_document_where: action
            .original_document_where
            .iter()
            .map(narrowed_clause)
            .map(where_clause_to_proto)
            .collect::<Result<_, _>>()?,
        owner_ids: action.owner_ids.iter().map(Identifier::to_vec).collect(),
        price: action
            .price
            .as_ref()
            .map(|price| {
                Ok::<_, Error>(document_filter::PriceClause {
                    operator: where_operator_to_proto(price.operator) as i32,
                    value: Some(value_to_proto(narrowed(&price.value))?),
                })
            })
            .transpose()?,
    })
}

fn document_filter_from_proto(filter: proto::DocumentFilter) -> Result<DocumentFilter, String> {
    Ok(DocumentFilter {
        data_contract_id: identifier(&filter.data_contract_id)?,
        document_type_name: filter.document_type_name,
        actions: filter
            .actions
            .into_iter()
            .map(action_match_from_proto)
            .collect::<Result<_, _>>()?,
        batch_owner_id: filter
            .batch_owner_id
            .as_deref()
            .map(identifier)
            .transpose()?,
    })
}

fn action_match_from_proto(
    action: document_filter::ActionMatch,
) -> Result<DocumentActionMatch, String> {
    let value = action
        .action
        .ok_or_else(|| "every action match must name its action".to_string())?;
    let action_kind = document_filter::Action::try_from(value)
        .map_err(|_| format!("unknown document action {value}"))?;
    Ok(DocumentActionMatch {
        action: Some(action_from_proto(action_kind)),
        new_document_where: where_clauses_from_proto(action.new_document_where)?,
        original_document_where: where_clauses_from_proto(action.original_document_where)?,
        owner_ids: identifiers(&action.owner_ids)?,
        price: action
            .price
            .map(|price| {
                Ok::<_, String>(ValueClause {
                    operator: where_operator_from_proto(price.operator)?,
                    value: value_from_proto(
                        price
                            .value
                            .ok_or_else(|| "the price clause has no value".to_string())?,
                        0,
                    )?,
                })
            })
            .transpose()?,
    })
}

fn where_clauses_from_proto(clauses: Vec<ProtoWhereClause>) -> Result<Vec<WhereClause>, String> {
    clauses
        .into_iter()
        .map(|clause| {
            if clause.time_range.is_some() || clause.integer_range.is_some() {
                return Err(format!(
                    "where clause on '{}': time and integer range selections are not supported \
                     in subscriptions",
                    clause.field
                ));
            }
            let value = clause
                .value
                .ok_or_else(|| format!("where clause on '{}' has no value", clause.field))?;
            Ok(WhereClause {
                operator: where_operator_from_proto(clause.operator)?,
                value: value_from_proto(value, 0)?,
                field: clause.field,
            })
        })
        .collect()
}

fn where_operator_from_proto(operator: i32) -> Result<WhereOperator, String> {
    let operator = ProtoWhereOperator::try_from(operator)
        .map_err(|_| format!("unknown where operator {operator}"))?;
    Ok(match operator {
        ProtoWhereOperator::Equal => WhereOperator::Equal,
        ProtoWhereOperator::GreaterThan => WhereOperator::GreaterThan,
        ProtoWhereOperator::GreaterThanOrEquals => WhereOperator::GreaterThanOrEquals,
        ProtoWhereOperator::LessThan => WhereOperator::LessThan,
        ProtoWhereOperator::LessThanOrEquals => WhereOperator::LessThanOrEquals,
        ProtoWhereOperator::Between => WhereOperator::Between,
        ProtoWhereOperator::BetweenExcludeBounds => WhereOperator::BetweenExcludeBounds,
        ProtoWhereOperator::BetweenExcludeLeft => WhereOperator::BetweenExcludeLeft,
        ProtoWhereOperator::BetweenExcludeRight => WhereOperator::BetweenExcludeRight,
        ProtoWhereOperator::In => WhereOperator::In,
        ProtoWhereOperator::StartsWith => WhereOperator::StartsWith,
        ProtoWhereOperator::InTimeRange | ProtoWhereOperator::InIntegerRange => {
            return Err(
                "time and integer range selections are not supported in subscriptions".to_string(),
            )
        }
    })
}

/// The wire operand as a `Value`, by primitive type; the document type's schema later brings
/// it to the field's type. Only one level of list (for IN and BETWEEN*) is accepted.
fn value_from_proto(value: ProtoDocumentFieldValue, depth: u8) -> Result<Value, String> {
    let variant = value
        .variant
        .ok_or_else(|| "a where clause value has no variant set".to_string())?;
    Ok(match variant {
        document_field_value::Variant::BoolValue(value) => Value::Bool(value),
        document_field_value::Variant::Int64Value(value) => Value::I64(value),
        document_field_value::Variant::Uint64Value(value) => Value::U64(value),
        document_field_value::Variant::DoubleValue(value) => Value::Float(value),
        document_field_value::Variant::Text(value) => Value::Text(value),
        document_field_value::Variant::BytesValue(value) => Value::Bytes(value),
        document_field_value::Variant::NullValue(_) => Value::Null,
        document_field_value::Variant::List(list) => {
            if depth >= 1 {
                return Err("nested where clause value lists are not supported".to_string());
            }
            Value::Array(
                list.values
                    .into_iter()
                    .map(|value| value_from_proto(value, depth + 1))
                    .collect::<Result<_, _>>()?,
            )
        }
    })
}

fn identifier(bytes: &[u8]) -> Result<Identifier, String> {
    Identifier::from_bytes(bytes)
        .map_err(|_| format!("expected a 32-byte identifier, got {} bytes", bytes.len()))
}

fn identifiers(ids: &[Vec<u8>]) -> Result<Vec<Identifier>, String> {
    ids.iter().map(|bytes| identifier(bytes)).collect()
}

fn role_to_proto(role: Role) -> proto::Role {
    match role {
        Role::Any => proto::Role::Any,
        Role::Sender => proto::Role::Sender,
        Role::Recipient => proto::Role::Recipient,
    }
}

fn role_from_proto(role: i32) -> Result<Role, String> {
    Ok(
        match proto::Role::try_from(role).map_err(|_| format!("unknown role {role}"))? {
            proto::Role::Any => Role::Any,
            proto::Role::Sender => Role::Sender,
            proto::Role::Recipient => Role::Recipient,
        },
    )
}

fn action_to_proto(action: DocumentAction) -> document_filter::Action {
    match action {
        DocumentAction::Create => document_filter::Action::Create,
        DocumentAction::Replace => document_filter::Action::Replace,
        DocumentAction::Delete => document_filter::Action::Delete,
        DocumentAction::Transfer => document_filter::Action::Transfer,
        DocumentAction::UpdatePrice => document_filter::Action::UpdatePrice,
        DocumentAction::Purchase => document_filter::Action::Purchase,
    }
}

fn action_from_proto(action: document_filter::Action) -> DocumentAction {
    match action {
        document_filter::Action::Create => DocumentAction::Create,
        document_filter::Action::Replace => DocumentAction::Replace,
        document_filter::Action::Delete => DocumentAction::Delete,
        document_filter::Action::Transfer => DocumentAction::Transfer,
        document_filter::Action::UpdatePrice => DocumentAction::UpdatePrice,
        document_filter::Action::Purchase => DocumentAction::Purchase,
    }
}
