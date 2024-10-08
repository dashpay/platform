use std::convert::TryFrom;

use anyhow::{anyhow, bail};

/// JsonPath represents a deserialized [JsonPathLiteral]. The JsonPath is made
/// of [JsonPathStep]. [JsonPath] can be created from string
/// ## Example
///  ```
/// use dpp::util::json_path::{JsonPath, JsonPathLiteral};
/// use std::convert::TryFrom;
///
/// let json_path = JsonPath::try_from(JsonPathLiteral("contract.data.collection[0]")).unwrap();
/// assert_eq!(4, json_path.len());
/// ```
pub type JsonPath = Vec<JsonPathStep>;

/// Single step in [`JsonPath`]. The step can be a `String` - to access data in objects
/// or `usize` - to access data in collections
// TODO To reduce memory allocation, the String should be replaced with the &str
#[derive(Debug, Clone)]
pub enum JsonPathStep {
    Key(String),
    Index(usize),
}

/// JsonPathLiteral represents the path in JSON structure.
pub struct JsonPathLiteral<'a>(pub &'a str);

impl<'a> std::ops::Deref for JsonPathLiteral<'a> {
    type Target = &'a str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a str> for JsonPathLiteral<'a> {
    fn from(s: &'a str) -> Self {
        JsonPathLiteral(s)
    }
}

impl<'a> TryFrom<JsonPathLiteral<'a>> for JsonPath {
    type Error = anyhow::Error;

    // TODO include validation:
    // - validation: empty steps.
    // - Valid and invalid characters should take into account the Schema
    // - no path steps
    fn try_from(path: JsonPathLiteral<'a>) -> Result<Self, Self::Error> {
        let mut steps: Vec<JsonPathStep> = vec![];
        let raw_steps = path.split('.');

        for step in raw_steps {
            if let Ok((step_key, step_index)) = try_parse_indexed_field(step) {
                steps.push(JsonPathStep::Key(step_key.to_string()));
                steps.push(JsonPathStep::Index(step_index));
            } else {
                steps.push(JsonPathStep::Key(step.to_string()))
            };
        }
        Ok(steps)
    }
}

// try to parse indexed step path. i.e: "property_name[0]"
fn try_parse_indexed_field(step: &str) -> Result<(String, usize), anyhow::Error> {
    let chars: Vec<char> = step.chars().collect();
    let index_open = chars.iter().position(|c| c == &'[');
    let index_close = chars.iter().position(|c| c == &']');

    if index_open.is_none() {
        bail!("open index bracket not found");
    }
    if index_close.is_none() {
        bail!("close index bracket not found");
    }
    if index_open > index_close {
        bail!("open bracket is ahead of close bracket")
    }
    if index_close.unwrap() != chars.len() - 1 {
        bail!("the close bracket must be the last character")
    }

    let index_str: String = chars[index_open.unwrap() + 1..index_close.unwrap()]
        .iter()
        .collect();

    let index: usize = index_str
        .parse()
        .map_err(|e| anyhow!("unable to parse '{}' into usize: {}", index_str, e))?;
    let key: String = chars[0..index_open.unwrap()].iter().collect();

    Ok((key, index))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_indexed_field() {
        let input = "data[1]";
        let (key, index) = try_parse_indexed_field(input).unwrap();

        assert_eq!("data", key);
        assert_eq!(1, index);

        let input = "数据[3]";
        let (key, index) = try_parse_indexed_field(input).unwrap();

        assert_eq!("数据", key);
        assert_eq!(3, index);

        let input = "data---__[1]";
        let (key, index) = try_parse_indexed_field(input).unwrap();

        assert_eq!("data---__", key);
        assert_eq!(1, index);

        let input = "";
        assert!(try_parse_indexed_field(input).is_err());
        assert_eq!(
            try_parse_indexed_field(input).unwrap_err().to_string(),
            "open index bracket not found"
        );

        let input = "da[0]ta";
        assert!(try_parse_indexed_field(input).is_err());
        assert_eq!(
            try_parse_indexed_field(input).unwrap_err().to_string(),
            "the close bracket must be the last character"
        );

        let input = "data[string]";
        assert!(try_parse_indexed_field(input).is_err());
        assert_eq!(
            try_parse_indexed_field(input).unwrap_err().to_string(),
            "unable to parse 'string' into usize: invalid digit found in string"
        );
    }
}
