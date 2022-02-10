mod defaults;

use crate::contract::{bytes_for_system_value, Contract, Document, DocumentType, IndexProperty};
use crate::query::WhereOperator::{
    Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, Equal, GreaterThan,
    GreaterThanOrEquals, In, LessThan, LessThanOrEquals, StartsWith,
};
use ciborium::value::{Integer, Value as CborValue, Value};
use grovedb::{Element, Error, GroveDb, PathQuery, Query, SizedQuery};
use indexmap::IndexMap;
use sqlparser::ast;
use sqlparser::ast::TableFactor::Table;
use sqlparser::ast::Value::Number;
use sqlparser::ast::{OrderByExpr, Select, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use storage::rocksdb_storage::OptimisticTransactionDBTransaction;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WhereOperator {
    Equal,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    Between,
    BetweenExcludeBounds,
    BetweenExcludeLeft,
    BetweenExcludeRight,
    In,
    StartsWith,
}

impl WhereOperator {
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

    pub fn flip(&self) -> Result<WhereOperator, Error> {
        match self {
            Equal => Ok(Equal),
            GreaterThan => Ok(LessThan),
            GreaterThanOrEquals => Ok(LessThanOrEquals),
            LessThan => Ok(GreaterThan),
            LessThanOrEquals => Ok(GreaterThanOrEquals),
            Between => Err(Error::InvalidQuery("Between clause order invalid")),
            BetweenExcludeBounds => Err(Error::InvalidQuery("Between clause order invalid")),
            BetweenExcludeLeft => Err(Error::InvalidQuery("Between clause order invalid")),
            BetweenExcludeRight => Err(Error::InvalidQuery("Between clause order invalid")),
            In => Err(Error::InvalidQuery("In clause order invalid")),
            StartsWith => Err(Error::InvalidQuery("Startswith clause order invalid")),
        }
    }
}

impl WhereOperator {
    pub const fn is_range(self) -> bool {
        match self {
            Equal => false,
            GreaterThan | GreaterThanOrEquals | LessThan | LessThanOrEquals | Between
            | BetweenExcludeBounds | BetweenExcludeLeft | BetweenExcludeRight | In | StartsWith => {
                true
            }
        }
    }
}

fn operator_from_string(string: &str) -> Option<WhereOperator> {
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

fn where_operator_from_sql_operator(sql_operator: ast::BinaryOperator) -> Option<WhereOperator> {
    match sql_operator {
        ast::BinaryOperator::Eq => Some(WhereOperator::Equal),
        ast::BinaryOperator::Gt => Some(WhereOperator::GreaterThan),
        ast::BinaryOperator::GtEq => Some(WhereOperator::GreaterThanOrEquals),
        ast::BinaryOperator::Lt => Some(WhereOperator::LessThan),
        ast::BinaryOperator::LtEq => Some(WhereOperator::LessThanOrEquals),
        ast::BinaryOperator::Like => Some(WhereOperator::StartsWith),
        _ => None,
    }
}

fn sql_value_to_cbor(sql_value: ast::Value) -> Option<CborValue> {
    match sql_value {
        ast::Value::Boolean(bool) => Some(CborValue::Bool(bool)),
        ast::Value::Number(num, _) => {
            let number_as_string = num as String;
            if number_as_string.contains('.') {
                // Float
                let num_as_float = number_as_string.parse::<f64>().ok();
                num_as_float.map(CborValue::Float)
            } else {
                // Integer
                let num_as_int = number_as_string.parse::<i64>().ok();
                num_as_int.map(|num| CborValue::Integer(Integer::from(num)))
            }
        }
        ast::Value::DoubleQuotedString(s) => Some(CborValue::Text(s)),
        ast::Value::SingleQuotedString(s) => Some(CborValue::Text(s)),
        ast::Value::HexStringLiteral(s) => Some(CborValue::Text(s)),
        ast::Value::NationalStringLiteral(s) => Some(CborValue::Text(s)),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InternalClauses {
    primary_key_in_clause: Option<WhereClause>,
    primary_key_equal_clause: Option<WhereClause>,
    in_clause: Option<WhereClause>,
    range_clause: Option<WhereClause>,
    equal_clauses: HashMap<String, WhereClause>,
}

impl InternalClauses {
    pub fn verify(&self) -> bool {
        // There can only be 1 primary key clause, or many other clauses
        if self.primary_key_in_clause.is_some() ^ self.primary_key_equal_clause.is_some() {
            // One is set, all rest must be empty
            !(self.in_clause.is_some()
                || self.range_clause.is_some()
                || !self.equal_clauses.is_empty())
        } else {
            !(self.primary_key_in_clause.is_some() && self.primary_key_equal_clause.is_some())
        }
    }

    pub fn is_for_primary_key(&self) -> bool {
        self.primary_key_in_clause.is_some() || self.primary_key_equal_clause.is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhereClause {
    field: String,
    operator: WhereOperator,
    value: Value,
}

impl<'a> WhereClause {
    pub fn is_identifier(&self) -> bool {
        return self.field == "$id";
    }

    pub fn from_components(clause_components: &'a [Value]) -> Result<Self, Error> {
        if clause_components.len() != 3 {
            return Err(Error::CorruptedData(String::from(
                "where clauses should have at most 3 components",
            )));
        }

        let field_value = clause_components
            .get(0)
            .expect("check above enforces it exists");
        let field_ref = field_value.as_text().ok_or_else(|| {
            Error::CorruptedData(String::from(
                "first field of where component should be a string",
            ))
        })?;
        let field = String::from(field_ref);

        let operator_value = clause_components
            .get(1)
            .expect("check above enforces it exists");
        let operator_string = operator_value.as_text().ok_or_else(|| {
            Error::CorruptedData(String::from(
                "second field of where component should be a string",
            ))
        })?;

        let operator = operator_from_string(operator_string).ok_or_else(|| {
            Error::CorruptedData(String::from(
                "second field of where component should be a known operator",
            ))
        })?;

        let value = clause_components
            .get(2)
            .ok_or_else(|| {
                Error::CorruptedData(String::from("third field of where component should exist"))
            })?
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
            1 => Ok(Some(lower_range_clauses.get(0).unwrap())),
            _ => Err(Error::CorruptedData(String::from(
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
            1 => Ok(Some(upper_range_clauses.get(0).unwrap())),
            _ => Err(Error::CorruptedData(String::from(
                "there can only at most one range clause with a lower bound",
            ))),
        }
    }

    fn group_range_clauses(where_clauses: &'a [WhereClause]) -> Result<Option<Self>, Error> {
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

        return if non_groupable_range_clauses.is_empty() {
            if groupable_range_clauses.is_empty() {
                return Ok(None);
            } else if groupable_range_clauses.len() == 1 {
                let clause = *groupable_range_clauses.get(0).unwrap();
                return Ok(Some(clause.clone()));
            } else if groupable_range_clauses.len() > 2 {
                return Err(Error::CorruptedData(String::from(
                    "there can only be at most 2 range clauses",
                )));
            } else if groupable_range_clauses
                .iter()
                .any(|&z| z.field != groupable_range_clauses.first().unwrap().field)
            {
                return Err(Error::CorruptedData(String::from(
                    "all ranges must be on same field",
                )));
            } else {
                let lower_upper_error = || {
                    Error::CorruptedData(String::from(
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

                Ok(Some(WhereClause {
                    field: groupable_range_clauses.first().unwrap().field.clone(),
                    operator,
                    value: Value::Array(vec![
                        lower_bounds_clause.value.clone(),
                        upper_bounds_clause.value.clone(),
                    ]),
                }))
            }
        } else if non_groupable_range_clauses.len() == 1 {
            let where_clause = *non_groupable_range_clauses.get(0).unwrap();
            Ok(Some(where_clause.clone()))
        } else {
            // if non_groupable_range_clauses.len() > 1
            Err(Error::CorruptedData(String::from(
                "there can not be more than 1 non groupable range clause",
            )))
        };
    }

    fn split_value_for_between(
        &self,
        document_type: &DocumentType,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let in_values = match &self.value {
            Value::Array(array) => Some(array),
            _ => None,
        }
        .ok_or_else(|| {
            Error::CorruptedData(String::from(
                "when using between operator you must provide a tuple array of values",
            ))
        })?;
        if in_values.len() != 2 {
            return Err(Error::CorruptedData(String::from(
                "when using between operator you must provide an array of exactly two values",
            )));
        }
        let left_key = document_type
            .serialize_value_for_key(self.field.as_str(), in_values.get(0).unwrap())?;
        let right_key = document_type
            .serialize_value_for_key(self.field.as_str(), in_values.get(1).unwrap())?;
        Ok((left_key, right_key))
    }

    // The start at document fields are:
    // document: The Document that we should start at
    // included: whether we should start at or after this document
    // left_to_right: should we be going left to right or right to left?
    fn to_path_query(
        &self,
        document_type: &DocumentType,
        start_at_document: &Option<(Document, bool)>,
        left_to_right: bool,
    ) -> Result<Query, Error> {
        // If there is a start_at_document, we need to get the value that it has for the
        // current field.
        let starts_at_key_option = match start_at_document {
            None => None,
            Some((document, included)) => {
                // if the key doesn't exist then we should ignore the starts at key
                document
                    .get_raw_for_document_type(self.field.as_str(), document_type, None)?
                    .map(|raw_value_option| (raw_value_option, *included))
            }
        };

        let mut query = Query::new_with_direction(left_to_right);
        match self.operator {
            Equal => {
                let key =
                    document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
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
                let in_values = match &self.value {
                    Value::Array(array) => Ok(array),
                    _ => Err(Error::CorruptedData(String::from(
                        "when using in operator you must provide an array of values",
                    ))),
                }?;
                match starts_at_key_option {
                    None => {
                        for value in in_values.iter() {
                            let key = document_type
                                .serialize_value_for_key(self.field.as_str(), value)?;
                            query.insert_key(key)
                        }
                    }
                    Some((starts_at_key, included)) => {
                        for value in in_values.iter() {
                            let key = document_type
                                .serialize_value_for_key(self.field.as_str(), value)?;

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
                let key =
                    document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
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
                let key =
                    document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
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
                let key =
                    document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
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
                let key =
                    document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
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
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
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
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
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
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
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
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
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
                let left_key =
                    document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
                let mut right_key = left_key.clone();
                let last_char = right_key.last_mut().ok_or_else(|| {
                    Error::CorruptedData(String::from(
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderClause {
    pub field: String,
    pub ascending: bool,
}

impl<'a> OrderClause {
    pub fn from_components(clause_components: &'a [Value]) -> Result<Self, Error> {
        if clause_components.len() != 2 {
            return Err(Error::CorruptedData(String::from(
                "order clause should have exactly 2 components",
            )));
        }

        let field_value = clause_components
            .get(0)
            .expect("check above enforces it exists");
        let field_ref = field_value.as_text().ok_or_else(|| {
            Error::CorruptedData(String::from(
                "first field of where component should be a string",
            ))
        })?;
        let field = String::from(field_ref);

        let asc_string_value = clause_components.get(1).unwrap();
        let asc_string = match asc_string_value {
            Value::Text(asc_string) => Some(asc_string.as_str()),
            _ => None,
        }
        .ok_or_else(|| {
            Error::CorruptedData(String::from("orderBy right component must be a string"))
        })?;
        let ascending = match asc_string {
            "asc" => true,
            "desc" => false,
            _ => {
                return Err(Error::CorruptedData(String::from(
                    "orderBy right component must be either a asc or desc string",
                )));
            }
        };

        Ok(OrderClause { field, ascending })
    }
}

#[derive(Debug, PartialEq)]
pub struct DriveQuery<'a> {
    pub contract: &'a Contract,
    pub document_type: &'a DocumentType,
    pub internal_clauses: InternalClauses,
    pub offset: u16,
    pub limit: u16,
    pub order_by: IndexMap<String, OrderClause>,
    pub start_at: Option<Vec<u8>>,
    pub start_at_included: bool,
}

impl<'a> DriveQuery<'a> {
    pub fn from_cbor(
        query_cbor: &[u8],
        contract: &'a Contract,
        document_type: &'a DocumentType,
    ) -> Result<Self, Error> {
        let query_document: HashMap<String, CborValue> = ciborium::de::from_reader(query_cbor)
            .map_err(|err| Error::CorruptedData(format!("unable to decode query: {}", err)))?;

        let limit: u16 = query_document
            .get("limit")
            .map_or(Some(defaults::DEFAULT_QUERY_LIMIT), |id_cbor| {
                if let CborValue::Integer(b) = id_cbor {
                    Some(i128::from(*b) as u16)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                Error::CorruptedData(String::from("limit should be a integer from 1 to 100"))
            })?;

        let all_where_clauses: Vec<WhereClause> =
            query_document.get("where").map_or(Ok(vec![]), |id_cbor| {
                if let CborValue::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .map(|where_clause| {
                            if let CborValue::Array(clauses_components) = where_clause {
                                WhereClause::from_components(clauses_components)
                            } else {
                                Err(Error::CorruptedData(String::from(
                                    "where clause must be an array",
                                )))
                            }
                        })
                        .collect::<Result<Vec<WhereClause>, Error>>()
                } else {
                    Err(Error::CorruptedData(String::from(
                        "where clause must be an array",
                    )))
                }
            })?;

        let internal_clauses = Self::extract_clauses(all_where_clauses)?;

        let start_at_option = query_document.get("startAt");
        let start_after_option = query_document.get("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::CorruptedData(String::from(
                "only one of startAt or startAfter should be provided",
            )));
        }

        let mut start_at_included = true;

        let mut start_option: Option<&Value> = None;

        if start_after_option.is_some() {
            start_option = start_after_option;
            start_at_included = false;
        } else if start_at_option.is_some() {
            start_option = start_at_option;
            start_at_included = true;
        }

        let start_at: Option<Vec<u8>> = start_option.and_then(bytes_for_system_value);

        let order_by: IndexMap<String, OrderClause> = query_document
            .get("orderBy")
            .map_or(vec![], |id_cbor| {
                if let CborValue::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .filter_map(|order_clause| {
                            if let CborValue::Array(clauses_components) = order_clause {
                                OrderClause::from_components(clauses_components).ok()
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            })
            .iter()
            .map(|order_clause| Ok((order_clause.field.clone(), order_clause.to_owned())))
            .collect::<Result<IndexMap<String, OrderClause>, Error>>()?;

        Ok(DriveQuery {
            contract,
            document_type,
            internal_clauses,
            offset: 0,
            limit,
            order_by,
            start_at,
            start_at_included,
        })
    }

    pub fn from_sql_expr(sql_string: &str, contract: &'a Contract) -> Result<Self, Error> {
        let dialect: GenericDialect = sqlparser::dialect::GenericDialect {};
        let statements: Vec<Statement> = Parser::parse_sql(&dialect, sql_string)
            .map_err(|_| Error::CorruptedData(String::from("Issue parsing sql")))?;

        // Should ideally iterate over each statement
        let first_statement = statements
            .get(0)
            .ok_or_else(|| Error::CorruptedData(String::from("Issue parsing SQL")))?;

        let query: &ast::Query = match first_statement {
            ast::Statement::Query(query_struct) => Some(query_struct),
            _ => None,
        }
        .ok_or_else(|| Error::CorruptedData(String::from("Issue parsing sql")))?;

        let limit = if let Some(limit_expr) = &query.limit {
            match limit_expr {
                ast::Expr::Value(Number(num_string, _)) => num_string.parse::<u16>().ok(),
                _ => None,
            }
            .ok_or_else(|| {
                Error::CorruptedData(String::from("Issue parsing sql: invalid limit value"))
            })?
        } else {
            defaults::DEFAULT_QUERY_LIMIT
        };

        let order_by: IndexMap<String, OrderClause> = query
            .order_by
            .iter()
            .map(|order_exp: &OrderByExpr| {
                let ascending = order_exp.asc.is_none() || order_exp.asc.unwrap();
                let field = order_exp.expr.to_string();
                (field.clone(), OrderClause { field, ascending })
            })
            .collect::<IndexMap<String, OrderClause>>();

        // Grab the select section of the query
        let select: &Select = match &query.body {
            ast::SetExpr::Select(select) => Some(select),
            _ => None,
        }
        .ok_or_else(|| Error::CorruptedData(String::from("Issue parsing sql")))?;

        // Get the document type from the 'from' section
        let document_type_name = match &select
            .from
            .get(0)
            .ok_or(Error::InvalidQuery("Invalid query: missing from section"))?
            .relation
        {
            Table {
                name,
                alias: _,
                args: _,
                with_hints: _,
            } => name.0.get(0).as_ref().map(|identifier| &identifier.value),
            _ => None,
        }
        .ok_or_else(|| {
            Error::CorruptedData(String::from("Issue parsing sql: invalid from value"))
        })?;

        let document_type = contract
            .document_types
            .get(document_type_name)
            .ok_or(Error::InvalidQuery("document type not found in contract"))?;

        // Restrictions
        // only binary where clauses are supported
        // i.e. [<fieldname>, <operator>, <value>]
        // [and] is used to separate where clauses
        // hence once [and] is encountered [left] and [right] must be binary operations
        // i.e other where clauses
        // e.g. firstname = wisdom and lastname = ogwu
        // if op is not [and] then [left] or [right] must not be a binary operation
        let mut all_where_clauses: Vec<WhereClause> = Vec::new();
        let selection_tree = select.selection.as_ref();

        fn build_where_clause(
            binary_operation: &ast::Expr,
            where_clauses: &mut Vec<WhereClause>,
        ) -> Result<(), Error> {
            match &binary_operation {
                ast::Expr::BinaryOp { left, op, right } => {
                    if *op == ast::BinaryOperator::And {
                        build_where_clause(&*left, where_clauses)?;
                        build_where_clause(&*right, where_clauses)?;
                    } else {
                        let mut where_operator = where_operator_from_sql_operator(op.clone())
                            .ok_or(Error::InvalidQuery("Unknown operator"))?;

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
                            return Err(Error::InvalidQuery(
                                "Invalid query: where clause should have field name and value",
                            ));
                        }

                        let field_name = if let ast::Expr::Identifier(ident) = identifier {
                            ident.value.clone()
                        } else {
                            panic!("unreachable: confirmed it's identifier variant");
                        };

                        let value = if let ast::Expr::Value(value) = value_expr {
                            let cbor_val = sql_value_to_cbor(value.clone()).ok_or({
                                Error::InvalidQuery("Invalid query: unexpected value type")
                            })?;
                            if where_operator == StartsWith {
                                // make sure the value is of the right format i.e prefix%
                                let inner_text = cbor_val.as_text().ok_or({
                                    Error::InvalidQuery("Invalid query: startsWith takes text")
                                })?;
                                let match_locations: Vec<_> =
                                    inner_text.match_indices('%').collect();
                                if match_locations.len() == 1
                                    && match_locations[0].0 == inner_text.len() - 1
                                {
                                    CborValue::Text(String::from(
                                        &inner_text[..(inner_text.len() - 1)],
                                    ))
                                } else {
                                    return Err(Error::InvalidQuery("Invalid query: like can only be used to represent startswith"));
                                }
                            } else {
                                cbor_val
                            }
                        } else {
                            panic!("unreachable: confirmed it's value variant");
                        };

                        where_clauses.push(WhereClause {
                            field: field_name,
                            operator: where_operator,
                            value,
                        });
                    }
                    Ok(())
                }
                _ => Err(Error::CorruptedData(String::from(
                    "Issue parsing sql: invalid selection format",
                ))),
            }
        }

        // Where clauses are optional
        if let Some(selection_tree) = selection_tree {
            build_where_clause(selection_tree, &mut all_where_clauses)?;
        }

        let internal_clauses = Self::extract_clauses(all_where_clauses)?;

        let start_at_option = None;
        let start_at_included = true;
        let start_at: Option<Vec<u8>> = start_at_option.and_then(bytes_for_system_value);

        Ok(DriveQuery {
            contract,
            document_type,
            internal_clauses,
            offset: 0,
            limit,
            order_by,
            start_at,
            start_at_included,
        })
    }

    fn extract_clauses(all_where_clauses: Vec<WhereClause>) -> Result<InternalClauses, Error> {
        let primary_key_equal_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                Equal => match where_clause.is_identifier() {
                    true => Some(where_clause.clone()),
                    false => None,
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let primary_key_in_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                In => match where_clause.is_identifier() {
                    true => Some(where_clause.clone()),
                    false => None,
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let range_clause = WhereClause::group_range_clauses(&all_where_clauses)?;

        let equal_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                Equal => match where_clause.is_identifier() {
                    true => None,
                    false => Some(where_clause.clone()),
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let in_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                In => match where_clause.is_identifier() {
                    true => None,
                    false => Some(where_clause.clone()),
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let primary_key_equal_clause = match primary_key_equal_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_equal_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::CorruptedData(String::from(
                "There should only be one equal clause for the primary key",
            ))),
        }?;

        let primary_key_in_clause = match primary_key_in_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_in_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::CorruptedData(String::from(
                "There should only be one in clause for the primary key",
            ))),
        }?;

        let in_clause = match in_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                in_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::CorruptedData(String::from(
                "There should only be one in clause",
            ))),
        }?;

        let equal_clauses: HashMap<String, WhereClause> = equal_clauses_array
            .into_iter()
            .map(|where_clause| (where_clause.field.clone(), where_clause))
            .collect();

        let internal_clauses = InternalClauses {
            primary_key_equal_clause,
            primary_key_in_clause,
            in_clause,
            range_clause,
            equal_clauses,
        };

        match internal_clauses.verify() {
            true => Ok(internal_clauses),
            false => Err(Error::InvalidQuery("Query has invalid where clauses")),
        }
    }

    pub fn execute_with_proof(
        self,
        _grove: &mut GroveDb,
        _transaction: Option<&OptimisticTransactionDBTransaction>,
    ) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn execute_no_proof(
        &self,
        grove: &mut GroveDb,
        transaction: Option<&OptimisticTransactionDBTransaction>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        // First we should get the overall document_type_path
        let document_type_path = self
            .contract
            .document_type_path(self.document_type.name.as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();

        let starts_at_document: Option<(Document, bool)> = match &self.start_at {
            None => Ok(None),
            Some(starts_at) => {
                // First if we have a startAt or or startsAfter we must get the element
                // from the backing store

                let document_holding_path = self
                    .contract
                    .documents_primary_key_path(self.document_type.name.as_str());
                let start_at_document = grove.get(document_holding_path, starts_at, transaction)?;
                if let Element::Item(item) = start_at_document {
                    let document = Document::from_cbor(item.as_slice(), None, None)?;
                    Ok(Some((document, self.start_at_included)))
                } else {
                    Err(Error::CorruptedData(String::from(
                        "Holding paths should only have items",
                    )))
                }
            }
        }?;

        let equal_fields = self
            .internal_clauses
            .equal_clauses
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();
        let in_field = self
            .internal_clauses
            .in_clause
            .as_ref()
            .map(|in_clause| in_clause.field.as_str());
        let range_field = self
            .internal_clauses
            .range_clause
            .as_ref()
            .map(|range_clause| range_clause.field.as_str());
        let mut fields = equal_fields;
        if let Some(range_field) = range_field {
            fields.push(range_field);
        }
        if let Some(in_field) = in_field {
            fields.push(in_field);
            //if there is an in_field, it always takes precedence
        }

        let order_by_keys: Vec<&str> = self
            .order_by
            .keys()
            .map(|key: &String| {
                let str = key.as_str();
                if !fields.contains(&str) {
                    fields.push(str);
                }
                str
            })
            .collect();

        let path_query = if !self.internal_clauses.is_for_primary_key() {
            let (index, difference) = self
                .document_type
                .index_for_types(fields.as_slice(), in_field, order_by_keys.as_slice())
                .ok_or(Error::InvalidQuery("query must be for valid indexes"))?;
            if difference > defaults::MAX_INDEX_DIFFERENCE {
                return Err(Error::InvalidQuery(
                    "query must better match an existing index",
                ));
            }

            let ordered_clauses: Vec<&WhereClause> = index
                .properties
                .iter()
                .filter_map(|field| self.internal_clauses.equal_clauses.get(field.name.as_str()))
                .collect();
            let (last_clause, last_clause_is_range, subquery_clause) =
                match &self.internal_clauses.in_clause {
                    None => match &self.internal_clauses.range_clause {
                        None => (ordered_clauses.last().copied(), false, None),
                        Some(where_clause) => (Some(where_clause), true, None),
                    },
                    Some(where_clause) => match &self.internal_clauses.range_clause {
                        None => (Some(where_clause), true, None),
                        Some(range_clause) => (Some(where_clause), true, Some(range_clause)),
                    },
                };

            // We need to get the terminal indexes unused by clauses.
            let left_over_index_properties = index
                .properties
                .iter()
                .filter(|field| {
                    !(self
                        .internal_clauses
                        .equal_clauses
                        .get(field.name.as_str())
                        .is_some()
                        || (last_clause.is_some() && last_clause.unwrap().field == field.name)
                        || (subquery_clause.is_some()
                            && subquery_clause.unwrap().field == field.name))
                })
                .collect::<Vec<&IndexProperty>>();

            let intermediate_values = index
                .properties
                .iter()
                .filter_map(|field| {
                    match self.internal_clauses.equal_clauses.get(field.name.as_str()) {
                        None => None,
                        Some(where_clause) => {
                            if !last_clause_is_range
                                && last_clause.is_some()
                                && last_clause.unwrap().field == field.name
                            {
                                //there is no need to give an intermediate value as the last clause is an equality
                                None
                            } else {
                                Some(self.document_type.serialize_value_for_key(
                                    field.name.as_str(),
                                    &where_clause.value,
                                ))
                            }
                        }
                    }
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?;

            fn recursive_insert(
                query: Option<&mut Query>,
                left_over_index_properties: &[&IndexProperty],
                unique: bool,
            ) -> Option<Query> {
                match left_over_index_properties.split_first() {
                    None => {
                        if let Some(query) = query {
                            match unique {
                                true => {
                                    query.set_subquery_key(vec![0]);
                                }
                                false => {
                                    query.set_subquery_key(vec![0]);
                                    // we just get all by document id order ascending
                                    let mut full_query = Query::new();
                                    full_query.insert_all();
                                    query.set_subquery(full_query);
                                }
                            }
                        }
                        None
                    }
                    Some((first, left_over)) => {
                        let mut inner_query = Query::new_with_direction(first.ascending);
                        inner_query.insert_all();
                        recursive_insert(Some(&mut inner_query), left_over, unique);
                        match query {
                            None => Some(inner_query),
                            Some(query) => {
                                query.set_subquery(inner_query);
                                query.set_subquery_key(first.name.as_bytes().to_vec());
                                None
                            }
                        }
                    }
                }
            }

            let final_query = match last_clause {
                None => recursive_insert(None, left_over_index_properties.as_slice(), index.unique)
                    .expect("Index must have left over properties if no last clause"),
                Some(where_clause) => {
                    let left_to_right = if where_clause.operator.is_range() {
                        let order_clause: &OrderClause = self
                            .order_by
                            .get(where_clause.field.as_str())
                            .ok_or(Error::InvalidQuery(
                                "query must have an orderBy field for each range element",
                            ))?;

                        order_clause.ascending
                    } else {
                        true
                    };

                    let mut query = where_clause.to_path_query(
                        self.document_type,
                        &starts_at_document,
                        left_to_right,
                    )?;

                    match subquery_clause {
                        None => {
                            recursive_insert(
                                Some(&mut query),
                                left_over_index_properties.as_slice(),
                                index.unique,
                            );
                        }
                        Some(where_clause) => {
                            let order_clause: &OrderClause = self
                                .order_by
                                .get(where_clause.field.as_str())
                                .ok_or(Error::InvalidQuery(
                                    "query must have an orderBy field for each range element",
                                ))?;
                            let mut subquery = where_clause.to_path_query(
                                self.document_type,
                                &starts_at_document,
                                order_clause.ascending,
                            )?;
                            recursive_insert(
                                Some(&mut subquery),
                                left_over_index_properties.as_slice(),
                                index.unique,
                            );
                            let subindex = where_clause.field.as_bytes().to_vec();
                            query.set_subquery_key(subindex);
                            query.set_subquery(subquery);
                        }
                    };

                    query
                }
            };

            let (intermediate_indexes, last_indexes) =
                index.properties.split_at(intermediate_values.len());

            // Now we should construct the path
            let last_index = last_indexes.first().ok_or_else(|| {
                Error::CorruptedData(String::from("document query has no index with fields"))
            })?;

            let mut path = document_type_path;

            for (intermediate_index, intermediate_value) in
                intermediate_indexes.iter().zip(intermediate_values.iter())
            {
                path.push(intermediate_index.name.as_bytes().to_vec());
                path.push(intermediate_value.as_slice().to_vec());
            }

            path.push(last_index.name.as_bytes().to_vec());

            Ok(PathQuery::new(
                path,
                SizedQuery::new(final_query, Some(self.limit), Some(self.offset)),
            ))
        } else {
            let mut path = document_type_path.clone();

            // Add primary key ($id) subtree
            path.push(vec![0]);

            if let Some(primary_key_equal_clause) = &self.internal_clauses.primary_key_equal_clause
            {
                let mut query = Query::new();
                let key = self
                    .document_type
                    .serialize_value_for_key("$id", &primary_key_equal_clause.value)?;
                query.insert_key(key);

                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, Some(self.limit), Some(self.offset)),
                ))
            } else if let Some(primary_key_in_clause) = &self.internal_clauses.primary_key_in_clause
            {
                let left_to_right = if self.order_by.keys().len() == 1 {
                    if self.order_by.keys().next().unwrap() != "$id" {
                        return Err(Error::CorruptedData(String::from(
                            "order by should include $id only",
                        )));
                    }

                    let order_clause = self.order_by.get("$id").unwrap();

                    order_clause.ascending
                } else {
                    true
                };
                let mut query = Query::new_with_direction(left_to_right);
                // If there is a start_at_document, we need to get the value that it has for the
                // current field.
                let starts_at_key_option = match starts_at_document {
                    None => None,
                    Some((document, included)) => {
                        // if the key doesn't exist then we should ignore the starts at key
                        document
                            .get_raw_for_document_type("$id", self.document_type, None)?
                            .map(|raw_value_option| (raw_value_option, included))
                    }
                };
                let in_values = match &primary_key_in_clause.value {
                    Value::Array(array) => Ok(array),
                    _ => Err(Error::CorruptedData(String::from(
                        "when using in operator you must provide an array of values",
                    ))),
                }?;
                match starts_at_key_option {
                    None => {
                        for value in in_values.iter() {
                            let key = self.document_type.serialize_value_for_key("$id", value)?;
                            query.insert_key(key)
                        }
                    }
                    Some((starts_at_key, included)) => {
                        for value in in_values.iter() {
                            let key = self.document_type.serialize_value_for_key("$id", value)?;

                            if (left_to_right && starts_at_key < key)
                                || (!left_to_right && starts_at_key > key)
                                || (included && starts_at_key == key)
                            {
                                query.insert_key(key);
                            }
                        }
                    }
                }
                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, Some(self.limit), Some(self.offset)),
                ))
            } else {
                Err(Error::CorruptedData(String::from(
                    "this should be impossible to get to",
                )))
            }
        }?;

        let query_result = grove.get_path_query(&path_query, transaction);
        match query_result {
            Err(Error::InvalidPathKey(ref message)) => {
                if message.starts_with("key not found in Merk:") {
                    Ok((Vec::new(), 0))
                } else {
                    query_result
                }
            }
            Err(e) => Err(e),
            Ok(result) => Ok(result),
        }
    }
}
