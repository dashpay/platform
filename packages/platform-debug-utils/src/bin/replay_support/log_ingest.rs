use super::replay::{LoadedRequest, ReplayItem};
use hex::encode as hex_encode;
use serde::de::Error as DeError;
use serde::de::IgnoredAny;
use serde::de::value;
use serde::de::{DeserializeOwned, DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess};
use serde::forward_to_deserialize_any;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::{Path, PathBuf};
use tenderdash_abci::proto::serializers::timestamp::to_rfc3339_nanos;
use time::OffsetDateTime;

pub struct LogRequestStream {
    path: PathBuf,
    lines: Lines<BufReader<File>>,
    line_number: usize,
    buffered: Option<ReplayItem>,
}

impl LogRequestStream {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        Ok(Self {
            path: canonical,
            lines: BufReader::new(file).lines(),
            line_number: 0,
            buffered: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn peek(&mut self) -> Result<Option<&ReplayItem>, Box<dyn std::error::Error>> {
        if self.buffered.is_none() {
            self.buffered = self.read_next_request()?;
        }
        Ok(self.buffered.as_ref())
    }

    pub fn next_item(&mut self) -> Result<Option<ReplayItem>, Box<dyn std::error::Error>> {
        if let Some(item) = self.buffered.take() {
            return Ok(Some(item));
        }
        self.read_next_request()
    }

    pub fn skip_processed_entries(
        &mut self,
        max_height: u64,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut skipped = 0usize;
        loop {
            let Some(item) = self.peek()? else {
                break;
            };

            let should_skip = match item.request.block_height() {
                Some(height) => height <= max_height,
                None => false,
            };

            if should_skip {
                let _ = self.next_item()?;
                skipped += 1;
            } else {
                break;
            }
        }

        Ok(skipped)
    }

    fn read_next_request(&mut self) -> Result<Option<ReplayItem>, Box<dyn std::error::Error>> {
        while let Some(line_result) = self.lines.next() {
            let line = line_result?;
            self.line_number += 1;
            if line.trim().is_empty() {
                continue;
            }

            let entry: LogEntry = match serde_json::from_str(&line) {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::debug!(
                        "skipping malformed log line {}:{} ({:?})",
                        self.path.display(),
                        self.line_number,
                        err
                    );
                    continue;
                }
            };

            let LogEntry {
                timestamp,
                fields,
                span,
            } = entry;

            let Some(fields) = fields else {
                continue;
            };

            if fields.message.as_deref() != Some("received ABCI request") {
                continue;
            }

            let Some(raw_request) = fields.request.as_deref() else {
                continue;
            };

            let endpoint = span.and_then(|s| s.endpoint);

            match parse_logged_request(raw_request) {
                Ok(Some(loaded)) => {
                    return Ok(Some(ReplayItem::from_log(
                        self.path(),
                        self.line_number,
                        timestamp,
                        endpoint,
                        loaded,
                    )));
                }
                Ok(None) => {
                    // tracing::trace!(
                    //     "skipping replay for ABCI request {}:{} (no-op variant)",
                    //     self.path.display(),
                    //     self.line_number
                    // );
                }
                Err(err) => {
                    tracing::warn!(
                        "failed to parse ABCI request from {}:{} ({})",
                        self.path.display(),
                        self.line_number,
                        err
                    );
                }
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Deserialize)]
struct LogEntry {
    timestamp: Option<String>,
    fields: Option<LogFields>,
    span: Option<LogSpan>,
}

#[derive(Debug, Deserialize)]
struct LogFields {
    message: Option<String>,
    request: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogSpan {
    endpoint: Option<String>,
}

fn parse_logged_request(
    raw: &str,
) -> Result<Option<super::replay::LoadedRequest>, Box<dyn std::error::Error>> {
    let parser = DebugValueParser::new(raw);
    let parsed = parser
        .parse()
        .map_err(|err| format!("failed to parse request payload: {}", err))?;
    convert_parsed_request(parsed)
}

fn convert_parsed_request(
    value: ParsedValue,
) -> Result<Option<super::replay::LoadedRequest>, Box<dyn std::error::Error>> {
    match value {
        ParsedValue::Tagged { tag, value } if tag == "Request" => {
            let mut object = value.into_object()?;
            let inner = object
                .remove("value")
                .ok_or_else(|| "request log entry missing value field".to_string())?;
            parse_request_variant(inner)
        }
        other => parse_request_variant(other),
    }
}

fn parse_request_variant(
    value: ParsedValue,
) -> Result<Option<super::replay::LoadedRequest>, Box<dyn std::error::Error>> {
    let (tag, inner) = match value {
        ParsedValue::Tagged { tag, value } => (tag, *value),
        _ => return Err("request log entry is missing request variant tag".into()),
    };

    let normalized = tag.trim_start_matches("Request");

    match normalized {
        "InitChain" => deserialize_to_loaded(inner, LoadedRequest::InitChain).map(Some),
        "Info" => deserialize_to_loaded(inner, LoadedRequest::Info).map(Some),
        "PrepareProposal" => deserialize_to_loaded(inner, LoadedRequest::Prepare).map(Some),
        "ProcessProposal" => deserialize_to_loaded(inner, LoadedRequest::Process).map(Some),
        "FinalizeBlock" => deserialize_to_loaded(inner, LoadedRequest::Finalize).map(Some),
        "ExtendVote" => deserialize_to_loaded(inner, LoadedRequest::ExtendVote).map(Some),
        "VerifyVoteExtension" => {
            deserialize_to_loaded(inner, LoadedRequest::VerifyVoteExtension).map(Some)
        }
        "Flush" => Ok(None),
        other => Err(format!("unsupported request variant {}", other).into()),
    }
}

pub struct DebugValueParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> DebugValueParser<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            input: raw.as_bytes(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<ParsedValue, ParseError> {
        let value = self.parse_value()?;
        self.skip_ws();
        if self.pos != self.input.len() {
            Err(self.error("unexpected trailing characters"))
        } else {
            Ok(value)
        }
    }

    fn parse_value(&mut self) -> Result<ParsedValue, ParseError> {
        self.skip_ws();

        if self.consume("Some(") {
            let value = self.parse_value()?;
            self.skip_ws();
            self.expect(')')?;
            return Ok(value);
        }

        if self.consume("None") {
            return Ok(ParsedValue::Null);
        }

        if self.consume("true") {
            return Ok(ParsedValue::Bool(true));
        }

        if self.consume("false") {
            return Ok(ParsedValue::Bool(false));
        }

        match self.peek_char() {
            Some('{') => return self.parse_object(None),
            Some('[') => return self.parse_array(),
            Some('"') => return self.parse_string(),
            Some(ch) if ch == '-' || ch.is_ascii_digit() => return self.parse_number(),
            Some(_) => {}
            None => return Err(self.error("unexpected end of input")),
        }

        let ident = self.parse_identifier()?;
        self.skip_ws();
        match self.peek_char() {
            Some('{') => self.parse_object(Some(ident)),
            Some('(') => {
                self.pos += 1;
                let value = self.parse_value()?;
                self.skip_ws();
                self.expect(')')?;
                Ok(ParsedValue::Tagged {
                    tag: ident,
                    value: Box::new(value),
                })
            }
            _ => Ok(normalize_identifier_value(ident)),
        }
    }

    fn parse_object(&mut self, tag: Option<String>) -> Result<ParsedValue, ParseError> {
        self.expect('{')?;
        let mut map = BTreeMap::new();
        loop {
            self.skip_ws();
            if self.peek_char() == Some('}') {
                self.pos += 1;
                break;
            }
            let key = normalize_field_key(self.parse_identifier()?);
            self.skip_ws();
            self.expect(':')?;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_ws();
            if self.peek_char() == Some(',') {
                self.pos += 1;
            }
        }

        let object = ParsedValue::Object(map);
        if let Some(tag) = tag {
            ParsedValue::from_tagged(tag, object)
        } else {
            Ok(object)
        }
    }

    fn parse_array(&mut self) -> Result<ParsedValue, ParseError> {
        self.expect('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_ws();
            if self.peek_char() == Some(']') {
                self.pos += 1;
                break;
            }
            let value = self.parse_value()?;
            values.push(value);
            self.skip_ws();
            if self.peek_char() == Some(',') {
                self.pos += 1;
            }
        }
        Ok(ParsedValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<ParsedValue, ParseError> {
        self.expect('"')?;
        let mut output = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '"' => return Ok(ParsedValue::String(output)),
                '\\' => {
                    let escaped = self
                        .next_char()
                        .ok_or_else(|| self.error("unterminated escape sequence"))?;
                    let translated = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    };
                    output.push(translated);
                }
                other => output.push(other),
            }
        }
        Err(self.error("unterminated string literal"))
    }

    fn parse_number(&mut self) -> Result<ParsedValue, ParseError> {
        let start = self.pos;
        if self.peek_char() == Some('-') {
            self.pos += 1;
        }
        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            self.pos += 1;
        }
        let number = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| self.error("invalid number encoding"))?;
        Ok(ParsedValue::Number(number.to_string()))
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        self.skip_ws();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' || ch == '#' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            Err(self.error("expected identifier"))
        } else {
            Ok(String::from_utf8_lossy(&self.input[start..self.pos]).to_string())
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        self.skip_ws();
        match self.peek_char() {
            Some(ch) if ch == expected => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.error(format!("expected '{}'", expected))),
        }
    }

    fn consume(&mut self, expected: &str) -> bool {
        if self.input[self.pos..].starts_with(expected.as_bytes()) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input.get(self.pos).copied().map(|b| b as char)
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek_char(), Some(ch) if ch.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn error<T: Into<String>>(&self, msg: T) -> ParseError {
        ParseError {
            message: msg.into(),
        }
    }
}

#[derive(Debug, Clone)]
enum ParsedValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<ParsedValue>),
    Object(BTreeMap<String, ParsedValue>),
    Tagged {
        tag: String,
        value: Box<ParsedValue>,
    },
}

impl ParsedValue {
    fn into_object(self) -> Result<BTreeMap<String, ParsedValue>, Box<dyn std::error::Error>> {
        match self {
            ParsedValue::Object(map) => Ok(map),
            ParsedValue::Tagged { value, .. } => value.into_object(),
            other => Err(format!("expected object but found {:?}", other).into()),
        }
    }

    fn to_string_value(&self) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            ParsedValue::String(value) => Ok(value.clone()),
            ParsedValue::Number(value) => Ok(value.clone()),
            ParsedValue::Tagged { value, .. } => value.to_string_value(),
            other => Err(format!("expected string but found {:?}", other).into()),
        }
    }

    fn from_tagged(tag: String, value: ParsedValue) -> Result<ParsedValue, ParseError> {
        match normalize_type_tag(&tag) {
            "Timestamp" => Ok(ParsedValue::String(format_timestamp(value)?)),
            "Duration" => Ok(ParsedValue::String(format_duration(value)?)),
            "VoteExtension" => Ok(ParsedValue::Tagged {
                tag,
                value: Box::new(normalize_vote_extension(value)?),
            }),
            _ => Ok(ParsedValue::Tagged {
                tag,
                value: Box::new(value),
            }),
        }
    }
}

fn normalize_type_tag(tag: &str) -> &str {
    tag.rsplit("::").next().unwrap_or(tag)
}

fn normalize_identifier_value(token: String) -> ParsedValue {
    let trimmed = token.trim();
    if let Some(suffix) = trimmed.strip_prefix("ConsensusVersion")
        && !suffix.is_empty()
        && suffix.chars().all(|ch| ch.is_ascii_digit())
    {
        return ParsedValue::Number(suffix.to_string());
    }
    if is_enum_variant_identifier(trimmed) {
        return ParsedValue::Tagged {
            tag: "EnumVariant".to_string(),
            value: Box::new(ParsedValue::String(trimmed.to_string())),
        };
    }

    ParsedValue::String(trimmed.to_string())
}

fn is_enum_variant_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let first = match chars.next() {
        Some(ch) => ch,
        None => return false,
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn normalize_field_key(key: String) -> String {
    key.strip_prefix("r#").unwrap_or(&key).to_string()
}

fn normalize_vote_extension(value: ParsedValue) -> Result<ParsedValue, ParseError> {
    let mut map = value.into_object()?;
    if let Some(field) = map.get_mut("type") {
        convert_enum_variant(field, vote_extension_type_from_name)?;
    }
    Ok(ParsedValue::Object(map))
}

fn convert_enum_variant<F>(field: &mut ParsedValue, resolver: F) -> Result<(), ParseError>
where
    F: Fn(&str) -> Option<i32>,
{
    if let ParsedValue::Tagged { tag, value } = field {
        if tag == "EnumVariant" {
            let name = value
                .to_string_value()
                .map_err(|err| ParseError::new(err.to_string()))?;
            if let Some(num) = resolver(&name) {
                *field = ParsedValue::Number(num.to_string());
            } else {
                *field = ParsedValue::String(name);
            }
        }
    }
    Ok(())
}

fn vote_extension_type_from_name(name: &str) -> Option<i32> {
    use tenderdash_abci::proto::types::VoteExtensionType;

    let normalized = camel_to_upper_snake(name);
    VoteExtensionType::from_str_name(normalized.as_str()).map(|value| value as i32)
}

fn camel_to_upper_snake(value: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() && idx != 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vote_extension_with_raw_identifier_keys() {
        let raw = "Request { value: Some(FinalizeBlock(RequestFinalizeBlock { commit: Some(CommitInfo { round: 0, quorum_hash: [], block_signature: [], threshold_vote_extensions: [VoteExtension { r#type: ThresholdRecoverRaw, extension: [], signature: [], sign_request_id: None }] }), misbehavior: [], hash: [], height: 1, round: 0, block: None, block_id: None })) }";
        let request = parse_logged_request(raw).expect("parse succeeds");
        assert!(matches!(request, Some(LoadedRequest::Finalize(_))));
    }
}

#[derive(Debug)]
struct ParseError {
    message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    fn new<T: Into<String>>(msg: T) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl From<Box<dyn std::error::Error>> for ParseError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        ParseError::new(err.to_string())
    }
}

struct TaggedEnumAccess {
    tag: String,
    value: ParsedValue,
}

impl<'de> EnumAccess<'de> for TaggedEnumAccess {
    type Error = value::Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.tag.into_deserializer())?;
        Ok((variant, VariantDeserializer { value: self.value }))
    }
}

struct VariantDeserializer {
    value: ParsedValue,
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = value::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        IgnoredAny::deserialize(self.value.into_deserializer()).map(|_| ())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let normalized = match self.value {
            ParsedValue::Tagged { tag, value: inner } if tag == "EnumVariant" => {
                ParsedValue::String(inner.to_string_value().map_err(value::Error::custom)?)
            }
            other => other,
        };
        seed.deserialize(normalized)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Deserializer::deserialize_tuple(self.value, len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Deserializer::deserialize_struct(self.value, "", fields, visitor)
    }
}

impl<'de> IntoDeserializer<'de, value::Error> for ParsedValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de> Deserializer<'de> for ParsedValue {
    type Error = value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Null => visitor.visit_unit(),
            ParsedValue::Bool(value) => visitor.visit_bool(value),
            ParsedValue::Number(value) => {
                if let Ok(int) = value.parse::<i64>() {
                    visitor.visit_i64(int)
                } else if let Ok(uint) = value.parse::<u64>() {
                    visitor.visit_u64(uint)
                } else if let Ok(float) = value.parse::<f64>() {
                    visitor.visit_f64(float)
                } else {
                    visitor.visit_string(value)
                }
            }
            ParsedValue::String(value) => visitor.visit_string(value),
            ParsedValue::Array(values) => {
                let iter = values.into_iter().map(|v| v.into_deserializer());
                let mut seq = value::SeqDeserializer::new(iter);
                let output = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(output)
            }
            ParsedValue::Object(map) => {
                let iter = map
                    .into_iter()
                    .map(|(k, v)| (k.into_deserializer(), v.into_deserializer()));
                let mut map_deserializer = value::MapDeserializer::new(iter);
                let output = visitor.visit_map(&mut map_deserializer)?;
                map_deserializer.end()?;
                Ok(output)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_any(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Bool(value) => visitor.visit_bool(value),
            ParsedValue::String(value) => match value.to_lowercase().as_str() {
                "true" => visitor.visit_bool(true),
                "false" => visitor.visit_bool(false),
                _ => Err(DeError::custom("invalid boolean value")),
            },
            ParsedValue::Tagged { value, .. } => value.deserialize_bool(visitor),
            other => Err(DeError::custom(format!("expected bool got {:?}", other))),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Number(value) | ParsedValue::String(value) => {
                let parsed = value.parse::<i64>().map_err(DeError::custom)?;
                visitor.visit_i64(parsed)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_i64(visitor),
            other => Err(DeError::custom(format!("expected i64 got {:?}", other))),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Number(value) | ParsedValue::String(value) => {
                let parsed = value.parse::<u64>().map_err(DeError::custom)?;
                visitor.visit_u64(parsed)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_u64(visitor),
            other => Err(DeError::custom(format!("expected u64 got {:?}", other))),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Number(value) | ParsedValue::String(value) => {
                let parsed = value.parse::<f32>().map_err(DeError::custom)?;
                visitor.visit_f32(parsed)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_f32(visitor),
            other => Err(DeError::custom(format!("expected f32 got {:?}", other))),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Number(value) | ParsedValue::String(value) => {
                let parsed = value.parse::<f64>().map_err(DeError::custom)?;
                visitor.visit_f64(parsed)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_f64(visitor),
            other => Err(DeError::custom(format!("expected f64 got {:?}", other))),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::String(value) => visitor.visit_string(value),
            ParsedValue::Number(value) => visitor.visit_string(value),
            ParsedValue::Bool(value) => visitor.visit_string(value.to_string()),
            ParsedValue::Array(values) => {
                let hex = array_to_hex(values)?;
                visitor.visit_string(hex)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_string(visitor),
            other => Err(DeError::custom(format!("expected string got {:?}", other))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Null => visitor.visit_none(),
            ParsedValue::Tagged { value, .. } => value.deserialize_option(visitor),
            other => visitor.visit_some(other.into_deserializer()),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Array(values) => {
                let iter = values.into_iter().map(|v| v.into_deserializer());
                let mut seq = value::SeqDeserializer::new(iter);
                let output = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(output)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_seq(visitor),
            other => Err(DeError::custom(format!(
                "expected sequence got {:?}",
                other
            ))),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Object(map) => {
                let iter = map
                    .into_iter()
                    .map(|(k, v)| (k.into_deserializer(), v.into_deserializer()));
                let mut map_deserializer = value::MapDeserializer::new(iter);
                let output = visitor.visit_map(&mut map_deserializer)?;
                map_deserializer.end()?;
                Ok(output)
            }
            ParsedValue::Tagged { value, .. } => value.deserialize_map(visitor),
            other => Err(DeError::custom(format!("expected map got {:?}", other))),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::String(tag) | ParsedValue::Number(tag) => {
                visitor.visit_enum(TaggedEnumAccess {
                    tag,
                    value: ParsedValue::Null,
                })
            }
            ParsedValue::Tagged { tag, value } => {
                visitor.visit_enum(TaggedEnumAccess { tag, value: *value })
            }
            ParsedValue::Object(map) => {
                if map.len() == 1 {
                    let (tag, value) = map.into_iter().next().unwrap();
                    visitor.visit_enum(TaggedEnumAccess { tag, value })
                } else {
                    Err(value::Error::custom(format!(
                        "invalid enum representation {:?}",
                        map
                    )))
                }
            }
            other => Err(value::Error::custom(format!(
                "invalid enum representation {:?}",
                other
            ))),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Array(values) => {
                let mut buffer = Vec::with_capacity(values.len());
                for value in values {
                    let byte = match value {
                        ParsedValue::Number(s) | ParsedValue::String(s) => {
                            s.parse::<u8>().map_err(DeError::custom)?
                        }
                        other => {
                            return Err(DeError::custom(format!("expected byte got {:?}", other)));
                        }
                    };
                    buffer.push(byte);
                }
                visitor.visit_byte_buf(buffer)
            }
            ParsedValue::String(value) => visitor.visit_byte_buf(value.into_bytes()),
            ParsedValue::Tagged { value, .. } => value.deserialize_bytes(visitor),
            other => Err(DeError::custom(format!("expected bytes got {:?}", other))),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            ParsedValue::Null => visitor.visit_unit(),
            ParsedValue::Tagged { value, .. } => value.deserialize_unit(visitor),
            other => Err(DeError::custom(format!("expected unit got {:?}", other))),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    forward_to_deserialize_any! {
        char i8 i16 i32 i128 u8 u16 u32 u128
        unit_struct newtype_struct tuple tuple_struct
    }
}

fn format_timestamp(value: ParsedValue) -> Result<String, ParseError> {
    let (seconds, nanos) = parse_seconds_nanos(value, "Timestamp")?;
    if nanos < -999_999_999 || nanos > 999_999_999 {
        return Err(ParseError::new("timestamp nanos out of range"));
    }
    let total = seconds
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(nanos))
        .ok_or_else(|| ParseError::new("timestamp overflow"))?;
    let datetime = OffsetDateTime::from_unix_timestamp_nanos(total)
        .map_err(|_| ParseError::new("invalid timestamp"))?;
    Ok(to_rfc3339_nanos(datetime))
}

fn format_duration(value: ParsedValue) -> Result<String, ParseError> {
    let (seconds, nanos) = parse_seconds_nanos(value, "Duration")?;
    if seconds < 0 || nanos < 0 {
        return Err(ParseError::new("duration cannot be negative"));
    }
    if nanos >= 1_000_000_000 {
        return Err(ParseError::new("duration nanos out of range"));
    }
    let total = seconds
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(nanos))
        .ok_or_else(|| ParseError::new("duration overflow"))?;
    Ok(total.to_string())
}

fn parse_seconds_nanos(value: ParsedValue, kind: &str) -> Result<(i128, i128), ParseError> {
    let mut map = match value {
        ParsedValue::Object(map) => map,
        ParsedValue::Tagged { value, .. } => return parse_seconds_nanos(*value, kind),
        other => {
            return Err(ParseError::new(format!(
                "expected {} object, got {:?}",
                kind, other
            )));
        }
    };
    let seconds = parse_number_field(map.remove("seconds"), "seconds")?;
    let nanos = parse_number_field(map.remove("nanos"), "nanos")?;
    Ok((seconds, nanos))
}

fn parse_number_field(value: Option<ParsedValue>, field: &str) -> Result<i128, ParseError> {
    match value {
        Some(value) => parse_number_value(value, field),
        None => Ok(0),
    }
}

fn parse_number_value(value: ParsedValue, field: &str) -> Result<i128, ParseError> {
    match value {
        ParsedValue::Number(num) | ParsedValue::String(num) => num
            .parse::<i128>()
            .map_err(|_| ParseError::new(format!("invalid number for {}", field))),
        ParsedValue::Bool(value) => Ok(if value { 1 } else { 0 }),
        ParsedValue::Tagged { value, .. } => parse_number_value(*value, field),
        ParsedValue::Object(map) => Err(ParseError::new(format!(
            "expected numeric value for {} but got {:?}",
            field,
            ParsedValue::Object(map)
        ))),
        other => Err(ParseError::new(format!(
            "expected numeric value for {} but got {:?}",
            field, other
        ))),
    }
}

fn array_to_hex(values: Vec<ParsedValue>) -> Result<String, value::Error> {
    let mut bytes = Vec::with_capacity(values.len());
    for value in values {
        let byte = match value {
            ParsedValue::Number(num) | ParsedValue::String(num) => {
                let parsed = num.parse::<i64>().map_err(value::Error::custom)?;
                if !(0..=255).contains(&parsed) {
                    return Err(value::Error::custom("byte out of range"));
                }
                parsed as u8
            }
            other => {
                return Err(value::Error::custom(format!(
                    "expected byte got {:?}",
                    other
                )));
            }
        };
        bytes.push(byte);
    }
    Ok(hex_encode(bytes))
}
fn deserialize_to_loaded<T, F>(
    value: ParsedValue,
    wrap: F,
) -> Result<LoadedRequest, Box<dyn std::error::Error>>
where
    T: DeserializeOwned,
    F: FnOnce(T) -> LoadedRequest,
{
    let request = T::deserialize(value)?;
    Ok(wrap(request))
}
