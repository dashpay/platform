use crate::contract::{Document, DocumentType};
use ciborium::value::{Integer, Value};
use grovedb::{Error, Query};
use sqlparser::ast;
use WhereOperator::{
    Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, Equal, GreaterThan,
    GreaterThanOrEquals, In, LessThan, LessThanOrEquals, StartsWith,
};

fn sql_value_to_cbor(sql_value: ast::Value) -> Option<Value> {
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
                num_as_int.map(|num| Value::Integer(Integer::from(num)))
            }
        }
        ast::Value::DoubleQuotedString(s) => Some(Value::Text(s)),
        ast::Value::SingleQuotedString(s) => Some(Value::Text(s)),
        ast::Value::HexStringLiteral(s) => Some(Value::Text(s)),
        ast::Value::NationalStringLiteral(s) => Some(Value::Text(s)),
        _ => None,
    }
}

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

    pub(crate) fn from_sql_operator(sql_operator: ast::BinaryOperator) -> Option<Self> {
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhereClause {
    pub field: String,
    pub operator: WhereOperator,
    pub value: Value,
}

impl<'a> WhereClause {
    pub fn is_identifier(&self) -> bool {
        self.field == "$id"
    }

    pub fn from_components(clause_components: &'a [Value]) -> Result<Self, Error> {
        if clause_components.len() != 3 {
            return Err(Error::InvalidQuery(
                "where clauses should have at most 3 components",
            ));
        }

        let field_value = clause_components
            .get(0)
            .expect("check above enforces it exists");
        let field_ref = field_value.as_text().ok_or(Error::InvalidQuery(
            "first field of where component should be a string",
        ))?;
        let field = String::from(field_ref);

        let operator_value = clause_components
            .get(1)
            .expect("check above enforces it exists");
        let operator_string = operator_value.as_text().ok_or(Error::InvalidQuery(
            "second field of where component should be a string",
        ))?;

        let operator = WhereOperator::from_string(operator_string).ok_or({
            Error::InvalidQuery("second field of where component should be a known operator")
        })?;

        let value = clause_components
            .get(2)
            .ok_or(Error::InvalidQuery(
                "third field of where component should exist",
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
            1 => Ok(Some(lower_range_clauses.get(0).unwrap())),
            _ => Err(Error::InvalidQuery(
                "there can only at most one range clause with a lower bound",
            )),
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
            _ => Err(Error::InvalidQuery(
                "there can only at most one range clause with a lower bound",
            )),
        }
    }

    pub(crate) fn group_range_clauses(
        where_clauses: &'a [WhereClause],
    ) -> Result<Option<Self>, Error> {
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
                return Err(Error::InvalidQuery(
                    "there can only be at most 2 range clauses",
                ));
            } else if groupable_range_clauses
                .iter()
                .any(|&z| z.field != groupable_range_clauses.first().unwrap().field)
            {
                return Err(Error::InvalidQuery("all ranges must be on same field"));
            } else {
                let lower_upper_error = || {
                    Error::InvalidQuery(
                        "lower and upper bounds must be passed if providing 2 ranges",
                    )
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
            Err(Error::InvalidQuery(
                "there can not be more than 1 non groupable range clause",
            ))
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
        .ok_or({
            Error::InvalidQuery(
                "when using between operator you must provide a tuple array of values",
            )
        })?;
        if in_values.len() != 2 {
            return Err(Error::InvalidQuery(
                "when using between operator you must provide an array of exactly two values",
            ));
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
    pub(crate) fn to_path_query(
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
                    _ => Err(Error::InvalidQuery(
                        "when using in operator you must provide an array of values",
                    )),
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
                let last_char = right_key.last_mut().ok_or({
                    Error::InvalidQuery("starts with must have at least one character")
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
        where_clauses: &mut Vec<WhereClause>,
    ) -> Result<(), Error> {
        match &binary_operation {
            ast::Expr::InList {
                expr,
                list,
                negated,
            } => {
                if *negated {
                    return Err(Error::InvalidQuery(
                        "Invalid query: negated in clause not supported",
                    ));
                }

                let field_name = if let ast::Expr::Identifier(ident) = &**expr {
                    ident.value.clone()
                } else {
                    return Err(Error::InvalidQuery(
                        "Invalid query: in clause should start with an identifier",
                    ));
                };

                let mut in_values: Vec<Value> = Vec::new();
                for value in list {
                    if let ast::Expr::Value(sql_value) = value {
                        let cbor_val = sql_value_to_cbor(sql_value.clone()).ok_or({
                            Error::InvalidQuery("Invalid query: unexpected value type")
                        })?;
                        in_values.push(cbor_val);
                    } else {
                        return Err(Error::InvalidQuery(
                            "Invalid query: expected a list of sql values",
                        ));
                    }
                }

                where_clauses.push(WhereClause {
                    field: field_name,
                    operator: WhereOperator::In,
                    value: Value::Array(in_values),
                });

                Ok(())
            }
            ast::Expr::BinaryOp { left, op, right } => {
                if *op == ast::BinaryOperator::And {
                    Self::build_where_clauses_from_operations(&*left, where_clauses)?;
                    Self::build_where_clauses_from_operations(&*right, where_clauses)?;
                } else {
                    let mut where_operator = WhereOperator::from_sql_operator(op.clone())
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
                            let match_locations: Vec<_> = inner_text.match_indices('%').collect();
                            if match_locations.len() == 1
                                && match_locations[0].0 == inner_text.len() - 1
                            {
                                Value::Text(String::from(&inner_text[..(inner_text.len() - 1)]))
                            } else {
                                return Err(Error::InvalidQuery(
                                    "Invalid query: like can only be used to represent startswith",
                                ));
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
            _ => Err(Error::InvalidQuery(
                "Issue parsing sql: invalid selection format",
            )),
        }
    }
}
