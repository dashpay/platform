//! Query Ordering
//!

use dpp::platform_value::Value;
use grovedb::Error;

/// Order clause struct
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderClause {
    /// Field
    pub field: String,
    /// Ascending bool
    pub ascending: bool,
}

impl OrderClause {
    /// Converts clause components to an `OrderClause`.
    pub fn from_components(clause_components: &[Value]) -> Result<Self, Error> {
        if clause_components.len() != 2 {
            return Err(Error::InvalidQuery(
                "order clause should have exactly 2 components",
            ));
        }

        let field_value = clause_components
            .first()
            .expect("check above enforces it exists");
        let field_ref = field_value.as_text().ok_or(Error::InvalidQuery(
            "first field of where component should be a string",
        ))?;
        let field = String::from(field_ref);

        let asc_string_value = clause_components.get(1).unwrap();
        let asc_string = match asc_string_value {
            Value::Text(asc_string) => Some(asc_string.as_str()),
            _ => None,
        }
        .ok_or(Error::InvalidQuery(
            "orderBy right component must be a string",
        ))?;
        let ascending = match asc_string {
            "asc" => true,
            "desc" => false,
            _ => {
                return Err(Error::InvalidQuery(
                    "orderBy right component must be either a asc or desc string",
                ));
            }
        };

        Ok(OrderClause { field, ascending })
    }
}

impl From<OrderClause> for Value {
    fn from(order: OrderClause) -> Self {
        let direction = match order.ascending {
            true => "asc",
            false => "desc",
        };

        Self::Array(vec![order.field.into(), direction.into()])
    }
}
