use crate::replay::{LoadedRequest, ReplayItem};
use hex::encode as hex_encode;
use serde::de::value;
use serde::de::Error as DeError;
use serde::de::IgnoredAny;
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

fn parse_logged_request(raw: &str) -> Result<Option<LoadedRequest>, Box<dyn std::error::Error>> {
    let parser = DebugValueParser::new(raw);
    let parsed = parser
        .parse()
        .map_err(|err| format!("failed to parse request payload: {}", err))?;
    convert_parsed_request(parsed)
}

fn convert_parsed_request(
    value: ParsedValue,
) -> Result<Option<LoadedRequest>, Box<dyn std::error::Error>> {
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
) -> Result<Option<LoadedRequest>, Box<dyn std::error::Error>> {
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
    if let Some(suffix) = trimmed.strip_prefix("ConsensusVersion") {
        if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
            return ParsedValue::Number(suffix.to_string());
        }
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
    if !(-999_999_999..=999_999_999).contains(&nanos) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_from_log(raw: &str) -> LoadedRequest {
        parse_logged_request(raw)
            .expect("log line should parse")
            .expect("request should not be filtered out")
    }

    #[test]
    fn parses_info_requests_from_testnet_log() {
        let request_one = parse_from_log(TESTNET_INFO_ONE);
        assert!(matches!(request_one, LoadedRequest::Info(_)));
    }

    #[test]
    fn parses_init_chain_from_testnet_log() {
        let request = parse_from_log(TESTNET_INIT_CHAIN);
        assert!(matches!(request, LoadedRequest::InitChain(_)));
    }

    #[test]
    fn parses_process_proposal_samples_from_testnet_log() {
        let request_one = parse_from_log(TESTNET_PROCESS_PROPOSAL_ONE);
        let request_two = parse_from_log(TESTNET_PROCESS_PROPOSAL_TWO);
        assert!(matches!(request_one, LoadedRequest::Process(_)));
        assert!(matches!(request_two, LoadedRequest::Process(_)));
    }

    #[test]
    fn parses_finalize_block_samples_from_testnet_log() {
        let request_one = parse_from_log(TESTNET_FINALIZE_BLOCK_ONE);
        let request_two = parse_from_log(TESTNET_FINALIZE_BLOCK_TWO);
        assert!(matches!(request_one, LoadedRequest::Finalize(_)));
        assert!(matches!(request_two, LoadedRequest::Finalize(_)));
    }

    const TESTNET_INFO_ONE: &str = r#"Request { value: Some(Info(RequestInfo { version: "1.5.1", block_version: 14, p2p_version: 10, abci_version: "1.3.0" })) }"#;

    const TESTNET_INIT_CHAIN: &str = r#"Request { value: Some(InitChain(RequestInitChain { time: Some(Timestamp { seconds: 1764074708, nanos: 919000000 }), chain_id: "dash-testnet-51", consensus_params: Some(ConsensusParams { block: Some(BlockParams { max_bytes: 2097152, max_gas: 57631392000 }), evidence: Some(EvidenceParams { max_age_num_blocks: 100000, max_age_duration: Some(Duration { seconds: 172800, nanos: 0 }), max_bytes: 512000 }), validator: Some(ValidatorParams { pub_key_types: ["bls12381"], voting_power_threshold: None }), version: Some(VersionParams { app_version: 1, consensus_version: ConsensusVersion0 }), synchrony: Some(SynchronyParams { message_delay: Some(Duration { seconds: 32, nanos: 0 }), precision: Some(Duration { seconds: 0, nanos: 500000000 }) }), timeout: Some(TimeoutParams { propose: Some(Duration { seconds: 30, nanos: 0 }), propose_delta: Some(Duration { seconds: 1, nanos: 0 }), vote: Some(Duration { seconds: 2, nanos: 0 }), vote_delta: Some(Duration { seconds: 0, nanos: 500000000 }) }), abci: Some(AbciParams { recheck_tx: true }) }), validator_set: None, app_state_bytes: [], initial_height: 1, initial_core_height: 0 })) }"#;

    const TESTNET_PROCESS_PROPOSAL_ONE: &str = r#"Request { value: Some(ProcessProposal(RequestProcessProposal { txs: [], proposed_last_commit: Some(CommitInfo { round: 0, quorum_hash: [], block_signature: [], threshold_vote_extensions: [] }), misbehavior: [], hash: [8, 250, 2, 194, 126, 192, 57, 11, 163, 1, 228, 252, 126, 61, 126, 173, 179, 80, 200, 25, 62, 62, 98, 160, 147, 104, 151, 6, 227, 162, 11, 250], height: 1, round: 0, time: Some(Timestamp { seconds: 1721353209, nanos: 0 }), next_validators_hash: [67, 141, 67, 156, 14, 170, 45, 148, 50, 130, 83, 98, 80, 9, 168, 59, 20, 74, 245, 208, 222, 120, 248, 14, 0, 20, 181, 247, 167, 138, 75, 141], core_chain_locked_height: 1090319, core_chain_lock_update: Some(CoreChainLock { core_block_height: 1090319, core_block_hash: [188, 208, 158, 250, 148, 183, 195, 57, 139, 20, 13, 219, 224, 167, 123, 230, 49, 159, 25, 154, 160, 12, 51, 34, 68, 102, 209, 84, 48, 1, 0, 0], signature: [149, 121, 58, 158, 78, 248, 78, 170, 243, 117, 235, 255, 251, 46, 2, 74, 179, 31, 158, 246, 193, 93, 23, 193, 136, 122, 196, 15, 108, 247, 172, 6, 169, 101, 215, 149, 150, 32, 35, 97, 252, 255, 134, 112, 170, 172, 13, 222, 3, 228, 106, 52, 44, 158, 242, 165, 19, 40, 253, 34, 104, 130, 246, 229, 84, 55, 221, 223, 40, 152, 72, 43, 159, 46, 78, 5, 1, 158, 149, 145, 239, 220, 27, 196, 104, 176, 16, 54, 90, 38, 68, 184, 43, 57, 17, 245] }), proposer_pro_tx_hash: [5, 182, 135, 151, 131, 68, 250, 36, 51, 178, 170, 153, 212, 31, 100, 62, 45, 133, 129, 167, 137, 205, 194, 48, 132, 136, 156, 236, 165, 36, 78, 168], proposed_app_version: 1, version: Some(Consensus { block: 14, app: 1 }), quorum_hash: [0, 0, 0, 175, 88, 108, 85, 57, 26, 90, 172, 207, 12, 99, 142, 240, 100, 211, 71, 145, 227, 27, 243, 154, 158, 173, 160, 199, 112, 203, 8, 120] })) }"#;

    const TESTNET_PROCESS_PROPOSAL_TWO: &str = r#"
        Request { value: Some(ProcessProposal(RequestProcessProposal {
            txs: [
                [2, 0, 45, 78, 164, 90, 178, 144, 237, 162, 36, 16, 179, 136, 18, 243, 44, 66,
                 117, 63, 224, 169, 203, 74, 197, 21, 20, 112, 113, 175, 43, 58, 235, 155, 1, 0,
                 0, 0, 243, 207, 138, 132, 145, 226, 98, 94, 28, 101, 1, 87, 179, 136, 251, 237,
                 65, 245, 65, 210, 208, 250, 92, 95, 42, 14, 220, 187, 116, 60, 109, 126, 5, 14,
                 99, 111, 110, 116, 97, 99, 116, 82, 101, 113, 117, 101, 115, 116, 162, 161, 180,
                 172, 111, 239, 34, 234, 42, 26, 104, 232, 18, 54, 68, 179, 87, 135, 95, 107, 65,
                 44, 24, 16, 146, 129, 193, 70, 231, 178, 113, 188, 218, 18, 242, 101, 39, 134,
                 235, 177, 130, 11, 124, 221, 128, 91, 176, 191, 225, 147, 222, 242, 151, 236,
                 236, 119, 26, 9, 32, 3, 155, 51, 167, 142, 6, 16, 97, 99, 99, 111, 117, 110, 116,
                 82, 101, 102, 101, 114, 101, 110, 99, 101, 3, 252, 27, 90, 108, 236, 21, 101, 110,
                 99, 114, 121, 112, 116, 101, 100, 65, 99, 99, 111, 117, 110, 116, 76, 97, 98, 101,
                 108, 10, 48, 248, 254, 160, 126, 247, 196, 142, 207, 10, 98, 88, 135, 178, 200,
                 37, 183, 104, 217, 69, 81, 239, 30, 184, 149, 165, 180, 206, 62, 82, 172, 165,
                 148, 122, 215, 182, 102, 159, 96, 57, 23, 138, 78, 41, 202, 192, 88, 118, 155,
                 18, 101, 110, 99, 114, 121, 112, 116, 101, 100, 80, 117, 98, 108, 105, 99, 75, 101,
                 121, 10, 96, 119, 70, 51, 109, 72, 189, 126, 141, 103, 114, 93, 25, 171, 156, 67,
                 231, 69, 47, 111, 71, 201, 112, 14, 39, 163, 85, 103, 129, 55, 72, 255, 60, 11, 84,
                 118, 32, 32, 59, 37, 45, 157, 214, 105, 54, 239, 69, 2, 113, 133, 8, 59, 41, 161,
                 75, 22, 61, 13, 211, 49, 222, 72, 61, 1, 175, 251, 138, 74, 218, 228, 202, 199, 1,
                 212, 219, 106, 81, 12, 123, 204, 181, 33, 215, 246, 216, 4, 168, 10, 128, 39, 13,
                 98, 209, 118, 32, 60, 184, 17, 114, 101, 99, 105, 112, 105, 101, 110, 116, 75, 101,
                 121, 73, 110, 100, 101, 120, 3, 0, 14, 115, 101, 110, 100, 101, 114, 75, 101, 121,
                 73, 110, 100, 101, 120, 3, 2, 8, 116, 111, 85, 115, 101, 114, 73, 100, 16, 97, 76,
                 52, 201, 139, 195, 240, 166, 24, 149, 31, 14, 97, 49, 5, 152, 160, 173, 142, 194,
                 37, 0, 127, 31, 0, 31, 221, 247, 231, 178, 146, 240, 0, 0, 1, 65, 32, 27, 157, 91,
                 5, 79, 181, 46, 205, 236, 51, 129, 157, 4, 6, 188, 236, 188, 172, 164, 231, 164,
                 44, 201, 66, 104, 173, 34, 228, 80, 60, 212, 197, 5, 47, 41, 78, 154, 99, 29, 118,
                 227, 86, 165, 172, 247, 43, 246, 245, 194, 84, 41, 198, 99, 130, 36, 116, 66, 187,
                 56, 25, 160, 87, 213, 138],
                [2, 0, 45, 78, 164, 90, 178, 144, 237, 162, 36, 16, 179, 136, 18, 243, 44, 66,
                 117, 63, 224, 169, 203, 74, 197, 21, 20, 112, 113, 175, 43, 58, 235, 155, 1, 0,
                 0, 0, 23, 72, 90, 243, 202, 126, 196, 210, 19, 54, 234, 23, 177, 178, 215, 167,
                 129, 47, 66, 193, 218, 34, 147, 66, 196, 207, 97, 224, 245, 71, 183, 9, 6, 14, 99,
                 111, 110, 116, 97, 99, 116, 82, 101, 113, 117, 101, 115, 116, 162, 161, 180, 172,
                 111, 239, 34, 234, 42, 26, 104, 232, 18, 54, 68, 179, 87, 135, 95, 107, 65, 44,
                 24, 16, 146, 129, 193, 70, 231, 178, 113, 188, 112, 237, 137, 163, 235, 139, 117,
                 158, 116, 242, 14, 29, 130, 74, 80, 169, 171, 187, 92, 209, 189, 71, 123, 134, 23,
                 239, 26, 77, 73, 181, 204, 96, 6, 16, 97, 99, 99, 111, 117, 110, 116, 82, 101, 102,
                 101, 114, 101, 110, 99, 101, 3, 252, 8, 49, 150, 10, 21, 101, 110, 99, 114, 121,
                 112, 116, 101, 100, 65, 99, 99, 111, 117, 110, 116, 76, 97, 98, 101, 108, 10, 48,
                 164, 30, 75, 218, 213, 182, 166, 107, 190, 19, 180, 130, 228, 190, 22, 103, 105,
                 179, 192, 237, 213, 2, 126, 237, 158, 129, 98, 101, 236, 82, 40, 147, 165, 129,
                 70, 0, 29, 52, 143, 160, 58, 10, 236, 15, 56, 144, 224, 186, 18, 101, 110, 99, 114,
                 121, 112, 116, 101, 100, 80, 117, 98, 108, 105, 99, 75, 101, 121, 10, 96, 215, 58,
                 88, 42, 120, 211, 175, 41, 107, 102, 20, 249, 7, 50, 252, 46, 247, 204, 180, 39,
                 25, 7, 21, 243, 52, 77, 95, 48, 207, 57, 229, 118, 30, 133, 144, 91, 233, 3, 49,
                 164, 44, 194, 138, 30, 156, 11, 130, 80, 67, 117, 154, 30, 173, 134, 89, 38, 96, 2,
                 198, 229, 126, 157, 193, 7, 88, 16, 40, 3, 55, 187, 56, 168, 63, 160, 1, 199, 22,
                 54, 254, 6, 57, 104, 83, 86, 253, 16, 68, 66, 59, 111, 88, 37, 30, 49, 169, 180,
                 17, 114, 101, 99, 105, 112, 105, 101, 110, 116, 75, 101, 121, 73, 110, 100, 101,
                 120, 3, 0, 14, 115, 101, 110, 100, 101, 114, 75, 101, 121, 73, 110, 100, 101, 120,
                 3, 2, 8, 116, 111, 85, 115, 101, 114, 73, 100, 16, 196, 168, 68, 208, 100, 228,
                 139, 149, 204, 118, 244, 251, 67, 150, 233, 2, 230, 196, 230, 251, 123, 37, 91,
                 103, 36, 82, 228, 68, 124, 146, 62, 106, 0, 0, 1, 65, 31, 179, 111, 38, 201, 227,
                 221, 204, 150, 217, 184, 31, 64, 226, 82, 115, 119, 176, 0, 222, 67, 184, 189, 4,
                 188, 81, 57, 153, 140, 156, 32, 175, 129, 38, 61, 58, 107, 95, 142, 225, 152, 199,
                 186, 84, 188, 96, 9, 117, 65, 129, 190, 57, 24, 19, 206, 160, 123, 156, 141, 244,
                 247, 131, 25, 195, 240],
            ],
            proposed_last_commit: Some(CommitInfo {
                round: 0,
                quorum_hash: [0, 0, 1, 140, 150, 8, 55, 78, 73, 90, 22, 124, 90, 202, 73, 150, 16,
                              239, 91, 176, 76, 16, 181, 122, 236, 22, 197, 148, 202, 119, 77, 210],
                block_signature: [167, 217, 185, 35, 134, 188, 204, 114, 135, 31, 147, 221, 25, 172,
                                  21, 164, 212, 143, 70, 44, 40, 154, 69, 16, 86, 203, 154, 15, 178,
                                  35, 99, 179, 48, 33, 97, 122, 88, 145, 128, 177, 217, 39, 247, 107,
                                  67, 76, 68, 229, 8, 94, 197, 196, 13, 137, 165, 213, 181, 9, 147,
                                  68, 147, 173, 38, 105, 250, 176, 184, 88, 221, 229, 33, 180, 241,
                                  186, 127, 206, 105, 27, 124, 46, 191, 61, 110, 60, 237, 9, 83, 133,
                                  218, 103, 111, 218, 252, 150, 64, 84],
                threshold_vote_extensions: [],
            }),
            misbehavior: [],
            hash: [103, 112, 201, 57, 9, 25, 69, 8, 44, 216, 190, 152, 108, 87, 2, 168, 213, 242,
                   101, 49, 254, 70, 159, 120, 184, 44, 135, 82, 71, 117, 194, 8],
            height: 1763,
            round: 0,
            time: Some(Timestamp { seconds: 1724858882, nanos: 478000000 }),
            next_validators_hash: [29, 218, 86, 252, 12, 33, 231, 178, 231, 204, 7, 223, 50, 81, 89,
                                   121, 242, 146, 120, 127, 206, 60, 181, 255, 207, 242, 41, 32, 40,
                                   153, 108, 65],
            core_chain_locked_height: 1092300,
            core_chain_lock_update: None,
            proposer_pro_tx_hash: [136, 37, 27, 212, 177, 36, 239, 235, 135, 83, 125, 234, 190, 236,
                                   84, 246, 200, 245, 117, 244, 223, 129, 241, 12, 245, 232, 238,
                                   160, 115, 9, 43, 111],
            proposed_app_version: 1,
            version: Some(Consensus { block: 14, app: 1 }),
            quorum_hash: [0, 0, 1, 140, 150, 8, 55, 78, 73, 90, 22, 124, 90, 202, 73, 150, 16, 239,
                          91, 176, 76, 16, 181, 122, 236, 22, 197, 148, 202, 119, 77, 210],
        })) }
    "#;

    const TESTNET_FINALIZE_BLOCK_ONE: &str = r#"Request { value: Some(FinalizeBlock(RequestFinalizeBlock { commit: Some(CommitInfo { round: 0, quorum_hash: [0, 0, 0, 175, 88, 108, 85, 57, 26, 90, 172, 207, 12, 99, 142, 240, 100, 211, 71, 145, 227, 27, 243, 154, 158, 173, 160, 199, 112, 203, 8, 120], block_signature: [135, 31, 49, 75, 118, 108, 104, 68, 227, 56, 171, 253, 41, 152, 98, 72, 166, 144, 178, 146, 18, 67, 56, 20, 130, 110, 158, 62, 6, 77, 138, 57, 113, 138, 206, 99, 241, 112, 245, 11, 237, 77, 36, 214, 24, 174, 166, 1, 21, 165, 233, 228, 11, 201, 201, 241, 5, 23, 224, 16, 178, 200, 142, 7, 105, 222, 229, 179, 82, 115, 216, 243, 16, 48, 204, 162, 100, 154, 25, 120, 1, 85, 170, 219, 228, 3, 170, 60, 81, 141, 68, 116, 22, 7, 239, 249], threshold_vote_extensions: [] }), misbehavior: [], hash: [8, 250, 2, 194, 126, 192, 57, 11, 163, 1, 228, 252, 126, 61, 126, 173, 179, 80, 200, 25, 62, 62, 98, 160, 147, 104, 151, 6, 227, 162, 11, 250], height: 1, round: 0, block: Some(Block { header: Some(Header { version: Some(Consensus { block: 14, app: 1 }), chain_id: "dash-testnet-51", height: 1, time: Some(Timestamp { seconds: 1721353209, nanos: 0 }), last_block_id: Some(BlockId { hash: [], part_set_header: Some(PartSetHeader { total: 0, hash: [] }), state_id: [] }), last_commit_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], data_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], validators_hash: [151, 116, 141, 147, 83, 105, 231, 87, 207, 88, 182, 176, 208, 225, 89, 251, 214, 179, 23, 205, 112, 160, 49, 250, 160, 132, 171, 97, 31, 189, 74, 77], next_validators_hash: [67, 141, 67, 156, 14, 170, 45, 148, 50, 130, 83, 98, 80, 9, 168, 59, 20, 74, 245, 208, 222, 120, 248, 14, 0, 20, 181, 247, 167, 138, 75, 141], consensus_hash: [180, 208, 169, 232, 73, 81, 202, 221, 119, 226, 150, 68, 128, 3, 115, 113, 118, 178, 89, 72, 173, 246, 104, 172, 110, 192, 105, 38, 200, 31, 172, 134], next_consensus_hash: [180, 208, 169, 232, 73, 81, 202, 221, 119, 226, 150, 68, 128, 3, 115, 113, 118, 178, 89, 72, 173, 246, 104, 172, 110, 192, 105, 38, 200, 31, 172, 134], app_hash: [191, 12, 203, 156, 160, 113, 186, 1, 174, 110, 103, 160, 192, 144, 249, 120, 3, 210, 109, 86, 214, 117, 220, 213, 19, 23, 129, 203, 202, 200, 236, 143], results_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], evidence_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], proposed_app_version: 1, proposer_pro_tx_hash: [5, 182, 135, 151, 131, 68, 250, 36, 51, 178, 170, 153, 212, 31, 100, 62, 45, 133, 129, 167, 137, 205, 194, 48, 132, 136, 156, 236, 165, 36, 78, 168], core_chain_locked_height: 1090319 }), data: Some(Data { txs: [] }), evidence: Some(EvidenceList { evidence: [] }), last_commit: Some(Commit { height: 0, round: 0, block_id: Some(BlockId { hash: [], part_set_header: Some(PartSetHeader { total: 0, hash: [] }), state_id: [] }), quorum_hash: [], threshold_block_signature: [], threshold_vote_extensions: [] }), core_chain_lock: Some(CoreChainLock { core_block_height: 1090319, core_block_hash: [188, 208, 158, 250, 148, 183, 195, 57, 139, 20, 13, 219, 224, 167, 123, 230, 49, 159, 25, 154, 160, 12, 51, 34, 68, 102, 209, 84, 48, 1, 0, 0], signature: [149, 121, 58, 158, 78, 248, 78, 170, 243, 117, 235, 255, 251, 46, 2, 74, 179, 31, 158, 246, 193, 93, 23, 193, 136, 122, 196, 15, 108, 247, 172, 6, 169, 101, 215, 149, 150, 32, 35, 97, 252, 255, 134, 112, 170, 172, 13, 222, 3, 228, 106, 52, 44, 158, 242, 165, 19, 40, 253, 34, 104, 130, 246, 229, 84, 55, 221, 223, 40, 152, 72, 43, 159, 46, 78, 5, 1, 158, 149, 145, 239, 220, 27, 196, 104, 176, 16, 54, 90, 38, 68, 184, 43, 57, 17, 245] }) }), block_id: Some(BlockId { hash: [8, 250, 2, 194, 126, 192, 57, 11, 163, 1, 228, 252, 126, 61, 126, 173, 179, 80, 200, 25, 62, 62, 98, 160, 147, 104, 151, 6, 227, 162, 11, 250], part_set_header: Some(PartSetHeader { total: 1, hash: [67, 44, 224, 208, 150, 55, 48, 6, 49, 94, 42, 158, 112, 26, 240, 103, 185, 51, 202, 165, 106, 78, 124, 206, 99, 140, 58, 172, 22, 209, 227, 68] }), state_id: [209, 145, 66, 195, 68, 86, 204, 201, 247, 175, 60, 157, 38, 207, 138, 107, 56, 77, 204, 254, 146, 83, 246, 110, 152, 76, 39, 22, 153, 243, 181, 131] }) })) }"#;

    const TESTNET_FINALIZE_BLOCK_TWO: &str = r#"Request { value: Some(FinalizeBlock(RequestFinalizeBlock { commit: Some(CommitInfo { round: 1, quorum_hash: [0, 0, 0, 217, 55, 192, 219, 26, 45, 97, 224, 140, 152, 79, 68, 251, 208, 148, 3, 190, 152, 206, 230, 126, 107, 37, 117, 76, 217, 104, 133, 231], block_signature: [151, 132, 224, 84, 5, 251, 44, 38, 191, 110, 146, 82, 255, 249, 7, 150, 50, 168, 38, 237, 69, 227, 135, 55, 144, 47, 4, 155, 3, 138, 102, 99, 154, 133, 217, 187, 169, 154, 174, 115, 139, 173, 105, 25, 62, 73, 129, 87, 7, 28, 172, 121, 137, 254, 243, 31, 71, 185, 143, 151, 213, 225, 44, 199, 70, 25, 157, 230, 75, 62, 81, 254, 217, 86, 171, 220, 120, 188, 69, 24, 109, 222, 98, 224, 102, 201, 145, 82, 53, 13, 228, 150, 0, 137, 116, 16], threshold_vote_extensions: [] }), misbehavior: [], hash: [98, 147, 39, 112, 110, 207, 177, 37, 173, 61, 20, 160, 95, 228, 179, 90, 184, 83, 202, 86, 64, 254, 53, 133, 57, 32, 50, 21, 253, 32, 16, 172], height: 2, round: 1, block: Some(Block { header: Some(Header { version: Some(Consensus { block: 14, app: 1 }), chain_id: "dash-testnet-51", height: 2, time: Some(Timestamp { seconds: 1724575888, nanos: 328000000 }), last_block_id: Some(BlockId { hash: [8, 250, 2, 194, 126, 192, 57, 11, 163, 1, 228, 252, 126, 61, 126, 173, 179, 80, 200, 25, 62, 62, 98, 160, 147, 104, 151, 6, 227, 162, 11, 250], part_set_header: Some(PartSetHeader { total: 1, hash: [67, 44, 224, 208, 150, 55, 48, 6, 49, 94, 42, 158, 112, 26, 240, 103, 185, 51, 202, 165, 106, 78, 124, 206, 99, 140, 58, 172, 22, 209, 227, 68] }), state_id: [209, 145, 66, 195, 68, 86, 204, 201, 247, 175, 60, 157, 38, 207, 138, 107, 56, 77, 204, 254, 146, 83, 246, 110, 152, 76, 39, 22, 153, 243, 181, 131] }), last_commit_hash: [125, 172, 60, 185, 43, 201, 215, 188, 72, 11, 207, 101, 109, 98, 7, 127, 216, 155, 4, 101, 230, 18, 156, 15, 99, 93, 122, 164, 178, 27, 60, 194], data_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], validators_hash: [67, 141, 67, 156, 14, 170, 45, 148, 50, 130, 83, 98, 80, 9, 168, 59, 20, 74, 245, 208, 222, 120, 248, 14, 0, 20, 181, 247, 167, 138, 75, 141], next_validators_hash: [67, 141, 67, 156, 14, 170, 45, 148, 50, 130, 83, 98, 80, 9, 168, 59, 20, 74, 245, 208, 222, 120, 248, 14, 0, 20, 181, 247, 167, 138, 75, 141], consensus_hash: [180, 208, 169, 232, 73, 81, 202, 221, 119, 226, 150, 68, 128, 3, 115, 113, 118, 178, 89, 72, 173, 246, 104, 172, 110, 192, 105, 38, 200, 31, 172, 134], next_consensus_hash: [180, 208, 169, 232, 73, 81, 202, 221, 119, 226, 150, 68, 128, 3, 115, 113, 118, 178, 89, 72, 173, 246, 104, 172, 110, 192, 105, 38, 200, 31, 172, 134], app_hash: [77, 189, 142, 170, 172, 111, 216, 169, 207, 157, 71, 134, 71, 149, 189, 31, 53, 102, 178, 80, 54, 238, 37, 128, 18, 176, 16, 200, 2, 126, 109, 200], results_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], evidence_hash: [227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85], proposed_app_version: 1, proposer_pro_tx_hash: [20, 61, 205, 106, 107, 118, 132, 253, 224, 30, 136, 161, 14, 93, 101, 222, 154, 41, 36, 76, 94, 205, 88, 109, 20, 163, 66, 101, 112, 37, 241, 19], core_chain_locked_height: 1090320 }), data: Some(Data { txs: [] }), evidence: Some(EvidenceList { evidence: [] }), last_commit: Some(Commit { height: 1, round: 0, block_id: Some(BlockId { hash: [8, 250, 2, 194, 126, 192, 57, 11, 163, 1, 228, 252, 126, 61, 126, 173, 179, 80, 200, 25, 62, 62, 98, 160, 147, 104, 151, 6, 227, 162, 11, 250], part_set_header: Some(PartSetHeader { total: 1, hash: [67, 44, 224, 208, 150, 55, 48, 6, 49, 94, 42, 158, 112, 26, 240, 103, 185, 51, 202, 165, 106, 78, 124, 206, 99, 140, 58, 172, 22, 209, 227, 68] }), state_id: [209, 145, 66, 195, 68, 86, 204, 201, 247, 175, 60, 157, 38, 207, 138, 107, 56, 77, 204, 254, 146, 83, 246, 110, 152, 76, 39, 22, 153, 243, 181, 131] }), quorum_hash: [0, 0, 0, 175, 88, 108, 85, 57, 26, 90, 172, 207, 12, 99, 142, 240, 100, 211, 71, 145, 227, 27, 243, 154, 158, 173, 160, 199, 112, 203, 8, 120], threshold_block_signature: [135, 31, 49, 75, 118, 108, 104, 68, 227, 56, 171, 253, 41, 152, 98, 72, 166, 144, 178, 146, 18, 67, 56, 20, 130, 110, 158, 62, 6, 77, 138, 57, 113, 138, 206, 99, 241, 112, 245, 11, 237, 77, 36, 214, 24, 174, 166, 1, 21, 165, 233, 228, 11, 201, 201, 241, 5, 23, 224, 16, 178, 200, 142, 7, 105, 222, 229, 179, 82, 115, 216, 243, 16, 48, 204, 162, 100, 154, 25, 120, 1, 85, 170, 219, 228, 3, 170, 60, 81, 141, 68, 116, 22, 7, 239, 249], threshold_vote_extensions: [] }), core_chain_lock: Some(CoreChainLock { core_block_height: 1090320, core_block_hash: [26, 216, 69, 45, 191, 148, 30, 6, 168, 162, 186, 137, 78, 18, 75, 156, 96, 37, 84, 103, 102, 16, 247, 253, 135, 49, 45, 177, 223, 0, 0, 0], signature: [180, 194, 224, 93, 153, 74, 203, 51, 184, 6, 210, 221, 105, 118, 214, 63, 153, 251, 150, 114, 108, 127, 204, 60, 218, 225, 67, 219, 1, 243, 242, 197, 83, 108, 245, 71, 67, 13, 18, 17, 65, 133, 148, 4, 209, 211, 188, 233, 24, 183, 215, 105, 227, 42, 217, 128, 60, 80, 202, 225, 220, 209, 240, 9, 200, 195, 48, 13, 154, 13, 142, 216, 112, 83, 180, 84, 254, 238, 233, 201, 141, 58, 116, 171, 125, 110, 118, 14, 225, 89, 66, 58, 3, 9, 42, 179] }) }), block_id: Some(BlockId { hash: [98, 147, 39, 112, 110, 207, 177, 37, 173, 61, 20, 160, 95, 228, 179, 90, 184, 83, 202, 86, 64, 254, 53, 133, 57, 32, 50, 21, 253, 32, 16, 172], part_set_header: Some(PartSetHeader { total: 1, hash: [200, 235, 186, 216, 169, 238, 109, 8, 187, 66, 3, 161, 186, 230, 247, 3, 129, 181, 218, 148, 158, 153, 68, 17, 111, 14, 75, 230, 202, 104, 255, 47] }), state_id: [45, 73, 190, 78, 51, 91, 154, 204, 110, 181, 196, 96, 54, 5, 235, 82, 94, 133, 4, 141, 52, 209, 253, 137, 36, 137, 200, 161, 182, 225, 218, 147] }) })) }"#;
}
