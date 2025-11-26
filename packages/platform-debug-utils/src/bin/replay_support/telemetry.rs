use hex;
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::OpenOptions;
use std::path::Path;
use textwrap::Options;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;

const TRACE_WRAP_WIDTH: usize = 120;

pub fn init_logging(log_output: Option<&Path>) -> Result<Option<WorkerGuard>, Box<dyn Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json_format = PrettyJsonEventFormat::new(TRACE_WRAP_WIDTH);
    let fmt_builder = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .event_format(json_format);

    if let Some(path) = log_output {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let (writer, guard) = tracing_appender::non_blocking(file);
        fmt_builder
            .with_writer(writer)
            .try_init()
            .map_err(ErrorString::from)?;
        Ok(Some(guard))
    } else {
        fmt_builder
            .with_writer(std::io::stderr)
            .try_init()
            .map_err(ErrorString::from)?;
        Ok(None)
    }
}

#[derive(Debug)]
struct ErrorString(String);
impl Display for ErrorString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for ErrorString {}

impl From<Box<dyn Error + Send + Sync + 'static>> for ErrorString {
    fn from(value: Box<dyn Error + Send + Sync + 'static>) -> Self {
        ErrorString(value.to_string())
    }
}

struct PrettyJsonEventFormat {
    width: usize,
}

impl PrettyJsonEventFormat {
    fn new(width: usize) -> Self {
        Self { width }
    }
}

impl<S, N> FormatEvent<S, N> for PrettyJsonEventFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut field_visitor = JsonFieldsVisitor::default();
        event.record(&mut field_visitor);
        let fields = field_visitor.finish();

        let mut payload = JsonMap::new();
        payload.insert(
            "level".into(),
            JsonValue::String(event.metadata().level().to_string()),
        );
        payload.insert(
            "target".into(),
            JsonValue::String(event.metadata().target().to_string()),
        );

        if let Some(file) = event.metadata().file() {
            payload.insert("file".into(), JsonValue::String(file.to_string()));
        }

        if let Some(line) = event.metadata().line() {
            payload.insert("line".into(), JsonValue::Number(JsonNumber::from(line)));
        }

        if !fields.is_empty() {
            payload.insert("fields".into(), JsonValue::Object(fields));
        }

        let spans = collect_span_names(ctx);
        if !spans.is_empty() {
            payload.insert(
                "spans".into(),
                JsonValue::Array(spans.into_iter().map(JsonValue::String).collect()),
            );
        }

        let json =
            serde_json::to_string_pretty(&JsonValue::Object(payload)).map_err(|_| fmt::Error)?;

        write_wrapped(writer, &json, self.width)
    }
}

fn write_wrapped(mut writer: Writer<'_>, text: &str, width: usize) -> fmt::Result {
    if text.is_empty() {
        writer.write_char('\n')?;
        return Ok(());
    }

    let mut remaining = text;

    while let Some(idx) = remaining.find('\n') {
        let line = &remaining[..idx];
        emit_wrapped_line(&mut writer, line, width)?;
        remaining = &remaining[idx + 1..];
    }

    if !remaining.is_empty() {
        emit_wrapped_line(&mut writer, remaining, width)?;
    }

    Ok(())
}

fn emit_wrapped_line(writer: &mut Writer<'_>, line: &str, width: usize) -> fmt::Result {
    if line.is_empty() {
        writer.write_char('\n')?;
        return Ok(());
    }

    let indent_end = line
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| line.len());
    let (indent, content) = line.split_at(indent_end);

    if content.is_empty() {
        writer.write_str(indent)?;
        writer.write_char('\n')?;
        return Ok(());
    }

    let indent_chars = indent.chars().count();
    let mut effective_width = width.saturating_sub(indent_chars);
    if effective_width == 0 {
        effective_width = 1;
    }

    let wrap_options = Options::new(effective_width).break_words(false);
    let wrapped = textwrap::wrap(content, &wrap_options);
    for piece in wrapped {
        if !indent.is_empty() {
            writer.write_str(indent)?;
        }
        writer.write_str(piece.as_ref())?;
        writer.write_char('\n')?;
    }

    Ok(())
}

fn collect_span_names<S, N>(ctx: &FmtContext<'_, S, N>) -> Vec<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    let mut names = Vec::new();
    let mut current = ctx.lookup_current();
    while let Some(span) = current {
        names.push(span.name().to_string());
        current = span.parent();
    }
    names.reverse();
    names
}

#[derive(Default)]
struct JsonFieldsVisitor {
    fields: JsonMap<String, JsonValue>,
}

impl JsonFieldsVisitor {
    fn finish(self) -> JsonMap<String, JsonValue> {
        self.fields
    }

    fn insert_value(&mut self, field: &Field, value: JsonValue) {
        self.fields.insert(field.name().to_string(), value);
    }
}

impl Visit for JsonFieldsVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert_value(field, JsonValue::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert_value(field, JsonValue::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert_value(field, JsonValue::Number(value.into()));
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.insert_value(field, JsonValue::String(value.to_string()));
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.insert_value(field, JsonValue::String(value.to_string()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        let json_value = JsonNumber::from_f64(value)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null);
        self.insert_value(field, json_value);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert_value(field, JsonValue::String(value.to_owned()));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.insert_value(field, JsonValue::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.insert_value(field, JsonValue::String(format!("{value:?}")));
    }

    fn record_bytes(&mut self, field: &Field, value: &[u8]) {
        self.insert_value(field, JsonValue::String(hex::encode(value)));
    }
}
