//! v0 of the where-clause grouping: the grammar every protocol version
//! up to 13 committed to. Restored verbatim from the pre-multi-`In`
//! implementation — most importantly, the `MultipleInClauses` check runs
//! before any same-field duplicate / overlap checks on the `In` clauses,
//! so every query with more than one non-primary-key `In` clause reports
//! `MultipleInClauses` regardless of what else is wrong with it.

use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::conditions::WhereClause;
use crate::query::conditions::WhereOperator::{
    Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, Equal, GreaterThan,
    GreaterThanOrEquals, In, LessThan, LessThanOrEquals, StartsWith,
};
use dpp::platform_value::Value;
use std::collections::{BTreeMap, BTreeSet};

/// `(equal_clauses, range_clause, in_clause)` — at most one `In` clause.
pub(super) type GroupedWhereClausesV0 = (
    BTreeMap<String, WhereClause>,
    Option<WhereClause>,
    Option<WhereClause>,
);

pub(super) fn group_where_clauses_v0(
    where_clauses: &[WhereClause],
) -> Result<GroupedWhereClausesV0, Error> {
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

    let range_clause = if non_groupable_range_clauses.is_empty() {
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

                let operator = match (lower_bounds_clause.operator, upper_bounds_clause.operator) {
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
            Err(Error::Query(
                QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                    "a non groupable range clause has same field as an equality or in clause",
                ),
            ))
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
