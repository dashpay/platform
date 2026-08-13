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
                        Some(Ordering::Less) => {
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
                        Some(Ordering::Less) => left_value > &bounds[0] && left_value < &bounds[1],
                        _ => false,
                    }
                }
                _ => false,
            },
            BetweenExcludeLeft => match right_value {
                Value::Array(bounds) if bounds.len() == 2 => {
                    match bounds[0].partial_cmp(&bounds[1]) {
                        Some(Ordering::Less) => left_value > &bounds[0] && left_value <= &bounds[1],
                        _ => false,
                    }
                }
                _ => false,
            },
            BetweenExcludeRight => match right_value {
                Value::Array(bounds) if bounds.len() == 2 => {
                    match bounds[0].partial_cmp(&bounds[1]) {
                        Some(Ordering::Less) => left_value >= &bounds[0] && left_value < &bounds[1],
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

    /// Given a list of where clauses, returns them in groups of equal, range, and in clauses.
    ///
    /// Multiple `In` clauses (on distinct fields) are grouped structurally here;
    /// whether more than one is *accepted* is decided later, at path-query
    /// lowering, where the platform version is known (protocol version 14 is the
    /// first to accept them).
    #[allow(clippy::type_complexity)]
    pub(crate) fn group_clauses(
        where_clauses: &'a [WhereClause],
        // TODO: Define a type/struct for return value
    ) -> Result<(BTreeMap<String, Self>, Option<Self>, Vec<Self>), Error> {
        if where_clauses.is_empty() {
            return Ok((BTreeMap::new(), None, Vec::new()));
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

        let in_clauses = in_clauses_array
            .into_iter()
            .map(|clause| {
                if known_fields.contains(&clause.field) {
                    Err(Error::Query(
                        QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                            "in clause has same field as an equality or in clause",
                        ),
                    ))
                } else {
                    known_fields.insert(clause.field.clone());
                    Ok(clause)
                }
            })
            .collect::<Result<Vec<WhereClause>, Error>>()?;

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

        Ok((equal_clauses, range_clause, in_clauses))
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
                                    query.insert_range(starts_at_key..key);
                                } else {
                                    query.insert_range_after_to(starts_at_key..key);
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
                                    query.insert_range_inclusive(starts_at_key..=key);
                                } else {
                                    query.insert_range_after_to_inclusive(starts_at_key..=key);
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
                            Some(Ordering::Less) => {}
                            _ => {
                                return QuerySyntaxSimpleValidationResult::new_with_error(
                                    QuerySyntaxError::InvalidBetweenClause(
                                        "when using between operator bounds must be strictly ascending",
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
                T::Identifier | T::IdentifierWithReference(_) => matches!(v, Value::Identifier(_)),
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
                    T::Identifier | T::IdentifierWithReference(_) => {
                        matches!(self.value, Value::Identifier(_))
                    }
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
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            &[Equal, In]
        }
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
        "$id" | "$ownerId" | "$dataContractId" | "$creatorId" => {
            Some(DocumentPropertyType::Identifier)
        }
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
#[allow(clippy::approx_constant)]
mod tests {
    use crate::error::query::QuerySyntaxError;
    use crate::query::conditions::WhereClause;
    use crate::query::conditions::{
        Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, Equal, GreaterThan,
        GreaterThanOrEquals, In, LessThan, LessThanOrEquals, ValueClause,
    };
    use crate::query::InternalClauses;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::document::DocumentV0;
    use dpp::platform_value::Value;
    use dpp::prelude::Identifier;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::LATEST_PLATFORM_VERSION;
    use grovedb::Query;
    use std::collections::BTreeMap;

    fn cursor_document(field: &str, value: Value) -> dpp::document::Document {
        DocumentV0 {
            id: Identifier::from([3u8; 32]),
            owner_id: Identifier::from([4u8; 32]),
            properties: BTreeMap::from([(field.to_string(), value)]),
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into()
    }

    #[test]
    fn ascending_less_than_ranges_start_at_the_cursor() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type exists");
        let cursor_value = Value::Text("m".to_string());
        let upper_value = Value::Text("z".to_string());
        let cursor_key = document_type
            .serialize_value_for_key("name", &cursor_value, LATEST_PLATFORM_VERSION)
            .unwrap();
        let upper_key = document_type
            .serialize_value_for_key("name", &upper_value, LATEST_PLATFORM_VERSION)
            .unwrap();

        for (operator, cursor_included) in [
            (LessThan, true),
            (LessThan, false),
            (LessThanOrEquals, true),
            (LessThanOrEquals, false),
        ] {
            let clause = WhereClause {
                field: "name".to_string(),
                operator,
                value: upper_value.clone(),
            };
            let start_at = Some((
                cursor_document("name", cursor_value.clone()),
                cursor_included,
            ));
            let actual = clause
                .to_path_query(document_type, &start_at, true, LATEST_PLATFORM_VERSION)
                .unwrap();
            let mut expected = Query::new_with_direction(true);

            match (operator, cursor_included) {
                (LessThan, true) => expected.insert_range(cursor_key.clone()..upper_key.clone()),
                (LessThan, false) => {
                    expected.insert_range_after_to(cursor_key.clone()..upper_key.clone())
                }
                (LessThanOrEquals, true) => {
                    expected.insert_range_inclusive(cursor_key.clone()..=upper_key.clone())
                }
                (LessThanOrEquals, false) => {
                    expected.insert_range_after_to_inclusive(cursor_key.clone()..=upper_key.clone())
                }
                _ => unreachable!(),
            }

            assert_eq!(actual.items, expected.items);
        }
    }

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

        let clauses = InternalClauses {
            primary_key_in_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: In,
                value: Value::Array(vec![
                    Value::Text("a".to_string()),
                    Value::Text("b".to_string()),
                ]),
            }),
            ..Default::default()
        };

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
    fn validate_rejects_between_variants_with_equal_bounds() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("uniqueDates")
            .expect("doc type exists");

        for operator in [
            Between,
            BetweenExcludeBounds,
            BetweenExcludeLeft,
            BetweenExcludeRight,
        ] {
            let clause = WhereClause {
                field: "$createdAt".to_string(),
                operator,
                value: Value::Array(vec![Value::U64(1000), Value::U64(1000)]),
            };

            let res = clause.validate_against_schema(doc_type);
            assert!(
                res.is_err(),
                "{operator:?} should reject equal bounds during validation"
            );
            assert!(matches!(
                res.first_error(),
                Some(QuerySyntaxError::InvalidBetweenClause(_))
            ));
        }
    }

    #[test]
    fn value_clause_between_variants_do_not_match_equal_bounds() {
        let equal_bounds = Value::Array(vec![Value::U64(1000), Value::U64(1000)]);
        let value_to_test = Value::U64(1000);

        for operator in [
            Between,
            BetweenExcludeBounds,
            BetweenExcludeLeft,
            BetweenExcludeRight,
        ] {
            let clause = ValueClause {
                operator,
                value: equal_bounds.clone(),
            };

            assert!(
                !clause.matches_value(&value_to_test),
                "{operator:?} should not match when bounds are equal"
            );
        }
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
            value: Value::Float(3.15),
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

    // ---- WhereOperator::allows_flip ----

    #[test]
    fn allows_flip_returns_true_for_comparison_operators() {
        assert!(Equal.allows_flip());
        assert!(GreaterThan.allows_flip());
        assert!(GreaterThanOrEquals.allows_flip());
        assert!(LessThan.allows_flip());
        assert!(LessThanOrEquals.allows_flip());
    }

    #[test]
    fn allows_flip_returns_false_for_non_flippable_operators() {
        assert!(!Between.allows_flip());
        assert!(!BetweenExcludeBounds.allows_flip());
        assert!(!BetweenExcludeLeft.allows_flip());
        assert!(!BetweenExcludeRight.allows_flip());
        assert!(!In.allows_flip());
        assert!(!super::StartsWith.allows_flip());
    }

    // ---- WhereOperator::flip ----

    #[test]
    fn flip_equal_stays_equal() {
        assert_eq!(Equal.flip().unwrap(), Equal);
    }

    #[test]
    fn flip_greater_than_becomes_less_than() {
        assert_eq!(GreaterThan.flip().unwrap(), LessThan);
    }

    #[test]
    fn flip_greater_than_or_equals_becomes_less_than_or_equals() {
        assert_eq!(GreaterThanOrEquals.flip().unwrap(), LessThanOrEquals);
    }

    #[test]
    fn flip_less_than_becomes_greater_than() {
        assert_eq!(LessThan.flip().unwrap(), GreaterThan);
    }

    #[test]
    fn flip_less_than_or_equals_becomes_greater_than_or_equals() {
        assert_eq!(LessThanOrEquals.flip().unwrap(), GreaterThanOrEquals);
    }

    #[test]
    fn flip_between_returns_error() {
        assert!(Between.flip().is_err());
    }

    #[test]
    fn flip_between_exclude_bounds_returns_error() {
        assert!(BetweenExcludeBounds.flip().is_err());
    }

    #[test]
    fn flip_between_exclude_left_returns_error() {
        assert!(BetweenExcludeLeft.flip().is_err());
    }

    #[test]
    fn flip_between_exclude_right_returns_error() {
        assert!(BetweenExcludeRight.flip().is_err());
    }

    #[test]
    fn flip_in_returns_error() {
        assert!(In.flip().is_err());
    }

    #[test]
    fn flip_starts_with_returns_error() {
        assert!(super::StartsWith.flip().is_err());
    }

    // ---- WhereOperator::is_range ----

    #[test]
    fn is_range_false_for_equal() {
        assert!(!Equal.is_range());
    }

    #[test]
    fn is_range_true_for_all_range_operators() {
        assert!(GreaterThan.is_range());
        assert!(GreaterThanOrEquals.is_range());
        assert!(LessThan.is_range());
        assert!(LessThanOrEquals.is_range());
        assert!(Between.is_range());
        assert!(BetweenExcludeBounds.is_range());
        assert!(BetweenExcludeLeft.is_range());
        assert!(BetweenExcludeRight.is_range());
        assert!(In.is_range());
        assert!(super::StartsWith.is_range());
    }

    // ---- WhereOperator::from_string ----

    #[test]
    fn from_string_parses_equality_operators() {
        use super::WhereOperator;
        assert_eq!(WhereOperator::from_string("="), Some(Equal));
        assert_eq!(WhereOperator::from_string("=="), Some(Equal));
    }

    #[test]
    fn from_string_parses_comparison_operators() {
        use super::WhereOperator;
        assert_eq!(WhereOperator::from_string(">"), Some(GreaterThan));
        assert_eq!(WhereOperator::from_string(">="), Some(GreaterThanOrEquals));
        assert_eq!(WhereOperator::from_string("<"), Some(LessThan));
        assert_eq!(WhereOperator::from_string("<="), Some(LessThanOrEquals));
    }

    #[test]
    fn from_string_parses_between_variants() {
        use super::WhereOperator;
        assert_eq!(WhereOperator::from_string("Between"), Some(Between));
        assert_eq!(WhereOperator::from_string("between"), Some(Between));
        assert_eq!(
            WhereOperator::from_string("BetweenExcludeBounds"),
            Some(BetweenExcludeBounds)
        );
        assert_eq!(
            WhereOperator::from_string("betweenExcludeBounds"),
            Some(BetweenExcludeBounds)
        );
        assert_eq!(
            WhereOperator::from_string("betweenexcludebounds"),
            Some(BetweenExcludeBounds)
        );
        assert_eq!(
            WhereOperator::from_string("between_exclude_bounds"),
            Some(BetweenExcludeBounds)
        );
        assert_eq!(
            WhereOperator::from_string("BetweenExcludeLeft"),
            Some(BetweenExcludeLeft)
        );
        assert_eq!(
            WhereOperator::from_string("betweenExcludeLeft"),
            Some(BetweenExcludeLeft)
        );
        assert_eq!(
            WhereOperator::from_string("betweenexcludeleft"),
            Some(BetweenExcludeLeft)
        );
        assert_eq!(
            WhereOperator::from_string("between_exclude_left"),
            Some(BetweenExcludeLeft)
        );
        assert_eq!(
            WhereOperator::from_string("BetweenExcludeRight"),
            Some(BetweenExcludeRight)
        );
        assert_eq!(
            WhereOperator::from_string("betweenExcludeRight"),
            Some(BetweenExcludeRight)
        );
        assert_eq!(
            WhereOperator::from_string("betweenexcluderight"),
            Some(BetweenExcludeRight)
        );
        assert_eq!(
            WhereOperator::from_string("between_exclude_right"),
            Some(BetweenExcludeRight)
        );
    }

    #[test]
    fn from_string_parses_in_operator() {
        use super::WhereOperator;
        assert_eq!(WhereOperator::from_string("In"), Some(In));
        assert_eq!(WhereOperator::from_string("in"), Some(In));
    }

    #[test]
    fn from_string_parses_starts_with_operator() {
        use super::WhereOperator;
        assert_eq!(
            WhereOperator::from_string("StartsWith"),
            Some(super::StartsWith)
        );
        assert_eq!(
            WhereOperator::from_string("startsWith"),
            Some(super::StartsWith)
        );
        assert_eq!(
            WhereOperator::from_string("startswith"),
            Some(super::StartsWith)
        );
        assert_eq!(
            WhereOperator::from_string("starts_with"),
            Some(super::StartsWith)
        );
    }

    #[test]
    fn from_string_returns_none_for_unknown() {
        use super::WhereOperator;
        assert_eq!(WhereOperator::from_string("LIKE"), None);
        assert_eq!(WhereOperator::from_string("!="), None);
        assert_eq!(WhereOperator::from_string(""), None);
    }

    // ---- WhereOperator::from_sql_operator ----

    #[test]
    fn from_sql_operator_maps_known_operators() {
        use super::WhereOperator;
        use sqlparser::ast::BinaryOperator;
        assert_eq!(
            WhereOperator::from_sql_operator(BinaryOperator::Eq),
            Some(Equal)
        );
        assert_eq!(
            WhereOperator::from_sql_operator(BinaryOperator::Gt),
            Some(GreaterThan)
        );
        assert_eq!(
            WhereOperator::from_sql_operator(BinaryOperator::GtEq),
            Some(GreaterThanOrEquals)
        );
        assert_eq!(
            WhereOperator::from_sql_operator(BinaryOperator::Lt),
            Some(LessThan)
        );
        assert_eq!(
            WhereOperator::from_sql_operator(BinaryOperator::LtEq),
            Some(LessThanOrEquals)
        );
    }

    #[test]
    fn from_sql_operator_returns_none_for_unsupported() {
        use super::WhereOperator;
        use sqlparser::ast::BinaryOperator;
        assert_eq!(
            WhereOperator::from_sql_operator(BinaryOperator::NotEq),
            None
        );
        assert_eq!(WhereOperator::from_sql_operator(BinaryOperator::Plus), None);
    }

    // ---- WhereOperator::eval ----

    #[test]
    fn eval_equal_matches_identical_values() {
        assert!(Equal.eval(&Value::I64(42), &Value::I64(42)));
        assert!(!Equal.eval(&Value::I64(42), &Value::I64(43)));
    }

    #[test]
    fn eval_greater_than() {
        assert!(GreaterThan.eval(&Value::I64(10), &Value::I64(5)));
        assert!(!GreaterThan.eval(&Value::I64(5), &Value::I64(10)));
        assert!(!GreaterThan.eval(&Value::I64(5), &Value::I64(5)));
    }

    #[test]
    fn eval_greater_than_or_equals() {
        assert!(GreaterThanOrEquals.eval(&Value::I64(10), &Value::I64(5)));
        assert!(GreaterThanOrEquals.eval(&Value::I64(5), &Value::I64(5)));
        assert!(!GreaterThanOrEquals.eval(&Value::I64(4), &Value::I64(5)));
    }

    #[test]
    fn eval_less_than() {
        assert!(LessThan.eval(&Value::I64(3), &Value::I64(5)));
        assert!(!LessThan.eval(&Value::I64(5), &Value::I64(3)));
        assert!(!LessThan.eval(&Value::I64(5), &Value::I64(5)));
    }

    #[test]
    fn eval_less_than_or_equals() {
        assert!(LessThanOrEquals.eval(&Value::I64(3), &Value::I64(5)));
        assert!(LessThanOrEquals.eval(&Value::I64(5), &Value::I64(5)));
        assert!(!LessThanOrEquals.eval(&Value::I64(6), &Value::I64(5)));
    }

    #[test]
    fn eval_in_with_array() {
        let arr = Value::Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        assert!(In.eval(&Value::I64(2), &arr));
        assert!(!In.eval(&Value::I64(4), &arr));
    }

    #[test]
    fn eval_in_with_bytes() {
        let bytes = Value::Bytes(vec![10, 20, 30]);
        assert!(In.eval(&Value::U8(20), &bytes));
        assert!(!In.eval(&Value::U8(40), &bytes));
        // Non-U8 value against Bytes should return false
        assert!(!In.eval(&Value::I64(20), &bytes));
    }

    #[test]
    fn eval_in_with_non_collection_returns_false() {
        assert!(!In.eval(&Value::I64(1), &Value::I64(1)));
    }

    #[test]
    fn eval_between_inclusive() {
        let bounds = Value::Array(vec![Value::I64(10), Value::I64(20)]);
        assert!(Between.eval(&Value::I64(10), &bounds));
        assert!(Between.eval(&Value::I64(15), &bounds));
        assert!(Between.eval(&Value::I64(20), &bounds));
        assert!(!Between.eval(&Value::I64(9), &bounds));
        assert!(!Between.eval(&Value::I64(21), &bounds));
    }

    #[test]
    fn eval_between_exclude_bounds() {
        let bounds = Value::Array(vec![Value::I64(10), Value::I64(20)]);
        assert!(!BetweenExcludeBounds.eval(&Value::I64(10), &bounds));
        assert!(BetweenExcludeBounds.eval(&Value::I64(15), &bounds));
        assert!(!BetweenExcludeBounds.eval(&Value::I64(20), &bounds));
    }

    #[test]
    fn eval_between_exclude_left() {
        let bounds = Value::Array(vec![Value::I64(10), Value::I64(20)]);
        assert!(!BetweenExcludeLeft.eval(&Value::I64(10), &bounds));
        assert!(BetweenExcludeLeft.eval(&Value::I64(15), &bounds));
        assert!(BetweenExcludeLeft.eval(&Value::I64(20), &bounds));
    }

    #[test]
    fn eval_between_exclude_right() {
        let bounds = Value::Array(vec![Value::I64(10), Value::I64(20)]);
        assert!(BetweenExcludeRight.eval(&Value::I64(10), &bounds));
        assert!(BetweenExcludeRight.eval(&Value::I64(15), &bounds));
        assert!(!BetweenExcludeRight.eval(&Value::I64(20), &bounds));
    }

    #[test]
    fn eval_between_with_wrong_bound_order_returns_false() {
        // Bounds in descending order should not match anything
        let bounds = Value::Array(vec![Value::I64(20), Value::I64(10)]);
        assert!(!Between.eval(&Value::I64(15), &bounds));
        assert!(!BetweenExcludeBounds.eval(&Value::I64(15), &bounds));
        assert!(!BetweenExcludeLeft.eval(&Value::I64(15), &bounds));
        assert!(!BetweenExcludeRight.eval(&Value::I64(15), &bounds));
    }

    #[test]
    fn eval_between_with_non_array_returns_false() {
        assert!(!Between.eval(&Value::I64(5), &Value::I64(10)));
    }

    #[test]
    fn eval_between_with_wrong_array_len_returns_false() {
        let single = Value::Array(vec![Value::I64(10)]);
        assert!(!Between.eval(&Value::I64(10), &single));
    }

    #[test]
    fn eval_starts_with_text() {
        assert!(super::StartsWith.eval(
            &Value::Text("hello world".to_string()),
            &Value::Text("hello".to_string())
        ));
        assert!(!super::StartsWith.eval(
            &Value::Text("hello world".to_string()),
            &Value::Text("world".to_string())
        ));
    }

    #[test]
    fn eval_starts_with_non_text_returns_false() {
        assert!(!super::StartsWith.eval(&Value::I64(123), &Value::Text("1".to_string())));
        assert!(!super::StartsWith.eval(&Value::Text("hello".to_string()), &Value::I64(1)));
    }

    // ---- WhereOperator Display ----

    #[test]
    fn display_formatting_for_all_operators() {
        assert_eq!(format!("{}", Equal), "=");
        assert_eq!(format!("{}", GreaterThan), ">");
        assert_eq!(format!("{}", GreaterThanOrEquals), ">=");
        assert_eq!(format!("{}", LessThan), "<");
        assert_eq!(format!("{}", LessThanOrEquals), "<=");
        assert_eq!(format!("{}", Between), "Between");
        assert_eq!(format!("{}", BetweenExcludeBounds), "BetweenExcludeBounds");
        assert_eq!(format!("{}", BetweenExcludeLeft), "BetweenExcludeLeft");
        assert_eq!(format!("{}", BetweenExcludeRight), "BetweenExcludeRight");
        assert_eq!(format!("{}", In), "In");
        assert_eq!(format!("{}", super::StartsWith), "StartsWith");
    }

    // ---- WhereOperator -> Value conversion ----

    #[test]
    fn where_operator_into_value() {
        let val: Value = Equal.into();
        assert_eq!(val, Value::Text("=".to_string()));

        let val: Value = In.into();
        assert_eq!(val, Value::Text("In".to_string()));
    }

    // ---- WhereClause::is_identifier ----

    #[test]
    fn is_identifier_returns_true_for_dollar_id() {
        let clause = WhereClause {
            field: "$id".to_string(),
            operator: Equal,
            value: Value::I64(1),
        };
        assert!(clause.is_identifier());
    }

    #[test]
    fn is_identifier_returns_false_for_other_fields() {
        let clause = WhereClause {
            field: "name".to_string(),
            operator: Equal,
            value: Value::I64(1),
        };
        assert!(!clause.is_identifier());

        let clause = WhereClause {
            field: "$ownerId".to_string(),
            operator: Equal,
            value: Value::I64(1),
        };
        assert!(!clause.is_identifier());
    }

    // ---- WhereClause::in_values ----

    #[test]
    fn in_values_with_array() {
        let clause = WhereClause {
            field: "f".to_string(),
            operator: In,
            value: Value::Array(vec![Value::I64(1), Value::I64(2)]),
        };
        let result = clause.in_values();
        assert!(result.is_valid());
        let data = result.into_data().expect("should have data");
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn in_values_with_bytes() {
        let clause = WhereClause {
            field: "f".to_string(),
            operator: In,
            value: Value::Bytes(vec![10, 20]),
        };
        let result = clause.in_values();
        assert!(result.is_valid());
        let data = result.into_data().expect("should have data");
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], Value::U8(10));
        assert_eq!(data[1], Value::U8(20));
    }

    #[test]
    fn in_values_non_array_returns_error() {
        let clause = WhereClause {
            field: "f".to_string(),
            operator: In,
            value: Value::I64(42),
        };
        let result = clause.in_values();
        assert!(!result.is_valid());
    }

    #[test]
    fn in_values_empty_array_returns_error() {
        let clause = WhereClause {
            field: "f".to_string(),
            operator: In,
            value: Value::Array(vec![]),
        };
        let result = clause.in_values();
        assert!(!result.is_valid());
    }

    #[test]
    fn in_values_too_many_returns_error() {
        let values: Vec<Value> = (0..101).map(Value::I64).collect();
        let clause = WhereClause {
            field: "f".to_string(),
            operator: In,
            value: Value::Array(values),
        };
        let result = clause.in_values();
        assert!(!result.is_valid());
    }

    #[test]
    fn in_values_with_duplicates_returns_error() {
        let clause = WhereClause {
            field: "f".to_string(),
            operator: In,
            value: Value::Array(vec![Value::I64(1), Value::I64(1)]),
        };
        let result = clause.in_values();
        assert!(!result.is_valid());
    }

    // ---- WhereClause::less_than ----

    #[test]
    fn less_than_with_i128_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I128(5),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I128(10),
        };
        assert!(a.less_than(&b, false).unwrap());
        assert!(a.less_than(&b, true).unwrap());
        assert!(!b.less_than(&a, false).unwrap());
        assert!(a.less_than(&a, true).unwrap()); // le
        assert!(!a.less_than(&a, false).unwrap()); // lt
    }

    #[test]
    fn less_than_with_u128_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U128(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U128(2),
        };
        assert!(a.less_than(&b, false).unwrap());
        assert!(!b.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_with_i64_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I64(-5),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I64(10),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_u64_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U64(3),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U64(7),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_i32_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I32(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I32(2),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_u32_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U32(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U32(2),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_i16_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I16(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I16(2),
        };
        assert!(a.less_than(&b, false).unwrap());
        assert!(a.less_than(&b, true).unwrap());
    }

    #[test]
    fn less_than_with_u16_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U16(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U16(2),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_i8_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I8(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I8(2),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_u8_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U8(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U8(2),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_bytes_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Bytes(vec![1, 2]),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Bytes(vec![1, 3]),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_float_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Float(1.5),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Float(2.5),
        };
        assert!(a.less_than(&b, false).unwrap());
        assert!(a.less_than(&b, true).unwrap());
    }

    #[test]
    fn less_than_with_text_values() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Text("abc".to_string()),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Text("xyz".to_string()),
        };
        assert!(a.less_than(&b, false).unwrap());
    }

    #[test]
    fn less_than_with_mismatched_types_returns_error() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I64(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Text("abc".to_string()),
        };
        assert!(a.less_than(&b, false).is_err());
    }

    // ---- WhereClause::from_components ----

    #[test]
    fn from_components_valid_clause() {
        let components = vec![
            Value::Text("name".to_string()),
            Value::Text("=".to_string()),
            Value::Text("alice".to_string()),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.field, "name");
        assert_eq!(clause.operator, Equal);
        assert_eq!(clause.value, Value::Text("alice".to_string()));
    }

    #[test]
    fn from_components_wrong_count_returns_error() {
        let components = vec![
            Value::Text("name".to_string()),
            Value::Text("=".to_string()),
        ];
        assert!(WhereClause::from_components(&components).is_err());

        let components = vec![
            Value::Text("name".to_string()),
            Value::Text("=".to_string()),
            Value::I64(1),
            Value::I64(2),
        ];
        assert!(WhereClause::from_components(&components).is_err());
    }

    #[test]
    fn from_components_non_string_field_returns_error() {
        let components = vec![Value::I64(123), Value::Text("=".to_string()), Value::I64(1)];
        assert!(WhereClause::from_components(&components).is_err());
    }

    #[test]
    fn from_components_non_string_operator_returns_error() {
        let components = vec![
            Value::Text("name".to_string()),
            Value::I64(1),
            Value::I64(1),
        ];
        assert!(WhereClause::from_components(&components).is_err());
    }

    #[test]
    fn from_components_unknown_operator_returns_error() {
        let components = vec![
            Value::Text("name".to_string()),
            Value::Text("LIKE".to_string()),
            Value::I64(1),
        ];
        assert!(WhereClause::from_components(&components).is_err());
    }

    #[test]
    fn from_components_with_in_operator() {
        let components = vec![
            Value::Text("status".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::I64(1), Value::I64(2)]),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, In);
    }

    #[test]
    fn from_components_with_starts_with_operator() {
        let components = vec![
            Value::Text("name".to_string()),
            Value::Text("startsWith".to_string()),
            Value::Text("alice".to_string()),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, super::StartsWith);
    }

    // ---- WhereClause -> Value conversion ----

    #[test]
    fn where_clause_into_value() {
        let clause = WhereClause {
            field: "name".to_string(),
            operator: Equal,
            value: Value::Text("alice".to_string()),
        };
        let val: Value = clause.into();
        match val {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], Value::Text("name".to_string()));
                assert_eq!(arr[1], Value::Text("=".to_string()));
                assert_eq!(arr[2], Value::Text("alice".to_string()));
            }
            _ => panic!("expected Array"),
        }
    }

    // ---- ValueClause::matches_value ----

    #[test]
    fn value_clause_matches_value_equal() {
        let clause = ValueClause {
            operator: Equal,
            value: Value::I64(42),
        };
        assert!(clause.matches_value(&Value::I64(42)));
        assert!(!clause.matches_value(&Value::I64(43)));
    }

    #[test]
    fn value_clause_matches_value_greater_than() {
        let clause = ValueClause {
            operator: GreaterThan,
            value: Value::I64(10),
        };
        assert!(clause.matches_value(&Value::I64(20)));
        assert!(!clause.matches_value(&Value::I64(5)));
    }

    #[test]
    fn value_clause_matches_value_in() {
        let clause = ValueClause {
            operator: In,
            value: Value::Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]),
        };
        assert!(clause.matches_value(&Value::I64(2)));
        assert!(!clause.matches_value(&Value::I64(4)));
    }

    #[test]
    fn value_clause_matches_value_starts_with() {
        let clause = ValueClause {
            operator: super::StartsWith,
            value: Value::Text("hello".to_string()),
        };
        assert!(clause.matches_value(&Value::Text("hello world".to_string())));
        assert!(!clause.matches_value(&Value::Text("world hello".to_string())));
    }

    // ---- WhereClause::matches_value ----

    #[test]
    fn where_clause_matches_value_delegates_to_eval() {
        let clause = WhereClause {
            field: "age".to_string(),
            operator: GreaterThanOrEquals,
            value: Value::I64(18),
        };
        assert!(clause.matches_value(&Value::I64(18)));
        assert!(clause.matches_value(&Value::I64(25)));
        assert!(!clause.matches_value(&Value::I64(17)));
    }

    // ---- group_clauses: additional coverage ----

    #[test]
    fn group_clauses_empty_input() {
        let clauses: Vec<WhereClause> = vec![];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).expect("empty should succeed");
        assert!(eq.is_empty());
        assert!(range.is_none());
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_single_equality() {
        let clauses = vec![WhereClause {
            field: "name".to_string(),
            operator: Equal,
            value: Value::Text("alice".to_string()),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert_eq!(eq.len(), 1);
        assert!(eq.contains_key("name"));
        assert!(range.is_none());
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_equality_on_id_is_excluded_from_equals() {
        let clauses = vec![WhereClause {
            field: "$id".to_string(),
            operator: Equal,
            value: Value::I64(1),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        // $id equality is excluded from the equal_clauses map
        assert!(eq.is_empty());
        assert!(range.is_none());
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_in_on_id_is_excluded_from_in_clause() {
        let clauses = vec![WhereClause {
            field: "$id".to_string(),
            operator: In,
            value: Value::Array(vec![Value::I64(1), Value::I64(2)]),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert!(eq.is_empty());
        assert!(range.is_none());
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_single_in() {
        let clauses = vec![WhereClause {
            field: "status".to_string(),
            operator: In,
            value: Value::Array(vec![Value::I64(1), Value::I64(2)]),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert!(eq.is_empty());
        assert!(range.is_none());
        assert_eq!(in_c.len(), 1);
        assert_eq!(in_c[0].field, "status");
    }

    #[test]
    fn group_clauses_multiple_in_on_distinct_fields_groups_structurally() {
        // Whether more than one in clause is accepted is decided at
        // path-query lowering (protocol version 14+); the grammar groups
        // them structurally in query order.
        let clauses = vec![
            WhereClause {
                field: "a".to_string(),
                operator: In,
                value: Value::Array(vec![Value::I64(1)]),
            },
            WhereClause {
                field: "b".to_string(),
                operator: In,
                value: Value::Array(vec![Value::I64(2)]),
            },
        ];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert!(eq.is_empty());
        assert!(range.is_none());
        assert_eq!(in_c.len(), 2);
        assert_eq!(in_c[0].field, "a");
        assert_eq!(in_c[1].field, "b");
    }

    #[test]
    fn group_clauses_multiple_in_on_same_field_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "a".to_string(),
                operator: In,
                value: Value::Array(vec![Value::I64(1)]),
            },
            WhereClause {
                field: "a".to_string(),
                operator: In,
                value: Value::Array(vec![Value::I64(2)]),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_in_same_field_as_equality_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "status".to_string(),
                operator: Equal,
                value: Value::I64(1),
            },
            WhereClause {
                field: "status".to_string(),
                operator: In,
                value: Value::Array(vec![Value::I64(2)]),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_duplicate_equality_same_field_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "name".to_string(),
                operator: Equal,
                value: Value::Text("alice".to_string()),
            },
            WhereClause {
                field: "name".to_string(),
                operator: Equal,
                value: Value::Text("bob".to_string()),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_single_range_operator() {
        let clauses = vec![WhereClause {
            field: "age".to_string(),
            operator: GreaterThan,
            value: Value::I64(18),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert!(eq.is_empty());
        assert!(range.is_some());
        assert_eq!(range.unwrap().operator, GreaterThan);
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_single_non_groupable_range_between() {
        let clauses = vec![WhereClause {
            field: "age".to_string(),
            operator: Between,
            value: Value::Array(vec![Value::Float(0.0), Value::Float(100.0)]),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert!(eq.is_empty());
        assert!(range.is_some());
        assert_eq!(range.unwrap().operator, Between);
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_starts_with_empty_string_returns_error() {
        let clauses = vec![WhereClause {
            field: "name".to_string(),
            operator: super::StartsWith,
            value: Value::Text("".to_string()),
        }];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_starts_with_valid_string() {
        let clauses = vec![WhereClause {
            field: "name".to_string(),
            operator: super::StartsWith,
            value: Value::Text("al".to_string()),
        }];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert!(eq.is_empty());
        assert!(range.is_some());
        assert_eq!(range.unwrap().operator, super::StartsWith);
        assert!(in_c.is_empty());
    }

    #[test]
    fn group_clauses_non_groupable_range_same_field_as_equality_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "name".to_string(),
                operator: Equal,
                value: Value::Text("alice".to_string()),
            },
            WhereClause {
                field: "name".to_string(),
                operator: super::StartsWith,
                value: Value::Text("al".to_string()),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_multiple_non_groupable_ranges_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "a".to_string(),
                operator: Between,
                value: Value::Array(vec![Value::Float(0.0), Value::Float(10.0)]),
            },
            WhereClause {
                field: "b".to_string(),
                operator: super::StartsWith,
                value: Value::Text("x".to_string()),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_mixed_groupable_and_non_groupable_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "a".to_string(),
                operator: GreaterThan,
                value: Value::Float(0.0),
            },
            WhereClause {
                field: "b".to_string(),
                operator: Between,
                value: Value::Array(vec![Value::Float(0.0), Value::Float(10.0)]),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_three_groupable_ranges_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "a".to_string(),
                operator: GreaterThan,
                value: Value::Float(0.0),
            },
            WhereClause {
                field: "a".to_string(),
                operator: LessThan,
                value: Value::Float(10.0),
            },
            WhereClause {
                field: "a".to_string(),
                operator: GreaterThanOrEquals,
                value: Value::Float(5.0),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_range_same_field_as_equality_returns_error() {
        let clauses = vec![
            WhereClause {
                field: "age".to_string(),
                operator: Equal,
                value: Value::I64(25),
            },
            WhereClause {
                field: "age".to_string(),
                operator: GreaterThan,
                value: Value::I64(18),
            },
        ];
        assert!(WhereClause::group_clauses(&clauses).is_err());
    }

    #[test]
    fn group_clauses_two_ranges_combined_into_between() {
        let clauses = vec![
            WhereClause {
                field: "age".to_string(),
                operator: GreaterThanOrEquals,
                value: Value::Float(10.0),
            },
            WhereClause {
                field: "age".to_string(),
                operator: LessThanOrEquals,
                value: Value::Float(20.0),
            },
        ];
        let (_, range, _) = WhereClause::group_clauses(&clauses).unwrap();
        let r = range.unwrap();
        assert_eq!(r.operator, Between);
        assert_eq!(r.field, "age");
    }

    #[test]
    fn group_clauses_two_ranges_combined_into_between_exclude_right() {
        let clauses = vec![
            WhereClause {
                field: "age".to_string(),
                operator: GreaterThanOrEquals,
                value: Value::Float(10.0),
            },
            WhereClause {
                field: "age".to_string(),
                operator: LessThan,
                value: Value::Float(20.0),
            },
        ];
        let (_, range, _) = WhereClause::group_clauses(&clauses).unwrap();
        assert_eq!(range.unwrap().operator, BetweenExcludeRight);
    }

    #[test]
    fn group_clauses_two_ranges_combined_into_between_exclude_left() {
        let clauses = vec![
            WhereClause {
                field: "age".to_string(),
                operator: GreaterThan,
                value: Value::Float(10.0),
            },
            WhereClause {
                field: "age".to_string(),
                operator: LessThanOrEquals,
                value: Value::Float(20.0),
            },
        ];
        let (_, range, _) = WhereClause::group_clauses(&clauses).unwrap();
        assert_eq!(range.unwrap().operator, BetweenExcludeLeft);
    }

    #[test]
    fn group_clauses_two_ranges_combined_into_between_exclude_bounds() {
        let clauses = vec![
            WhereClause {
                field: "age".to_string(),
                operator: GreaterThan,
                value: Value::Float(10.0),
            },
            WhereClause {
                field: "age".to_string(),
                operator: LessThan,
                value: Value::Float(20.0),
            },
        ];
        let (_, range, _) = WhereClause::group_clauses(&clauses).unwrap();
        assert_eq!(range.unwrap().operator, BetweenExcludeBounds);
    }

    #[test]
    fn group_clauses_equality_plus_in_on_different_fields() {
        let clauses = vec![
            WhereClause {
                field: "name".to_string(),
                operator: Equal,
                value: Value::Text("alice".to_string()),
            },
            WhereClause {
                field: "status".to_string(),
                operator: In,
                value: Value::Array(vec![Value::I64(1), Value::I64(2)]),
            },
        ];
        let (eq, _, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert_eq!(eq.len(), 1);
        assert_eq!(in_c.len(), 1);
    }

    #[test]
    fn group_clauses_equality_plus_range_on_different_fields() {
        let clauses = vec![
            WhereClause {
                field: "name".to_string(),
                operator: Equal,
                value: Value::Text("alice".to_string()),
            },
            WhereClause {
                field: "age".to_string(),
                operator: GreaterThan,
                value: Value::Float(18.0),
            },
        ];
        let (eq, range, in_c) = WhereClause::group_clauses(&clauses).unwrap();
        assert_eq!(eq.len(), 1);
        assert!(range.is_some());
        assert!(in_c.is_empty());
    }

    // ---- meta_field_property_type ----

    #[test]
    fn meta_field_property_type_all_identifiers() {
        use super::meta_field_property_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        for field in ["$id", "$ownerId", "$dataContractId", "$creatorId"] {
            let pt = meta_field_property_type(field);
            assert!(
                matches!(
                    pt,
                    Some(
                        DocumentPropertyType::Identifier
                            | DocumentPropertyType::IdentifierWithReference(_)
                    )
                ),
                "expected Identifier for {field}"
            );
        }
    }

    #[test]
    fn meta_field_property_type_dates() {
        use super::meta_field_property_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        for field in ["$createdAt", "$updatedAt", "$transferredAt"] {
            let pt = meta_field_property_type(field);
            assert!(
                matches!(pt, Some(DocumentPropertyType::Date)),
                "expected Date for {field}"
            );
        }
    }

    #[test]
    fn meta_field_property_type_block_heights() {
        use super::meta_field_property_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        for field in [
            "$createdAtBlockHeight",
            "$updatedAtBlockHeight",
            "$transferredAtBlockHeight",
        ] {
            let pt = meta_field_property_type(field);
            assert!(
                matches!(pt, Some(DocumentPropertyType::U64)),
                "expected U64 for {field}"
            );
        }
    }

    #[test]
    fn meta_field_property_type_core_block_heights() {
        use super::meta_field_property_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        for field in [
            "$createdAtCoreBlockHeight",
            "$updatedAtCoreBlockHeight",
            "$transferredAtCoreBlockHeight",
        ] {
            let pt = meta_field_property_type(field);
            assert!(
                matches!(pt, Some(DocumentPropertyType::U32)),
                "expected U32 for {field}"
            );
        }
    }

    #[test]
    fn meta_field_property_type_revision_and_protocol_version() {
        use super::meta_field_property_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(matches!(
            meta_field_property_type("$revision"),
            Some(DocumentPropertyType::U64)
        ));
        assert!(matches!(
            meta_field_property_type("$protocolVersion"),
            Some(DocumentPropertyType::U64)
        ));
    }

    #[test]
    fn meta_field_property_type_type_field() {
        use super::meta_field_property_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(matches!(
            meta_field_property_type("$type"),
            Some(DocumentPropertyType::String(_))
        ));
    }

    #[test]
    fn meta_field_property_type_unknown_returns_none() {
        use super::meta_field_property_type;

        assert!(meta_field_property_type("unknown").is_none());
        assert!(meta_field_property_type("$nonexistent").is_none());
    }

    // ---- allowed_ops_for_type ----

    #[test]
    fn allowed_ops_for_numeric_types_include_ranges() {
        use super::allowed_ops_for_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        for ty in [
            DocumentPropertyType::U8,
            DocumentPropertyType::I8,
            DocumentPropertyType::U16,
            DocumentPropertyType::I16,
            DocumentPropertyType::U32,
            DocumentPropertyType::I32,
            DocumentPropertyType::U64,
            DocumentPropertyType::I64,
            DocumentPropertyType::U128,
            DocumentPropertyType::I128,
            DocumentPropertyType::F64,
            DocumentPropertyType::Date,
        ] {
            let ops = allowed_ops_for_type(&ty);
            assert!(ops.contains(&Equal), "numeric type should allow Equal");
            assert!(ops.contains(&In), "numeric type should allow In");
            assert!(
                ops.contains(&GreaterThan),
                "numeric type should allow GreaterThan"
            );
            assert!(ops.contains(&Between), "numeric type should allow Between");
            assert!(
                !ops.contains(&super::StartsWith),
                "numeric type should not allow StartsWith"
            );
        }
    }

    #[test]
    fn allowed_ops_for_string_includes_starts_with() {
        use super::allowed_ops_for_type;
        use dpp::data_contract::document_type::{DocumentPropertyType, StringPropertySizes};

        let ty = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let ops = allowed_ops_for_type(&ty);
        assert!(ops.contains(&super::StartsWith));
        assert!(ops.contains(&Equal));
        assert!(ops.contains(&In));
        assert!(ops.contains(&GreaterThan));
    }

    #[test]
    fn allowed_ops_for_identifier_only_equal_and_in() {
        use super::allowed_ops_for_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let ops = allowed_ops_for_type(&DocumentPropertyType::Identifier);
        assert_eq!(ops, &[Equal, In]);
    }

    #[test]
    fn allowed_ops_for_boolean_only_equal() {
        use super::allowed_ops_for_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let ops = allowed_ops_for_type(&DocumentPropertyType::Boolean);
        assert_eq!(ops, &[Equal]);
    }

    #[test]
    fn allowed_ops_for_object_is_empty() {
        use super::allowed_ops_for_type;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let ops = allowed_ops_for_type(&DocumentPropertyType::Object(Default::default()));
        assert!(ops.is_empty());
    }

    // ---- value_shape_ok ----

    #[test]
    fn value_shape_ok_equal_always_true() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        // Equal accepts any value shape
        assert!(WhereOperator::Equal.value_shape_ok(&Value::I64(1), &DocumentPropertyType::U64));
        assert!(WhereOperator::Equal
            .value_shape_ok(&Value::Text("x".into()), &DocumentPropertyType::Boolean));
    }

    #[test]
    fn value_shape_ok_in_requires_array_or_bytes() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(WhereOperator::In.value_shape_ok(
            &Value::Array(vec![Value::I64(1)]),
            &DocumentPropertyType::U64
        ));
        assert!(WhereOperator::In.value_shape_ok(&Value::Bytes(vec![1]), &DocumentPropertyType::U8));
        assert!(!WhereOperator::In.value_shape_ok(&Value::I64(1), &DocumentPropertyType::U64));
    }

    #[test]
    fn value_shape_ok_starts_with_requires_text() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::{DocumentPropertyType, StringPropertySizes};

        let str_ty = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        assert!(WhereOperator::StartsWith.value_shape_ok(&Value::Text("abc".into()), &str_ty));
        assert!(!WhereOperator::StartsWith.value_shape_ok(&Value::I64(1), &str_ty));
    }

    #[test]
    fn value_shape_ok_range_for_f64_requires_numeric() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(WhereOperator::GreaterThan
            .value_shape_ok(&Value::Float(1.0), &DocumentPropertyType::F64));
        assert!(
            WhereOperator::GreaterThan.value_shape_ok(&Value::I64(1), &DocumentPropertyType::F64)
        );
        assert!(!WhereOperator::GreaterThan
            .value_shape_ok(&Value::Text("x".into()), &DocumentPropertyType::F64));
    }

    #[test]
    fn value_shape_ok_range_for_string_requires_text() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::{DocumentPropertyType, StringPropertySizes};

        let str_ty = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        assert!(WhereOperator::LessThan.value_shape_ok(&Value::Text("a".into()), &str_ty));
        assert!(!WhereOperator::LessThan.value_shape_ok(&Value::I64(1), &str_ty));
    }

    #[test]
    fn value_shape_ok_range_for_integer_requires_integer() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(
            WhereOperator::GreaterThan.value_shape_ok(&Value::U64(1), &DocumentPropertyType::U64)
        );
        assert!(
            WhereOperator::GreaterThan.value_shape_ok(&Value::I32(1), &DocumentPropertyType::I32)
        );
        assert!(!WhereOperator::GreaterThan
            .value_shape_ok(&Value::Float(1.0), &DocumentPropertyType::U64));
        assert!(!WhereOperator::GreaterThan
            .value_shape_ok(&Value::Text("x".into()), &DocumentPropertyType::U64));
    }

    #[test]
    fn value_shape_ok_between_requires_array_of_two() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let good = Value::Array(vec![Value::I64(1), Value::I64(10)]);
        assert!(WhereOperator::Between.value_shape_ok(&good, &DocumentPropertyType::I64));

        let bad_len = Value::Array(vec![Value::I64(1)]);
        assert!(!WhereOperator::Between.value_shape_ok(&bad_len, &DocumentPropertyType::I64));

        let not_array = Value::I64(5);
        assert!(!WhereOperator::Between.value_shape_ok(&not_array, &DocumentPropertyType::I64));

        // All between variants
        assert!(
            WhereOperator::BetweenExcludeBounds.value_shape_ok(&good, &DocumentPropertyType::I64)
        );
        assert!(WhereOperator::BetweenExcludeLeft.value_shape_ok(&good, &DocumentPropertyType::I64));
        assert!(
            WhereOperator::BetweenExcludeRight.value_shape_ok(&good, &DocumentPropertyType::I64)
        );
    }

    // ---- validate_against_schema: additional coverage ----

    #[test]
    fn validate_rejects_unknown_field() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "nonexistentField".to_string(),
            operator: Equal,
            value: Value::I64(1),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
    }

    #[test]
    fn validate_rejects_disallowed_operator_for_boolean() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        // Boolean only allows Equal, not GreaterThan -- but we need a boolean field.
        // Check if we can use a meta field or if there is one in the doc type.
        // $type is a String, so let's validate that startsWith is allowed for $type
        let clause = WhereClause {
            field: "$type".to_string(),
            operator: super::StartsWith,
            value: Value::Text("nice".to_string()),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_valid());
    }

    #[test]
    fn validate_rejects_starts_with_empty_string() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$type".to_string(),
            operator: super::StartsWith,
            value: Value::Text("".to_string()),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
        assert!(matches!(
            res.first_error(),
            Some(QuerySyntaxError::StartsWithIllegalString(_))
        ));
    }

    #[test]
    fn validate_rejects_in_with_empty_array() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$ownerId".to_string(),
            operator: In,
            value: Value::Array(vec![]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
    }

    #[test]
    fn validate_rejects_in_with_duplicates() {
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
                Value::Identifier([1u8; 32]),
            ]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
    }

    #[test]
    fn validate_rejects_between_with_descending_bounds() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("uniqueDates")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$createdAt".to_string(),
            operator: Between,
            value: Value::Array(vec![Value::U64(2000), Value::U64(1000)]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
        assert!(matches!(
            res.first_error(),
            Some(QuerySyntaxError::InvalidBetweenClause(_))
        ));
    }

    #[test]
    fn validate_rejects_range_operator_not_allowed_for_identifier() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$ownerId".to_string(),
            operator: GreaterThan,
            value: Value::Identifier([1u8; 32]),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_err());
    }

    #[test]
    fn validate_accepts_valid_integer_equality() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let doc_type = contract
            .document_type_for_name("niceDocument")
            .expect("doc type exists");

        let clause = WhereClause {
            field: "$revision".to_string(),
            operator: Equal,
            value: Value::U64(5),
        };
        let res = clause.validate_against_schema(doc_type);
        assert!(res.is_valid());
    }

    // ---- sql_value_to_platform_value ----

    #[test]
    fn sql_value_boolean_true() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::Boolean(true));
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn sql_value_boolean_false() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::Boolean(false));
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn sql_value_number_integer() {
        use super::sql_value_to_platform_value;
        let result =
            sql_value_to_platform_value(sqlparser::ast::Value::Number("42".to_string(), false));
        assert_eq!(result, Some(Value::I64(42)));
    }

    #[test]
    fn sql_value_number_negative_integer() {
        use super::sql_value_to_platform_value;
        let result =
            sql_value_to_platform_value(sqlparser::ast::Value::Number("-7".to_string(), false));
        assert_eq!(result, Some(Value::I64(-7)));
    }

    #[test]
    fn sql_value_number_float() {
        use super::sql_value_to_platform_value;
        let result =
            sql_value_to_platform_value(sqlparser::ast::Value::Number("3.14".to_string(), false));
        assert_eq!(result, Some(Value::Float(3.14)));
    }

    #[test]
    fn sql_value_number_unparseable_returns_none() {
        use super::sql_value_to_platform_value;
        // A string that cannot parse as i64
        let result = sql_value_to_platform_value(sqlparser::ast::Value::Number(
            "not_a_number".to_string(),
            false,
        ));
        assert_eq!(result, None);
    }

    #[test]
    fn sql_value_single_quoted_string() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::SingleQuotedString(
            "hello".to_string(),
        ));
        assert_eq!(result, Some(Value::Text("hello".to_string())));
    }

    #[test]
    fn sql_value_double_quoted_string() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::DoubleQuotedString(
            "world".to_string(),
        ));
        assert_eq!(result, Some(Value::Text("world".to_string())));
    }

    #[test]
    fn sql_value_hex_string_literal() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::HexStringLiteral(
            "0xABCD".to_string(),
        ));
        assert_eq!(result, Some(Value::Text("0xABCD".to_string())));
    }

    #[test]
    fn sql_value_national_string_literal() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::NationalStringLiteral(
            "n_str".to_string(),
        ));
        assert_eq!(result, Some(Value::Text("n_str".to_string())));
    }

    #[test]
    fn sql_value_null_returns_none() {
        use super::sql_value_to_platform_value;
        let result = sql_value_to_platform_value(sqlparser::ast::Value::Null);
        assert_eq!(result, None);
    }

    #[test]
    fn sql_value_placeholder_returns_none() {
        use super::sql_value_to_platform_value;
        let result =
            sql_value_to_platform_value(sqlparser::ast::Value::Placeholder("?".to_string()));
        assert_eq!(result, None);
    }

    // ---- WhereClause::from_components: additional operator coverage ----

    #[test]
    fn from_components_with_between_operator() {
        let components = vec![
            Value::Text("age".to_string()),
            Value::Text("between".to_string()),
            Value::Array(vec![Value::I64(10), Value::I64(20)]),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.field, "age");
        assert_eq!(clause.operator, Between);
        assert_eq!(
            clause.value,
            Value::Array(vec![Value::I64(10), Value::I64(20)])
        );
    }

    #[test]
    fn from_components_with_between_exclude_bounds_operator() {
        let components = vec![
            Value::Text("score".to_string()),
            Value::Text("betweenExcludeBounds".to_string()),
            Value::Array(vec![Value::Float(1.0), Value::Float(9.0)]),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, BetweenExcludeBounds);
    }

    #[test]
    fn from_components_with_greater_than_or_equals() {
        let components = vec![
            Value::Text("price".to_string()),
            Value::Text(">=".to_string()),
            Value::U64(100),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, GreaterThanOrEquals);
        assert_eq!(clause.value, Value::U64(100));
    }

    #[test]
    fn from_components_with_less_than() {
        let components = vec![
            Value::Text("height".to_string()),
            Value::Text("<".to_string()),
            Value::I64(200),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, LessThan);
    }

    #[test]
    fn from_components_with_less_than_or_equals() {
        let components = vec![
            Value::Text("height".to_string()),
            Value::Text("<=".to_string()),
            Value::I64(200),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, LessThanOrEquals);
    }

    #[test]
    fn from_components_preserves_value_type() {
        // Ensure the value is cloned as-is, including complex types
        let components = vec![
            Value::Text("tags".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![
                Value::Text("a".to_string()),
                Value::Text("b".to_string()),
                Value::Text("c".to_string()),
            ]),
        ];
        let clause = WhereClause::from_components(&components).unwrap();
        assert_eq!(clause.operator, In);
        if let Value::Array(arr) = &clause.value {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("expected Array value");
        }
    }

    #[test]
    fn from_components_empty_returns_error() {
        let components: Vec<Value> = vec![];
        assert!(WhereClause::from_components(&components).is_err());
    }

    #[test]
    fn from_components_single_element_returns_error() {
        let components = vec![Value::Text("name".to_string())];
        assert!(WhereClause::from_components(&components).is_err());
    }

    // ---- WhereClause::less_than: additional equal-value coverage ----

    #[test]
    fn less_than_u64_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U64(10),
        };
        assert!(a.less_than(&a, true).unwrap()); // le
        assert!(!a.less_than(&a, false).unwrap()); // lt
    }

    #[test]
    fn less_than_u32_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U32(5),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_i32_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I32(-3),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_u16_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U16(100),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_u8_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U8(7),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_i8_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I8(-1),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_u128_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U128(999),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_bytes_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Bytes(vec![1, 2, 3]),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_text_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Text("same".to_string()),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_float_equal_values_with_allow_eq() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Float(2.5),
        };
        assert!(a.less_than(&a, true).unwrap());
        assert!(!a.less_than(&a, false).unwrap());
    }

    #[test]
    fn less_than_mismatched_integer_types_returns_error() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::U64(1),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::I64(1),
        };
        assert!(a.less_than(&b, false).is_err());
    }

    #[test]
    fn less_than_bool_vs_bool_returns_error() {
        let a = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Bool(true),
        };
        let b = WhereClause {
            field: "f".to_string(),
            operator: Equal,
            value: Value::Bool(false),
        };
        assert!(a.less_than(&b, false).is_err());
    }

    // ---- value_shape_ok: additional coverage ----

    #[test]
    fn value_shape_ok_between_with_three_elements_rejected() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let three = Value::Array(vec![Value::I64(1), Value::I64(5), Value::I64(10)]);
        assert!(!WhereOperator::Between.value_shape_ok(&three, &DocumentPropertyType::I64));
    }

    #[test]
    fn value_shape_ok_between_with_empty_array_rejected() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let empty = Value::Array(vec![]);
        assert!(!WhereOperator::Between.value_shape_ok(&empty, &DocumentPropertyType::I64));
    }

    #[test]
    fn value_shape_ok_between_for_f64_property_requires_numeric_elements() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        let good = Value::Array(vec![Value::Float(1.0), Value::Float(10.0)]);
        assert!(WhereOperator::Between.value_shape_ok(&good, &DocumentPropertyType::F64));

        let also_good = Value::Array(vec![Value::I64(1), Value::I64(10)]);
        assert!(WhereOperator::Between.value_shape_ok(&also_good, &DocumentPropertyType::F64));

        let bad = Value::Array(vec![Value::Text("a".into()), Value::Text("b".into())]);
        assert!(!WhereOperator::Between.value_shape_ok(&bad, &DocumentPropertyType::F64));
    }

    #[test]
    fn value_shape_ok_between_for_string_property_requires_text_elements() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::{DocumentPropertyType, StringPropertySizes};

        let str_ty = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });

        let good = Value::Array(vec![Value::Text("aaa".into()), Value::Text("zzz".into())]);
        assert!(WhereOperator::Between.value_shape_ok(&good, &str_ty));

        let bad = Value::Array(vec![Value::I64(1), Value::I64(10)]);
        assert!(!WhereOperator::Between.value_shape_ok(&bad, &str_ty));
    }

    #[test]
    fn value_shape_ok_between_exclude_left_with_non_array_rejected() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(!WhereOperator::BetweenExcludeLeft
            .value_shape_ok(&Value::I64(5), &DocumentPropertyType::I64));
    }

    #[test]
    fn value_shape_ok_between_exclude_right_with_non_array_rejected() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(!WhereOperator::BetweenExcludeRight
            .value_shape_ok(&Value::I64(5), &DocumentPropertyType::I64));
    }

    #[test]
    fn value_shape_ok_between_exclude_bounds_with_non_array_rejected() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(!WhereOperator::BetweenExcludeBounds
            .value_shape_ok(&Value::I64(5), &DocumentPropertyType::I64));
    }

    #[test]
    fn value_shape_ok_range_accepts_all_integer_widths() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        // Each integer value variant should be accepted for its corresponding property type
        let cases: Vec<(Value, DocumentPropertyType)> = vec![
            (Value::U8(1), DocumentPropertyType::U8),
            (Value::I8(-1), DocumentPropertyType::I8),
            (Value::U16(1), DocumentPropertyType::U16),
            (Value::I16(-1), DocumentPropertyType::I16),
            (Value::U32(1), DocumentPropertyType::U32),
            (Value::I32(-1), DocumentPropertyType::I32),
            (Value::U64(1), DocumentPropertyType::U64),
            (Value::I64(-1), DocumentPropertyType::I64),
            (Value::U128(1), DocumentPropertyType::U128),
            (Value::I128(-1), DocumentPropertyType::I128),
        ];
        for (val, ty) in cases {
            assert!(
                WhereOperator::GreaterThan.value_shape_ok(&val, &ty),
                "GreaterThan should accept integer value for {:?}",
                ty
            );
            assert!(
                WhereOperator::LessThanOrEquals.value_shape_ok(&val, &ty),
                "LessThanOrEquals should accept integer value for {:?}",
                ty
            );
        }
    }

    #[test]
    fn value_shape_ok_range_rejects_bool_for_integer_type() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(!WhereOperator::GreaterThan
            .value_shape_ok(&Value::Bool(true), &DocumentPropertyType::U64));
    }

    #[test]
    fn value_shape_ok_in_rejects_text() {
        use super::WhereOperator;
        use dpp::data_contract::document_type::DocumentPropertyType;

        assert!(!WhereOperator::In
            .value_shape_ok(&Value::Text("not-array".into()), &DocumentPropertyType::U64));
    }

    // ---- ValueClause::matches_value: additional operator coverage ----

    #[test]
    fn value_clause_matches_value_less_than() {
        let clause = ValueClause {
            operator: LessThan,
            value: Value::I64(50),
        };
        assert!(clause.matches_value(&Value::I64(30)));
        assert!(!clause.matches_value(&Value::I64(50)));
        assert!(!clause.matches_value(&Value::I64(60)));
    }

    #[test]
    fn value_clause_matches_value_less_than_or_equals() {
        let clause = ValueClause {
            operator: LessThanOrEquals,
            value: Value::I64(50),
        };
        assert!(clause.matches_value(&Value::I64(30)));
        assert!(clause.matches_value(&Value::I64(50)));
        assert!(!clause.matches_value(&Value::I64(51)));
    }

    #[test]
    fn value_clause_matches_value_greater_than_or_equals() {
        let clause = ValueClause {
            operator: GreaterThanOrEquals,
            value: Value::I64(10),
        };
        assert!(clause.matches_value(&Value::I64(10)));
        assert!(clause.matches_value(&Value::I64(100)));
        assert!(!clause.matches_value(&Value::I64(9)));
    }

    #[test]
    fn value_clause_matches_between_inclusive() {
        let clause = ValueClause {
            operator: Between,
            value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
        };
        assert!(clause.matches_value(&Value::U64(10)));
        assert!(clause.matches_value(&Value::U64(15)));
        assert!(clause.matches_value(&Value::U64(20)));
        assert!(!clause.matches_value(&Value::U64(9)));
        assert!(!clause.matches_value(&Value::U64(21)));
    }

    #[test]
    fn value_clause_matches_between_exclude_bounds() {
        let clause = ValueClause {
            operator: BetweenExcludeBounds,
            value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
        };
        assert!(!clause.matches_value(&Value::U64(10)));
        assert!(clause.matches_value(&Value::U64(15)));
        assert!(!clause.matches_value(&Value::U64(20)));
    }

    #[test]
    fn value_clause_matches_between_exclude_left() {
        let clause = ValueClause {
            operator: BetweenExcludeLeft,
            value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
        };
        assert!(!clause.matches_value(&Value::U64(10)));
        assert!(clause.matches_value(&Value::U64(11)));
        assert!(clause.matches_value(&Value::U64(20)));
    }

    #[test]
    fn value_clause_matches_between_exclude_right() {
        let clause = ValueClause {
            operator: BetweenExcludeRight,
            value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
        };
        assert!(clause.matches_value(&Value::U64(10)));
        assert!(clause.matches_value(&Value::U64(19)));
        assert!(!clause.matches_value(&Value::U64(20)));
    }

    #[test]
    fn value_clause_in_with_bytes() {
        let clause = ValueClause {
            operator: In,
            value: Value::Bytes(vec![5, 10, 15]),
        };
        assert!(clause.matches_value(&Value::U8(10)));
        assert!(!clause.matches_value(&Value::U8(20)));
        // Non-U8 against Bytes returns false
        assert!(!clause.matches_value(&Value::I64(10)));
    }

    #[test]
    fn value_clause_starts_with_non_text_returns_false() {
        let clause = ValueClause {
            operator: super::StartsWith,
            value: Value::Text("he".to_string()),
        };
        assert!(!clause.matches_value(&Value::I64(42)));
    }

    // ---- WhereClause::matches_value: additional coverage ----

    #[test]
    fn where_clause_matches_value_between() {
        let clause = WhereClause {
            field: "price".to_string(),
            operator: Between,
            value: Value::Array(vec![Value::U64(100), Value::U64(500)]),
        };
        assert!(clause.matches_value(&Value::U64(100)));
        assert!(clause.matches_value(&Value::U64(300)));
        assert!(clause.matches_value(&Value::U64(500)));
        assert!(!clause.matches_value(&Value::U64(99)));
        assert!(!clause.matches_value(&Value::U64(501)));
    }

    #[test]
    fn where_clause_matches_value_in() {
        let clause = WhereClause {
            field: "status".to_string(),
            operator: In,
            value: Value::Array(vec![
                Value::Text("a".to_string()),
                Value::Text("b".to_string()),
            ]),
        };
        assert!(clause.matches_value(&Value::Text("a".to_string())));
        assert!(clause.matches_value(&Value::Text("b".to_string())));
        assert!(!clause.matches_value(&Value::Text("c".to_string())));
    }

    #[test]
    fn where_clause_matches_value_starts_with() {
        let clause = WhereClause {
            field: "name".to_string(),
            operator: super::StartsWith,
            value: Value::Text("pre".to_string()),
        };
        assert!(clause.matches_value(&Value::Text("prefix_value".to_string())));
        assert!(!clause.matches_value(&Value::Text("no_match".to_string())));
    }

    // ---- eval: additional coverage for text comparison operators ----

    #[test]
    fn eval_greater_than_with_text() {
        assert!(GreaterThan.eval(
            &Value::Text("banana".to_string()),
            &Value::Text("apple".to_string())
        ));
        assert!(!GreaterThan.eval(
            &Value::Text("apple".to_string()),
            &Value::Text("banana".to_string())
        ));
    }

    #[test]
    fn eval_less_than_with_text() {
        assert!(LessThan.eval(
            &Value::Text("apple".to_string()),
            &Value::Text("banana".to_string())
        ));
        assert!(!LessThan.eval(
            &Value::Text("banana".to_string()),
            &Value::Text("apple".to_string())
        ));
    }

    #[test]
    fn eval_between_with_text() {
        let bounds = Value::Array(vec![
            Value::Text("b".to_string()),
            Value::Text("d".to_string()),
        ]);
        assert!(Between.eval(&Value::Text("b".to_string()), &bounds));
        assert!(Between.eval(&Value::Text("c".to_string()), &bounds));
        assert!(Between.eval(&Value::Text("d".to_string()), &bounds));
        assert!(!Between.eval(&Value::Text("a".to_string()), &bounds));
        assert!(!Between.eval(&Value::Text("e".to_string()), &bounds));
    }

    #[test]
    fn eval_equal_with_text() {
        assert!(Equal.eval(
            &Value::Text("same".to_string()),
            &Value::Text("same".to_string())
        ));
        assert!(!Equal.eval(
            &Value::Text("one".to_string()),
            &Value::Text("two".to_string())
        ));
    }

    #[test]
    fn eval_in_with_empty_array_returns_false() {
        let arr = Value::Array(vec![]);
        assert!(!In.eval(&Value::I64(1), &arr));
    }

    #[test]
    fn eval_starts_with_empty_prefix_matches_everything() {
        assert!(super::StartsWith.eval(
            &Value::Text("anything".to_string()),
            &Value::Text("".to_string())
        ));
    }
}
