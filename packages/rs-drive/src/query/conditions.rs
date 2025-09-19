//! Query Conditions
//!

use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{QuerySyntaxSimpleValidationResult, QuerySyntaxValidationResult};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentType, DocumentTypeRef};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::Query;
use sqlparser::ast;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use WhereOperator::{
    Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, Equal, GreaterThan,
    GreaterThanOrEquals, In, LessThan, LessThanOrEquals, StartsWith,
};

/// Converts SQL values to CBOR.
fn sql_value_to_platform_value(sql_value: ast::Value) -> Option<Value> {
    match sql_value {
        ast::Value::Boolean(bool) => Some(Value::Bool(bool)),
        ast::Value::Number(num, _) => {
            let number_as_string = num as String;
            if number_as_string.contains('.') {
                // Float
                let num_as_float = number_as_string.parse::<f64>().ok();
                num_as_float.map(Value::Float)
            } else {
                // Integer
                let num_as_int = number_as_string.parse::<i64>().ok();
                num_as_int.map(Value::I64)
            }
        }
        ast::Value::DoubleQuotedString(s) => Some(Value::Text(s)),
        ast::Value::SingleQuotedString(s) => Some(Value::Text(s)),
        ast::Value::HexStringLiteral(s) => Some(Value::Text(s)),
        ast::Value::NationalStringLiteral(s) => Some(Value::Text(s)),
        _ => None,
    }
}

/// Where operator arguments
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WhereOperator {
    /// Equal
    Equal,
    /// Greater than
    GreaterThan,
    /// Greater than or equal
    GreaterThanOrEquals,
    /// Less than
    LessThan,
    /// Less than or equal
    LessThanOrEquals,
    /// Between
    Between,
    /// Between excluding bounds
    BetweenExcludeBounds,
    /// Between excluding left bound
    BetweenExcludeLeft,
    /// Between excluding right bound
    BetweenExcludeRight,
    /// In
    In,
    /// Starts with
    StartsWith,
}

impl WhereOperator {
    /// Matches the where operator argument and returns true if it allows `flip` function
    pub fn allows_flip(&self) -> bool {
        match self {
            Equal => true,
            GreaterThan => true,
            GreaterThanOrEquals => true,
            LessThan => true,
            LessThanOrEquals => true,
            Between => false,
            BetweenExcludeBounds => false,
            BetweenExcludeLeft => false,
            BetweenExcludeRight => false,
            In => false,
            StartsWith => false,
        }
    }

    /// Flips the where operator
    pub fn flip(&self) -> Result<WhereOperator, Error> {
        match self {
            Equal => Ok(Equal),
            GreaterThan => Ok(LessThan),
            GreaterThanOrEquals => Ok(LessThanOrEquals),
            LessThan => Ok(GreaterThan),
            LessThanOrEquals => Ok(GreaterThanOrEquals),
            Between => Err(Error::Query(QuerySyntaxError::InvalidWhereClauseOrder(
                "Between clause order invalid",
            ))),
            BetweenExcludeBounds => Err(Error::Query(QuerySyntaxError::InvalidWhereClauseOrder(
                "Between clause order invalid",
            ))),
            BetweenExcludeLeft => Err(Error::Query(QuerySyntaxError::InvalidWhereClauseOrder(
                "Between clause order invalid",
            ))),
            BetweenExcludeRight => Err(Error::Query(QuerySyntaxError::InvalidWhereClauseOrder(
                "Between clause order invalid",
            ))),
            In => Err(Error::Query(QuerySyntaxError::InvalidWhereClauseOrder(
                "In clause order invalid",
            ))),
            StartsWith => Err(Error::Query(QuerySyntaxError::InvalidWhereClauseOrder(
                "Startswith clause order invalid",
            ))),
        }
    }
}

impl WhereOperator {
    /// Returns true if the where operator result is a range
    pub const fn is_range(self) -> bool {
        match self {
            Equal => false,
            GreaterThan | GreaterThanOrEquals | LessThan | LessThanOrEquals | Between
            | BetweenExcludeBounds | BetweenExcludeLeft | BetweenExcludeRight | In | StartsWith => {
                true
            }
        }
    }

    /// Matches the where operator as a string and returns it as a proper `WhereOperator`
    pub(crate) fn from_string(string: &str) -> Option<Self> {
        match string {
            "=" | "==" => Some(Equal),
            ">" => Some(GreaterThan),
            ">=" => Some(GreaterThanOrEquals),
            "<" => Some(LessThan),
            "<=" => Some(LessThanOrEquals),
            "Between" | "between" => Some(Between),
            "BetweenExcludeBounds"
            | "betweenExcludeBounds"
            | "betweenexcludebounds"
            | "between_exclude_bounds" => Some(BetweenExcludeBounds),
            "BetweenExcludeLeft"
            | "betweenExcludeLeft"
            | "betweenexcludeleft"
            | "between_exclude_left" => Some(BetweenExcludeLeft),
            "BetweenExcludeRight"
            | "betweenExcludeRight"
            | "betweenexcluderight"
            | "between_exclude_right" => Some(BetweenExcludeRight),
            "In" | "in" => Some(In),
            "StartsWith" | "startsWith" | "startswith" | "starts_with" => Some(StartsWith),
            &_ => None,
        }
    }

    /// Matches the where operator as a SQL operator and returns it as a proper `WhereOperator`
    pub(crate) fn from_sql_operator(sql_operator: ast::BinaryOperator) -> Option<Self> {
        match sql_operator {
            ast::BinaryOperator::Eq => Some(Equal),
            ast::BinaryOperator::Gt => Some(GreaterThan),
            ast::BinaryOperator::GtEq => Some(GreaterThanOrEquals),
            ast::BinaryOperator::Lt => Some(LessThan),
            ast::BinaryOperator::LtEq => Some(LessThanOrEquals),
            _ => None,
        }
    }

    /// Shared operator evaluator for both WhereClause and ValueClause
    pub fn eval(&self, left_value: &Value, right_value: &Value) -> bool {
        match self {
            Equal => left_value == right_value,
            GreaterThan => left_value > right_value,
            GreaterThanOrEquals => left_value >= right_value,
            LessThan => left_value < right_value,
            LessThanOrEquals => left_value <= right_value,
            In => match right_value {
                Value::Array(array) => array.contains(left_value),
                Value::Bytes(bytes) => match left_value {
                    Value::U8(b) => bytes.contains(b),
                    _ => false,
                },
                _ => false,
            },
            Between => match right_value {
                Value::Array(bounds) if bounds.len() == 2 => {
                    match bounds[0].partial_cmp(&bounds[1]) {
                        Some(Ordering::Less) | Some(Ordering::Equal) => {
                            left_value >= &bounds[0] && left_value <= &bounds[1]
                        }
                        _ => false,
                    }
                }
                _ => false,
            },
            BetweenExcludeBounds => match right_value {
                Value::Array(bounds) if bounds.len() == 2 => {
                    match bounds[0].partial_cmp(&bounds[1]) {
                        Some(Ordering::Less) | Some(Ordering::Equal) => {
                            left_value > &bounds[0] && left_value < &bounds[1]
                        }
                        _ => false,
                    }
                }
                _ => false,
            },
            BetweenExcludeLeft => match right_value {
                Value::Array(bounds) if bounds.len() == 2 => {
                    match bounds[0].partial_cmp(&bounds[1]) {
                        Some(Ordering::Less) | Some(Ordering::Equal) => {
                            left_value > &bounds[0] && left_value <= &bounds[1]
                        }
                        _ => false,
                    }
                }
                _ => false,
            },
            BetweenExcludeRight => match right_value {
                Value::Array(bounds) if bounds.len() == 2 => {
                    match bounds[0].partial_cmp(&bounds[1]) {
                        Some(Ordering::Less) | Some(Ordering::Equal) => {
                            left_value >= &bounds[0] && left_value < &bounds[1]
                        }
                        _ => false,
                    }
                }
                _ => false,
            },
            StartsWith => match (left_value, right_value) {
                (Value::Text(text), Value::Text(prefix)) => text.starts_with(prefix.as_str()),
                _ => false,
            },
        }
    }

    /// Validates that a value matches the expected shape for this operator and property type
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn value_shape_ok(&self, value: &Value, property_type: &DocumentPropertyType) -> bool {
        match self {
            Equal => true,
            In => matches!(value, Value::Array(_) | Value::Bytes(_)),
            StartsWith => matches!(value, Value::Text(_)),
            GreaterThan | GreaterThanOrEquals | LessThan | LessThanOrEquals => {
                match property_type {
                    DocumentPropertyType::F64 => is_numeric_value(value),
                    DocumentPropertyType::String(_) => {
                        matches!(value, Value::Text(_))
                    }
                    _ => matches!(
                        value,
                        Value::U128(_)
                            | Value::I128(_)
                            | Value::U64(_)
                            | Value::I64(_)
                            | Value::U32(_)
                            | Value::I32(_)
                            | Value::U16(_)
                            | Value::I16(_)
                            | Value::U8(_)
                            | Value::I8(_)
                    ),
                }
            }
            Between | BetweenExcludeBounds | BetweenExcludeLeft | BetweenExcludeRight => {
                if let Value::Array(arr) = value {
                    arr.len() == 2
                        && arr.iter().all(|x| match property_type {
                            DocumentPropertyType::F64 => is_numeric_value(x),
                            DocumentPropertyType::String(_) => {
                                matches!(x, Value::Text(_))
                            }
                            _ => matches!(
                                x,
                                Value::U128(_)
                                    | Value::I128(_)
                                    | Value::U64(_)
                                    | Value::I64(_)
                                    | Value::U32(_)
                                    | Value::I32(_)
                                    | Value::U16(_)
                                    | Value::I16(_)
                                    | Value::U8(_)
                                    | Value::I8(_)
                            ),
                        })
                } else {
                    false
                }
            }
        }
    }
}

impl Display for WhereOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Equal => "=",
            GreaterThan => ">",
            GreaterThanOrEquals => ">=",
            LessThan => "<",
            LessThanOrEquals => "<=",
            Between => "Between",
            BetweenExcludeBounds => "BetweenExcludeBounds",
            BetweenExcludeLeft => "BetweenExcludeLeft",
            BetweenExcludeRight => "BetweenExcludeRight",
            In => "In",
            StartsWith => "StartsWith",
        };

        write!(f, "{}", s)
    }
}

impl From<WhereOperator> for Value {
    fn from(value: WhereOperator) -> Self {
        Self::Text(value.to_string())
    }
}

/// Where clause struct
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WhereClause {
    /// Field
    pub field: String,
    /// Operator
    pub operator: WhereOperator,
    /// Value
    pub value: Value,
}

impl<'a> WhereClause {
    /// Returns true if the `WhereClause` is an identifier
    pub fn is_identifier(&self) -> bool {
        self.field == "$id"
    }

    /// Returns the where clause `in` values if they are an array of values, else an error
    pub fn in_values(&self) -> QuerySyntaxValidationResult<Cow<'_, Vec<Value>>> {
        let in_values = match &self.value {
            Value::Array(array) => Cow::Borrowed(array),
            Value::Bytes(bytes) => Cow::Owned(bytes.iter().map(|int| Value::U8(*int)).collect()),
            _ => {
                return QuerySyntaxValidationResult::new_with_error(
                    QuerySyntaxError::InvalidInClause(
                        "when using in operator you must provide an array of values".to_string(),
                    ),
                )
            }
        };

        let len = in_values.len();
        if len == 0 {
            return QuerySyntaxValidationResult::new_with_error(QuerySyntaxError::InvalidInClause(
                "in clause must have at least 1 value".to_string(),
            ));
        }

        if len > 100 {
            return QuerySyntaxValidationResult::new_with_error(QuerySyntaxError::InvalidInClause(
                "in clause must have at most 100 values".to_string(),
            ));
        }

        // Throw an error if there are duplicates
        if (1..in_values.len()).any(|i| in_values[i..].contains(&in_values[i - 1])) {
            return QuerySyntaxValidationResult::new_with_error(QuerySyntaxError::InvalidInClause(
                "there should be no duplicates values for In query".to_string(),
            ));
        }
        QuerySyntaxValidationResult::new_with_data(in_values)
    }

    /// Returns true if the less than where clause is true
    pub fn less_than(&self, other: &Self, allow_eq: bool) -> Result<bool, Error> {
        match (&self.value, &other.value) {
            (Value::I128(x), Value::I128(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::U128(x), Value::U128(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::I64(x), Value::I64(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::U64(x), Value::U64(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::I32(x), Value::I32(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::U32(x), Value::U32(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::I16(x), Value::I16(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::U16(x), Value::U16(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::I8(x), Value::I8(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::U8(x), Value::U8(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::Bytes(x), Value::Bytes(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::Float(x), Value::Float(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            (Value::Text(x), Value::Text(y)) => {
                if allow_eq {
                    Ok(x.le(y))
                } else {
                    Ok(x.lt(y))
                }
            }
            _ => Err(Error::Query(QuerySyntaxError::RangeClausesNotGroupable(
                "range clauses can not be coherently grouped",
            ))),
        }
    }

    /// Returns a `WhereClause` given a list of clause components
    pub fn from_components(clause_components: &'a [Value]) -> Result<Self, Error> {
        if clause_components.len() != 3 {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "where clauses should have at most 3 components",
                ),
            ));
        }

        let field_value = clause_components
            .first()
            .expect("check above enforces it exists");
        let field_ref = field_value.as_text().ok_or(Error::Query(
            QuerySyntaxError::InvalidWhereClauseComponents(
                "first field of where component should be a string",
            ),
        ))?;
        let field = String::from(field_ref);

        let operator_value = clause_components
            .get(1)
            .expect("check above enforces it exists");
        let operator_string = operator_value.as_text().ok_or(Error::Query(
            QuerySyntaxError::InvalidWhereClauseComponents(
                "second field of where component should be a string",
            ),
        ))?;

        let operator = WhereOperator::from_string(operator_string).ok_or({
            Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                "second field of where component should be a known operator",
            ))
        })?;

        let value = clause_components
            .get(2)
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "third field of where component should exist",
                ),
            ))?
            .clone();

        Ok(WhereClause {
            field,
            operator,
            value,
        })
    }

    fn lower_bound_clause(where_clauses: &'a [&WhereClause]) -> Result<Option<&'a Self>, Error> {
        let lower_range_clauses: Vec<&&WhereClause> = where_clauses
            .iter()
            .filter(|&where_clause| {
                matches!(where_clause.operator, GreaterThan | GreaterThanOrEquals)
            })
            .collect::<Vec<&&WhereClause>>();
        match lower_range_clauses.len() {
            0 => Ok(None),
            1 => Ok(Some(lower_range_clauses.first().unwrap())),
            _ => Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                "there can only at most one range clause with a lower bound",
            ))),
        }
    }

    fn upper_bound_clause(where_clauses: &'a [&WhereClause]) -> Result<Option<&'a Self>, Error> {
        let upper_range_clauses: Vec<&&WhereClause> = where_clauses
            .iter()
            .filter(|&where_clause| matches!(where_clause.operator, LessThan | LessThanOrEquals))
            .collect::<Vec<&&WhereClause>>();
        match upper_range_clauses.len() {
            0 => Ok(None),
            1 => Ok(Some(upper_range_clauses.first().unwrap())),
            _ => Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                "there can only at most one range clause with a lower bound",
            ))),
        }
    }

    /// Given a list of where clauses, returns them in groups of equal, range, and in clauses
    #[allow(clippy::type_complexity)]
    pub(crate) fn group_clauses(
        where_clauses: &'a [WhereClause],
        // TODO: Define a type/struct for return value
    ) -> Result<(BTreeMap<String, Self>, Option<Self>, Option<Self>), Error> {
        if where_clauses.is_empty() {
            return Ok((BTreeMap::new(), None, None));
        }
        let equal_clauses_array =
            where_clauses
                .iter()
                .filter_map(|where_clause| match where_clause.operator {
                    Equal => match where_clause.is_identifier() {
                        true => None,
                        false => Some(where_clause.clone()),
                    },
                    _ => None,
                });
        let mut known_fields: BTreeSet<String> = BTreeSet::new();
        let equal_clauses: BTreeMap<String, WhereClause> = equal_clauses_array
            .into_iter()
            .map(|where_clause| {
                if known_fields.contains(&where_clause.field) {
                    Err(Error::Query(
                        QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                            "duplicate equality fields",
                        ),
                    ))
                } else {
                    known_fields.insert(where_clause.field.clone());
                    Ok((where_clause.field.clone(), where_clause))
                }
            })
            .collect::<Result<BTreeMap<String, WhereClause>, Error>>()?;

        let in_clauses_array = where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                In => match where_clause.is_identifier() {
                    true => None,
                    false => Some(where_clause.clone()),
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let in_clause = match in_clauses_array.len() {
            0 => Ok(None),
            1 => {
                let clause = in_clauses_array.first().expect("there must be a value");
                if known_fields.contains(&clause.field) {
                    Err(Error::Query(
                        QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                            "in clause has same field as an equality clause",
                        ),
                    ))
                } else {
                    known_fields.insert(clause.field.clone());
                    Ok(Some(clause.clone()))
                }
            }
            _ => Err(Error::Query(QuerySyntaxError::MultipleInClauses(
                "There should only be one in clause",
            ))),
        }?;

        // In order to group range clauses
        let groupable_range_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|where_clause| match where_clause.operator {
                Equal => false,
                In => false,
                GreaterThan => true,
                GreaterThanOrEquals => true,
                LessThan => true,
                LessThanOrEquals => true,
                StartsWith => false,
                Between => false,
                BetweenExcludeBounds => false,
                BetweenExcludeRight => false,
                BetweenExcludeLeft => false,
            })
            .collect();

        let non_groupable_range_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|where_clause| match where_clause.operator {
                Equal => false,
                In => false,
                GreaterThan => false,
                GreaterThanOrEquals => false,
                LessThan => false,
                LessThanOrEquals => false,
                StartsWith => true,
                Between => true,
                BetweenExcludeBounds => true,
                BetweenExcludeRight => true,
                BetweenExcludeLeft => true,
            })
            .collect();

        let range_clause =
            if non_groupable_range_clauses.is_empty() {
                if groupable_range_clauses.is_empty() {
                    Ok(None)
                } else if groupable_range_clauses.len() == 1 {
                    let clause = *groupable_range_clauses.first().unwrap();
                    if known_fields.contains(clause.field.as_str()) {
                        Err(Error::Query(
                            QuerySyntaxError::InvalidWhereClauseComponents(
                                "in clause has same field as an equality clause",
                            ),
                        ))
                    } else {
                        Ok(Some(clause.clone()))
                    }
                } else if groupable_range_clauses.len() > 2 {
                    Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                        "there can only be at most 2 range clauses that must be on the same field",
                    )))
                } else {
                    let first_field = groupable_range_clauses.first().unwrap().field.as_str();
                    if known_fields.contains(first_field) {
                        Err(Error::Query(
                            QuerySyntaxError::InvalidWhereClauseComponents(
                                "a range clause has same field as an equality or in clause",
                            ),
                        ))
                    } else if groupable_range_clauses
                        .iter()
                        .any(|&z| z.field.as_str() != first_field)
                    {
                        Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                            "all ranges must be on same field",
                        )))
                    } else {
                        let lower_upper_error = || {
                            Error::Query(QuerySyntaxError::RangeClausesNotGroupable(
                                "lower and upper bounds must be passed if providing 2 ranges",
                            ))
                        };

                        // we need to find the bounds of the clauses
                        let lower_bounds_clause =
                            WhereClause::lower_bound_clause(groupable_range_clauses.as_slice())?
                                .ok_or_else(lower_upper_error)?;
                        let upper_bounds_clause =
                            WhereClause::upper_bound_clause(groupable_range_clauses.as_slice())?
                                .ok_or_else(lower_upper_error)?;

                        let operator =
                            match (lower_bounds_clause.operator, upper_bounds_clause.operator) {
                                (GreaterThanOrEquals, LessThanOrEquals) => Some(Between),
                                (GreaterThanOrEquals, LessThan) => Some(BetweenExcludeRight),
                                (GreaterThan, LessThanOrEquals) => Some(BetweenExcludeLeft),
                                (GreaterThan, LessThan) => Some(BetweenExcludeBounds),
                                _ => None,
                            }
                            .ok_or_else(lower_upper_error)?;

                        if upper_bounds_clause
                            .less_than(lower_bounds_clause, operator == BetweenExcludeBounds)?
                        {
                            return Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                                "lower bounds must be under upper bounds",
                            )));
                        }

                        Ok(Some(WhereClause {
                            field: groupable_range_clauses.first().unwrap().field.clone(),
                            operator,
                            value: Value::Array(vec![
                                lower_bounds_clause.value.clone(),
                                upper_bounds_clause.value.clone(),
                            ]),
                        }))
                    }
                }
            } else if non_groupable_range_clauses.len() == 1 && groupable_range_clauses.is_empty() {
                let where_clause = *non_groupable_range_clauses.first().unwrap();
                if where_clause.operator == StartsWith {
                    // Starts with must null be against an empty string
                    if let Value::Text(text) = &where_clause.value {
                        if text.is_empty() {
                            return Err(Error::Query(QuerySyntaxError::StartsWithIllegalString(
                                "starts with can not start with an empty string",
                            )));
                        }
                    }
                }
                if known_fields.contains(where_clause.field.as_str()) {
                    Err(Error::Query(QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                    "a non groupable range clause has same field as an equality or in clause",
                )))
                } else {
                    Ok(Some(where_clause.clone()))
                }
            } else if groupable_range_clauses.is_empty() {
                Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                    "there can not be more than 1 non groupable range clause",
                )))
            } else {
                Err(Error::Query(QuerySyntaxError::RangeClausesNotGroupable(
                    "clauses are not groupable",
                )))
            }?;

        Ok((equal_clauses, range_clause, in_clause))
    }

    fn split_value_for_between(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let in_values = match &self.value {
            Value::Array(array) => Some(array),
            _ => None,
        }
        .ok_or({
            Error::Query(QuerySyntaxError::InvalidBetweenClause(
                "when using between operator you must provide a tuple array of values",
            ))
        })?;
        if in_values.len() != 2 {
            return Err(Error::Query(QuerySyntaxError::InvalidBetweenClause(
                "when using between operator you must provide an array of exactly two values",
            )));
        }
        let left_key = document_type.serialize_value_for_key(
            self.field.as_str(),
            in_values.first().unwrap(),
            platform_version,
        )?;
        let right_key = document_type.serialize_value_for_key(
            self.field.as_str(),
            in_values.get(1).unwrap(),
            platform_version,
        )?;
        Ok((left_key, right_key))
    }

    /// Returns a path query given the parameters
    // The start at document fields are:
    // document: The Document that we should start at
    // included: whether we should start at or after this document
    // left_to_right: should we be going left to right or right to left?
    pub(crate) fn to_path_query(
        &self,
        document_type: DocumentTypeRef,
        start_at_document: &Option<(Document, bool)>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Query, Error> {
        // If there is a start_at_document, we need to get the value that it has for the
        // current field.
        let starts_at_key_option = match start_at_document {
            None => None,
            Some((document, included)) => {
                // if the key doesn't exist then we should ignore the starts at key
                document
                    .get_raw_for_document_type(
                        self.field.as_str(),
                        document_type,
                        None,
                        platform_version,
                    )?
                    .map(|raw_value_option| (raw_value_option, *included))
            }
        };

        let mut query = Query::new_with_direction(left_to_right);
        match self.operator {
            Equal => {
                let key = document_type.serialize_value_for_key(
                    self.field.as_str(),
                    &self.value,
                    platform_version,
                )?;
                match starts_at_key_option {
                    None => {
                        query.insert_key(key);
                    }
                    Some((starts_at_key, included)) => {
                        if (left_to_right && starts_at_key < key)
                            || (!left_to_right && starts_at_key > key)
                            || (included && starts_at_key == key)
                        {
                            query.insert_key(key);
                        }
                    }
                }
            }
            In => {
                let in_values = self.in_values().into_data_with_error()??;

                match starts_at_key_option {
                    None => {
                        for value in in_values.iter() {
                            let key = document_type.serialize_value_for_key(
                                self.field.as_str(),
                                value,
                                platform_version,
                            )?;
                            query.insert_key(key)
                        }
                    }
                    Some((starts_at_key, included)) => {
                        for value in in_values.iter() {
                            let key = document_type.serialize_value_for_key(
                                self.field.as_str(),
                                value,
                                platform_version,
                            )?;

                            if (left_to_right && starts_at_key < key)
                                || (!left_to_right && starts_at_key > key)
                                || (included && starts_at_key == key)
                            {
                                query.insert_key(key);
                            }
                        }
                    }
                }
            }
            GreaterThan => {
                let key = document_type.serialize_value_for_key(
                    self.field.as_str(),
                    &self.value,
                    platform_version,
                )?;
                match starts_at_key_option {
                    None => query.insert_range_after(key..),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key <= key {
                                query.insert_range_after(key..);
                            } else if included {
                                query.insert_range_from(starts_at_key..);
                            } else {
                                query.insert_range_after(starts_at_key..);
                            }
                        } else if starts_at_key > key {
                            if included {
                                query.insert_range_after_to_inclusive(key..=starts_at_key);
                            } else {
                                query.insert_range_after_to(key..starts_at_key);
                            }
                        }
                    }
                }
            }
            GreaterThanOrEquals => {
                let key = document_type.serialize_value_for_key(
                    self.field.as_str(),
                    &self.value,
                    platform_version,
                )?;
                match starts_at_key_option {
                    None => query.insert_range_from(key..),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key < key || (included && starts_at_key == key) {
                                query.insert_range_from(key..);
                            } else if included {
                                query.insert_range_from(starts_at_key..);
                            } else {
                                query.insert_range_after(starts_at_key..);
                            }
                        } else if starts_at_key > key {
                            if included {
                                query.insert_range_inclusive(key..=starts_at_key);
                            } else {
                                query.insert_range(key..starts_at_key);
                            }
                        } else if included && starts_at_key == key {
                            query.insert_key(key);
                        }
                    }
                }
            }
            LessThan => {
                let key = document_type.serialize_value_for_key(
                    self.field.as_str(),
                    &self.value,
                    platform_version,
                )?;
                match starts_at_key_option {
                    None => query.insert_range_to(..key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key < key {
                                if included {
                                    query.insert_range(key..starts_at_key);
                                } else {
                                    query.insert_range_after_to(key..starts_at_key);
                                }
                            }
                        } else if starts_at_key > key {
                            query.insert_range_to(..key);
                        } else if included {
                            query.insert_range_to_inclusive(..=starts_at_key);
                        } else {
                            query.insert_range_to(..starts_at_key);
                        }
                    }
                }
            }
            LessThanOrEquals => {
                let key = document_type.serialize_value_for_key(
                    self.field.as_str(),
                    &self.value,
                    platform_version,
                )?;
                match starts_at_key_option {
                    None => query.insert_range_to_inclusive(..=key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if included && starts_at_key == key {
                                query.insert_key(key);
                            } else if starts_at_key < key {
                                if included {
                                    query.insert_range_inclusive(key..=starts_at_key);
                                } else {
                                    query.insert_range_after_to_inclusive(key..=starts_at_key);
                                }
                            }
                        } else if starts_at_key > key || (included && starts_at_key == key) {
                            query.insert_range_to_inclusive(..=key);
                        } else if included {
                            query.insert_range_to_inclusive(..=starts_at_key);
                        } else {
                            query.insert_range_to(..starts_at_key);
                        }
                    }
                }
            }
            Between => {
                let (left_key, right_key) =
                    self.split_value_for_between(document_type, platform_version)?;
                match starts_at_key_option {
                    None => query.insert_range_inclusive(left_key..=right_key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key < left_key || (included && starts_at_key == left_key) {
                                query.insert_range_inclusive(left_key..=right_key)
                            } else if starts_at_key == left_key {
                                query.insert_range_after_to_inclusive(left_key..=right_key)
                            } else if starts_at_key > left_key && starts_at_key < right_key {
                                if included {
                                    query.insert_range_inclusive(starts_at_key..=right_key);
                                } else {
                                    query
                                        .insert_range_after_to_inclusive(starts_at_key..=right_key);
                                }
                            } else if starts_at_key == right_key && included {
                                query.insert_key(right_key);
                            }
                        } else if starts_at_key > right_key
                            || (included && starts_at_key == right_key)
                        {
                            query.insert_range_inclusive(left_key..=right_key)
                        } else if starts_at_key == right_key {
                            query.insert_range(left_key..right_key)
                        } else if starts_at_key > left_key && starts_at_key < right_key {
                            if included {
                                query.insert_range_inclusive(left_key..=starts_at_key);
                            } else {
                                query.insert_range(left_key..starts_at_key);
                            }
                        } else if starts_at_key == left_key && included {
                            query.insert_key(left_key);
                        }
                    }
                }
            }
            BetweenExcludeBounds => {
                let (left_key, right_key) =
                    self.split_value_for_between(document_type, platform_version)?;
                match starts_at_key_option {
                    None => query.insert_range_after_to(left_key..right_key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key <= left_key {
                                query.insert_range_after_to(left_key..right_key)
                            } else if starts_at_key > left_key && starts_at_key < right_key {
                                if included {
                                    query.insert_range(starts_at_key..right_key);
                                } else {
                                    query.insert_range_after_to(starts_at_key..right_key);
                                }
                            }
                        } else if starts_at_key > right_key {
                            query.insert_range_inclusive(left_key..=right_key)
                        } else if starts_at_key == right_key {
                            query.insert_range(left_key..right_key)
                        } else if starts_at_key > left_key && starts_at_key < right_key {
                            if included {
                                query.insert_range_after_to_inclusive(left_key..=starts_at_key);
                            } else {
                                query.insert_range_after_to(left_key..starts_at_key);
                            }
                        }
                    }
                }
            }
            BetweenExcludeLeft => {
                let (left_key, right_key) =
                    self.split_value_for_between(document_type, platform_version)?;
                match starts_at_key_option {
                    None => query.insert_range_after_to_inclusive(left_key..=right_key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key <= left_key {
                                query.insert_range_after_to_inclusive(left_key..=right_key)
                            } else if starts_at_key > left_key && starts_at_key < right_key {
                                if included {
                                    query.insert_range_inclusive(starts_at_key..=right_key);
                                } else {
                                    query
                                        .insert_range_after_to_inclusive(starts_at_key..=right_key);
                                }
                            } else if starts_at_key == right_key && included {
                                query.insert_key(right_key);
                            }
                        } else if starts_at_key > right_key
                            || (included && starts_at_key == right_key)
                        {
                            query.insert_range_after_to_inclusive(left_key..=right_key)
                        } else if starts_at_key > left_key && starts_at_key < right_key {
                            if included {
                                query.insert_range_inclusive(left_key..=starts_at_key);
                            } else {
                                query.insert_range(left_key..starts_at_key);
                            }
                        }
                    }
                }
            }
            BetweenExcludeRight => {
                let (left_key, right_key) =
                    self.split_value_for_between(document_type, platform_version)?;
                match starts_at_key_option {
                    None => query.insert_range(left_key..right_key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key < left_key || (included && starts_at_key == left_key) {
                                query.insert_range(left_key..right_key)
                            } else if starts_at_key == left_key {
                                query.insert_range_after_to(left_key..right_key)
                            } else if starts_at_key > left_key && starts_at_key < right_key {
                                if included {
                                    query.insert_range(starts_at_key..right_key);
                                } else {
                                    query.insert_range_after_to(starts_at_key..right_key);
                                }
                            }
                        } else if starts_at_key >= right_key {
                            query.insert_range(left_key..right_key)
                        } else if starts_at_key > left_key && starts_at_key < right_key {
                            if included {
                                query.insert_range_inclusive(left_key..=starts_at_key);
                            } else {
                                query.insert_range(left_key..starts_at_key);
                            }
                        } else if starts_at_key == left_key && included {
                            query.insert_key(left_key);
                        }
                    }
                }
            }
            StartsWith => {
                let left_key = document_type.serialize_value_for_key(
                    self.field.as_str(),
                    &self.value,
                    platform_version,
                )?;
                let mut right_key = left_key.clone();
                let last_char = right_key.last_mut().ok_or({
                    Error::Query(QuerySyntaxError::InvalidStartsWithClause(
                        "starts with must have at least one character",
                    ))
                })?;
                *last_char += 1;
                match starts_at_key_option {
                    None => query.insert_range(left_key..right_key),
                    Some((starts_at_key, included)) => {
                        if left_to_right {
                            if starts_at_key < left_key || (included && starts_at_key == left_key) {
                                query.insert_range(left_key..right_key)
                            } else if starts_at_key == left_key {
                                query.insert_range_after_to(left_key..right_key)
                            } else if starts_at_key > left_key && starts_at_key < right_key {
                                if included {
                                    query.insert_range(starts_at_key..right_key);
                                } else {
                                    query.insert_range_after_to(starts_at_key..right_key);
                                }
                            }
                        } else if starts_at_key >= right_key {
                            query.insert_range(left_key..right_key)
                        } else if starts_at_key > left_key && starts_at_key < right_key {
                            if included {
                                query.insert_range_inclusive(left_key..=starts_at_key);
                            } else {
                                query.insert_range(left_key..starts_at_key);
                            }
                        } else if starts_at_key == left_key && included {
                            query.insert_key(left_key);
                        }
                    }
                }
            }
        }
        Ok(query)
    }

    pub(crate) fn build_where_clauses_from_operations(
        binary_operation: &ast::Expr,
        document_type: &DocumentType,
        where_clauses: &mut Vec<WhereClause>,
    ) -> Result<(), Error> {
        match &binary_operation {
            ast::Expr::InList {
                expr,
                list,
                negated,
            } => {
                if *negated {
                    return Err(Error::Query(QuerySyntaxError::Unsupported(
                        "Invalid query: negated in clause not supported".to_string(),
                    )));
                }

                let field_name: String = if let ast::Expr::Identifier(ident) = &**expr {
                    ident.value.clone()
                } else {
                    return Err(Error::Query(QuerySyntaxError::InvalidInClause(
                        "Invalid query: in clause should start with an identifier".to_string(),
                    )));
                };

                let property_type = if let Some(ty) = meta_field_property_type(&field_name) {
                    Cow::Owned(ty)
                } else {
                    let property = document_type
                        .flattened_properties()
                        .get(&field_name)
                        .ok_or_else(|| {
                            Error::Query(QuerySyntaxError::InvalidSQL(format!(
                                "Invalid query: property named {} not in document type",
                                field_name
                            )))
                        })?;
                    Cow::Borrowed(&property.property_type)
                };

                let mut in_values: Vec<Value> = Vec::new();
                for value in list {
                    if let ast::Expr::Value(sql_value) = value {
                        let platform_value =
                            sql_value_to_platform_value(sql_value.clone()).ok_or({
                                Error::Query(QuerySyntaxError::InvalidSQL(
                                    "Invalid query: unexpected value type".to_string(),
                                ))
                            })?;
                        let transformed_value = if let Value::Text(text_value) = &platform_value {
                            property_type.value_from_string(text_value)?
                        } else {
                            platform_value
                        };

                        in_values.push(transformed_value);
                    } else {
                        return Err(Error::Query(QuerySyntaxError::InvalidSQL(
                            "Invalid query: expected a list of sql values".to_string(),
                        )));
                    }
                }

                where_clauses.push(WhereClause {
                    field: field_name,
                    operator: In,
                    value: Value::Array(in_values),
                });

                Ok(())
            }
            ast::Expr::Like {
                negated,
                expr,
                pattern,
                escape_char: _,
            } => {
                let where_operator = StartsWith;
                if *negated {
                    return Err(Error::Query(QuerySyntaxError::Unsupported(
                        "Negated Like not supported".to_string(),
                    )));
                }

                let field_name: String = if let ast::Expr::Identifier(ident) = &**expr {
                    ident.value.clone()
                } else {
                    panic!("unreachable: confirmed it's identifier variant");
                };

                let transformed_value = if let ast::Expr::Value(value) = &**pattern {
                    let platform_value = sql_value_to_platform_value(value.clone()).ok_or({
                        Error::Query(QuerySyntaxError::InvalidSQL(
                            "Invalid query: unexpected value type".to_string(),
                        ))
                    })?;

                    // make sure the value is of the right format i.e. prefix%
                    let inner_text = platform_value.as_text().ok_or({
                        Error::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "Invalid query: startsWith takes text",
                        ))
                    })?;
                    let match_locations: Vec<_> = inner_text.match_indices('%').collect();
                    if match_locations.len() == 1 && match_locations[0].0 == inner_text.len() - 1 {
                        Value::Text(String::from(&inner_text[..(inner_text.len() - 1)]))
                    } else {
                        return Err(Error::Query(QuerySyntaxError::Unsupported(
                            "Invalid query: like can only be used to represent startswith"
                                .to_string(),
                        )));
                    }
                } else {
                    panic!("unreachable: confirmed it's value variant");
                };

                where_clauses.push(WhereClause {
                    field: field_name,
                    operator: where_operator,
                    value: transformed_value,
                });
                Ok(())
            }
            ast::Expr::BinaryOp { left, op, right } => {
                if *op == ast::BinaryOperator::And {
                    Self::build_where_clauses_from_operations(left, document_type, where_clauses)?;
                    Self::build_where_clauses_from_operations(right, document_type, where_clauses)?;
                } else {
                    let mut where_operator =
                        WhereOperator::from_sql_operator(op.clone()).ok_or(Error::Query(
                            QuerySyntaxError::Unsupported("Unknown operator".to_string()),
                        ))?;

                    let identifier;
                    let value_expr;

                    if matches!(&**left, ast::Expr::Identifier(_))
                        && matches!(&**right, ast::Expr::Value(_))
                    {
                        identifier = &**left;
                        value_expr = &**right;
                    } else if matches!(&**right, ast::Expr::Identifier(_))
                        && matches!(&**left, ast::Expr::Value(_))
                    {
                        identifier = &**right;
                        value_expr = &**left;
                        where_operator = where_operator.flip()?;
                    } else {
                        return Err(Error::Query(QuerySyntaxError::InvalidSQL(
                            "Invalid query: where clause should have field name and value"
                                .to_string(),
                        )));
                    }

                    let field_name: String = if let ast::Expr::Identifier(ident) = identifier {
                        ident.value.clone()
                    } else {
                        panic!("unreachable: confirmed it's identifier variant");
                    };

                    let property_type = if let Some(ty) = meta_field_property_type(&field_name) {
                        Cow::Owned(ty)
                    } else {
                        let property = document_type
                            .flattened_properties()
                            .get(&field_name)
                            .ok_or_else(|| {
                                Error::Query(QuerySyntaxError::InvalidSQL(format!(
                                    "Invalid query: property named {} not in document type",
                                    field_name
                                )))
                            })?;
                        Cow::Borrowed(&property.property_type)
                    };

                    let transformed_value = if let ast::Expr::Value(value) = value_expr {
                        let platform_value = sql_value_to_platform_value(value.clone()).ok_or({
                            Error::Query(QuerySyntaxError::InvalidSQL(
                                "Invalid query: unexpected value type".to_string(),
                            ))
                        })?;

                        if let Value::Text(text_value) = &platform_value {
                            property_type.value_from_string(text_value)?
                        } else {
                            platform_value
                        }
                    } else {
                        panic!("unreachable: confirmed it's value variant");
                    };

                    where_clauses.push(WhereClause {
                        field: field_name,
                        operator: where_operator,
                        value: transformed_value,
                    });
                }
                Ok(())
            }
            _ => Err(Error::Query(QuerySyntaxError::InvalidSQL(
                "Issue parsing sql: invalid selection format".to_string(),
            ))),
        }
    }

    /// Evaluate this WhereClause against a provided `Value`
    pub fn matches_value(&self, value: &Value) -> bool {
        self.operator.eval(value, &self.value)
    }

    /// Validate this where clause against the document schema
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate_against_schema(
        &self,
        document_type: DocumentTypeRef,
    ) -> QuerySyntaxSimpleValidationResult {
        // First determine the property type of self.field
        let property_type_cow = if let Some(meta_ty) = meta_field_property_type(&self.field) {
            Cow::Owned(meta_ty)
        } else {
            // Check that the field exists in the schema
            let Some(property) = document_type.flattened_properties().get(&self.field) else {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents("unknown field in where clause"),
                );
            };
            Cow::Borrowed(&property.property_type)
        };

        // Check operator is allowed for field type
        let property_type = property_type_cow.as_ref();
        if !allowed_ops_for_type(property_type).contains(&self.operator) {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "operator not allowed for field type",
                ),
            );
        }

        // Check starts_with value is not empty
        if self.operator == StartsWith {
            if let Value::Text(s) = &self.value {
                if s.is_empty() {
                    return QuerySyntaxSimpleValidationResult::new_with_error(
                        QuerySyntaxError::StartsWithIllegalString(
                            "starts_with can not start with an empty string",
                        ),
                    );
                }
            }
        }

        // Check in clause values
        if self.operator == In {
            // Ensure array value, length bounds and no duplicates
            let result = self.in_values();
            if !result.is_valid() {
                return QuerySyntaxSimpleValidationResult::new_with_errors(result.errors);
            }
            // If value provided as Bytes, only allow for U8 numeric fields
            if matches!(self.value, Value::Bytes(_))
                && !matches!(property_type, DocumentPropertyType::U8)
            {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "IN Bytes only allowed for U8 fields",
                    ),
                );
            }
        }

        // Check value shape is correct for operator and field type
        if !self.operator.value_shape_ok(&self.value, property_type) {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::InvalidWhereClauseComponents("invalid value shape for operator"),
            );
        }

        // For Between variants, ensure bounds are in ascending order to avoid surprising matches
        match self.operator {
            Between | BetweenExcludeBounds | BetweenExcludeLeft | BetweenExcludeRight => {
                if let Value::Array(bounds) = &self.value {
                    if bounds.len() == 2 {
                        match bounds[0].partial_cmp(&bounds[1]) {
                            Some(Ordering::Less) | Some(Ordering::Equal) => {}
                            _ => {
                                return QuerySyntaxSimpleValidationResult::new_with_error(
                                    QuerySyntaxError::InvalidBetweenClause(
                                        "when using between operator bounds must be ascending",
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Check value type matches field type for equality and IN operators
        let value_type_matches = |prop_ty: &DocumentPropertyType, v: &Value| -> bool {
            use DocumentPropertyType as T;
            match prop_ty {
                T::String(_) => matches!(v, Value::Text(_)),
                T::Identifier => matches!(v, Value::Identifier(_)),
                T::Boolean => matches!(v, Value::Bool(_)),
                T::ByteArray(_) => matches!(v, Value::Bytes(_)),
                T::F64 => matches!(v, Value::Float(_)),
                T::Date => matches!(
                    v,
                    Value::U64(_)
                        | Value::I64(_)
                        | Value::U32(_)
                        | Value::I32(_)
                        | Value::U16(_)
                        | Value::I16(_)
                        | Value::U8(_)
                        | Value::I8(_)
                ),
                T::U8 | T::U16 | T::U32 | T::U64 | T::U128 => matches!(
                    v,
                    Value::U8(_) | Value::U16(_) | Value::U32(_) | Value::U64(_) | Value::U128(_)
                ),
                T::I8 | T::I16 | T::I32 | T::I64 | T::I128 => matches!(
                    v,
                    Value::I8(_) | Value::I16(_) | Value::I32(_) | Value::I64(_) | Value::I128(_)
                ),
                // No validation for object/array types as operators are disallowed
                T::Object(_) | T::Array(_) | T::VariableTypeArray(_) => false,
            }
        };

        // For equality, allow some type coercion (e.g. integer types)
        match self.operator {
            Equal => {
                use DocumentPropertyType as T;
                let ok = match property_type {
                    // Accept any integer-like value for integer fields (signed/unsigned), reject floats
                    T::U8
                    | T::U16
                    | T::U32
                    | T::U64
                    | T::U128
                    | T::I8
                    | T::I16
                    | T::I32
                    | T::I64
                    | T::I128 => {
                        matches!(
                            self.value,
                            Value::U128(_)
                                | Value::I128(_)
                                | Value::U64(_)
                                | Value::I64(_)
                                | Value::U32(_)
                                | Value::I32(_)
                                | Value::U16(_)
                                | Value::I16(_)
                                | Value::U8(_)
                                | Value::I8(_)
                        )
                    }
                    T::F64 => matches!(self.value, Value::Float(_)),
                    T::Date => matches!(
                        self.value,
                        Value::U64(_)
                            | Value::I64(_)
                            | Value::U32(_)
                            | Value::I32(_)
                            | Value::U16(_)
                            | Value::I16(_)
                            | Value::U8(_)
                            | Value::I8(_)
                    ),
                    T::String(_) => matches!(self.value, Value::Text(_)),
                    T::Identifier => matches!(self.value, Value::Identifier(_)),
                    T::ByteArray(_) => matches!(self.value, Value::Bytes(_)),
                    T::Boolean => matches!(self.value, Value::Bool(_)),
                    // Not applicable for object/array/variable arrays
                    T::Object(_) | T::Array(_) | T::VariableTypeArray(_) => false,
                };
                if !ok {
                    return QuerySyntaxSimpleValidationResult::new_with_error(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "invalid value type for equality",
                        ),
                    );
                }
            }
            In => {
                if let Value::Array(arr) = &self.value {
                    if !arr.iter().all(|v| value_type_matches(property_type, v)) {
                        return QuerySyntaxSimpleValidationResult::new_with_error(
                            QuerySyntaxError::InvalidWhereClauseComponents(
                                "invalid value type in IN clause",
                            ),
                        );
                    }
                }
            }
            _ => {}
        }

        QuerySyntaxSimpleValidationResult::new()
    }
}

impl From<WhereClause> for Value {
    fn from(value: WhereClause) -> Self {
        Value::Array(vec![value.field.into(), value.operator.into(), value.value])
    }
}

/// Value-only clause used when there is no field lookup involved
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueClause {
    /// Operator
    pub operator: WhereOperator,
    /// Value
    pub value: Value,
}

impl ValueClause {
    /// Evaluate this clause against a provided `Value`
    pub fn matches_value(&self, value: &Value) -> bool {
        self.operator.eval(value, &self.value)
    }
}

/// Returns the set of allowed operators for a given property type
#[cfg(any(feature = "server", feature = "verify"))]
pub fn allowed_ops_for_type(property_type: &DocumentPropertyType) -> &'static [WhereOperator] {
    match property_type {
        DocumentPropertyType::U8
        | DocumentPropertyType::I8
        | DocumentPropertyType::U16
        | DocumentPropertyType::I16
        | DocumentPropertyType::U32
        | DocumentPropertyType::I32
        | DocumentPropertyType::U64
        | DocumentPropertyType::I64
        | DocumentPropertyType::U128
        | DocumentPropertyType::I128
        | DocumentPropertyType::F64
        | DocumentPropertyType::Date => &[
            Equal,
            In,
            GreaterThan,
            GreaterThanOrEquals,
            LessThan,
            LessThanOrEquals,
            Between,
            BetweenExcludeBounds,
            BetweenExcludeLeft,
            BetweenExcludeRight,
        ],
        DocumentPropertyType::String(_) => &[
            Equal,
            In,
            StartsWith,
            GreaterThan,
            GreaterThanOrEquals,
            LessThan,
            LessThanOrEquals,
            Between,
            BetweenExcludeBounds,
            BetweenExcludeLeft,
            BetweenExcludeRight,
        ],
        DocumentPropertyType::Identifier => &[Equal, In],
        DocumentPropertyType::ByteArray(_) => &[Equal, In],
        DocumentPropertyType::Boolean => &[Equal],
        DocumentPropertyType::Object(_)
        | DocumentPropertyType::Array(_)
        | DocumentPropertyType::VariableTypeArray(_) => &[],
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
fn is_numeric_value(value: &Value) -> bool {
    matches!(
        value,
        Value::U128(_)
            | Value::I128(_)
            | Value::U64(_)
            | Value::I64(_)
            | Value::U32(_)
            | Value::I32(_)
            | Value::U16(_)
            | Value::I16(_)
            | Value::U8(_)
            | Value::I8(_)
            | Value::Float(_)
    )
}

/// Map known meta/system fields to their corresponding property types.
/// Meta fields are top-level and always start with `$`.
fn meta_field_property_type(field: &str) -> Option<DocumentPropertyType> {
    match field {
        // Identifiers
        "$id" | "$ownerId" | "$dataContractId" => Some(DocumentPropertyType::Identifier),
        // Dates (millis since epoch)
        "$createdAt" | "$updatedAt" | "$transferredAt" => Some(DocumentPropertyType::Date),
        // Block heights and core block heights
        "$createdAtBlockHeight" | "$updatedAtBlockHeight" | "$transferredAtBlockHeight" => {
            Some(DocumentPropertyType::U64)
        }
        "$createdAtCoreBlockHeight"
        | "$updatedAtCoreBlockHeight"
        | "$transferredAtCoreBlockHeight" => Some(DocumentPropertyType::U32),
        // Revision and protocol version are integers
        "$revision" | "$protocolVersion" => Some(DocumentPropertyType::U64),
        // Type name is a string
        "$type" => Some(DocumentPropertyType::String(
            dpp::data_contract::document_type::StringPropertySizes {
                min_length: None,
                max_length: None,
            },
        )),
        _ => None,
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::error::query::QuerySyntaxError;
    use crate::query::conditions::WhereClause;
    use crate::query::conditions::{
        Equal, GreaterThan, GreaterThanOrEquals, In, LessThan, LessThanOrEquals,
    };
    use crate::query::InternalClauses;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::platform_value::Value;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::LATEST_PLATFORM_VERSION;

    #[test]
    fn test_allowed_sup_query_pairs() {
        let allowed_pairs_test_cases = [
            [GreaterThan, LessThan],
            [GreaterThan, LessThanOrEquals],
            [GreaterThanOrEquals, LessThanOrEquals],
        ];
        for query_pair in allowed_pairs_test_cases {
            let where_clauses = vec![
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.first().unwrap(),
                    value: Value::Float(0.0),
                },
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.get(1).unwrap(),
                    value: Value::Float(1.0),
                },
            ];
            let (_, range_clause, _) = WhereClause::group_clauses(&where_clauses)
                .expect("expected to have groupable pair");
            range_clause.expect("expected to have range clause returned");
        }
    }

    #[test]
    fn test_allowed_inf_query_pairs() {
        let allowed_pairs_test_cases = [
            [LessThan, GreaterThan],
            [LessThan, GreaterThanOrEquals],
            [LessThanOrEquals, GreaterThanOrEquals],
        ];
        for query_pair in allowed_pairs_test_cases {
            let where_clauses = vec![
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.first().unwrap(),
                    value: Value::Float(1.0),
                },
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.get(1).unwrap(),
                    value: Value::Float(0.0),
                },
            ];
            let (_, range_clause, _) = WhereClause::group_clauses(&where_clauses)
                .expect("expected to have groupable pair");
            range_clause.expect("expected to have range clause returned");
        }
    }

    #[test]
    fn test_query_pairs_incoherent_same_value() {
        let allowed_pairs_test_cases = [[LessThan, GreaterThan], [GreaterThan, LessThan]];
        for query_pair in allowed_pairs_test_cases {
            let where_clauses = vec![
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.first().unwrap(),
                    value: Value::Float(1.0),
                },
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.get(1).unwrap(),
                    value: Value::Float(1.0),
                },
            ];
            WhereClause::group_clauses(&where_clauses)
                .expect_err("expected to have an error returned");
        }
    }

    #[test]
    fn test_different_fields_grouping_causes_error() {
        let where_clauses = vec![
            WhereClause {
                field: "a".to_string(),
                operator: LessThan,
                value: Value::Float(0.0),
            },
            WhereClause {
                field: "b".to_string(),
                operator: GreaterThan,
                value: Value::Float(1.0),
            },
        ];
        WhereClause::group_clauses(&where_clauses)
            .expect_err("different fields should not be groupable");
    }

    #[test]
    fn test_restricted_query_pairs_causes_error() {
        let restricted_pairs_test_cases = [
            [Equal, LessThan],
            [Equal, GreaterThan],
            [In, LessThan],
            [Equal, GreaterThan],
            [LessThanOrEquals, LessThanOrEquals],
            [LessThan, LessThan],
            [LessThan, LessThanOrEquals],
            [GreaterThan, GreaterThan],
            [GreaterThan, GreaterThanOrEquals],
            [GreaterThanOrEquals, GreaterThanOrEquals],
            [Equal, Equal],
        ];
        for query_pair in restricted_pairs_test_cases {
            let where_clauses = vec![
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.first().unwrap(),
                    value: Value::Float(0.0),
                },
                WhereClause {
                    field: "a".to_string(),
                    operator: *query_pair.get(1).unwrap(),
                    value: Value::Float(1.0),
                },
            ];
            WhereClause::group_clauses(&where_clauses)
                .expect_err("expected to not have a groupable pair");
        }
    }

    #[test]
    fn validate_rejects_equality_with_wrong_type_for_string_field() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "name".to_string(),
            operator: Equal,
            value: Value::Identifier([1u8; 32]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
        assert!(matches!(
            res.first_error(),
            Some(QuerySyntaxError::InvalidWhereClauseComponents(_))
        ));
    }

    #[test]
    fn validate_rejects_in_with_wrong_element_types() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("indexedDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "firstName".to_string(),
            operator: In,
            value: Value::Array(vec![
                Value::Text("alice".to_string()),
                Value::Identifier([2u8; 32]),
            ]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
        assert!(matches!(
            res.first_error(),
            Some(QuerySyntaxError::InvalidWhereClauseComponents(_))
        ));
    }

    #[test]
    fn validate_rejects_primary_key_in_with_non_identifiers() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let mut clauses = InternalClauses::default();
        clauses.primary_key_in_clause = Some(WhereClause {
            field: "$id".to_string(),
            operator: In,
            value: Value::Array(vec![
                Value::Text("a".to_string()),
                Value::Text("b".to_string()),
            ]),
        });

        let res = clauses.validate_against_schema(doc_type);
        assert!(res.is_err());
        assert!(matches!(
            res.first_error(),
            Some(QuerySyntaxError::InvalidWhereClauseComponents(_))
        ));
    }

    #[test]
    fn validate_rejects_date_with_float_equality() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("uniqueDates")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$createdAt".to_string(),
            operator: Equal,
            value: Value::Float(1.23),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
        assert!(matches!(
            res.first_error(),
            Some(QuerySyntaxError::InvalidWhereClauseComponents(_))
        ));
    }

    #[test]
    fn validate_rejects_in_bytes_for_string_field() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        // IN with Bytes should be rejected on string fields
        let clause = WhereClause {
            field: "name".to_string(),
            operator: In,
            value: Value::Bytes(vec![1, 2, 3]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
    }

    #[test]
    fn validate_accepts_meta_owner_id_in_identifiers() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$ownerId".to_string(),
            operator: In,
            value: Value::Array(vec![
                Value::Identifier([1u8; 32]),
                Value::Identifier([2u8; 32]),
            ]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_valid());
    }

    #[test]
    fn validate_accepts_meta_created_at_between_integers() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("uniqueDates")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$createdAt".to_string(),
            operator: crate::query::conditions::Between,
            value: Value::Array(vec![Value::U64(1000), Value::U64(2000)]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_valid());
    }

    #[test]
    fn validate_rejects_meta_revision_float_equality() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$revision".to_string(),
            operator: Equal,
            value: Value::Float(3.14),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
    }

    #[test]
    fn validate_accepts_meta_created_at_block_height_range() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("uniqueDates")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$createdAtBlockHeight".to_string(),
            operator: GreaterThanOrEquals,
            value: Value::U64(100),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_valid());
    }

    #[test]
    fn validate_accepts_meta_data_contract_id_equality() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$dataContractId".to_string(),
            operator: Equal,
            value: Value::Identifier([3u8; 32]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_valid());
    }
}
