mod case_folding;

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::schema::validation_schema::{
    regex_syntax_parser, validator_regex_syntax, validator_regex_within,
};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::DocumentName;
use crate::prelude::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use case_folding::case_folded_code_points;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use regex::RegexBuilder;
use regex_syntax::ast::Ast;
use std::collections::{BTreeMap, BTreeSet};

/// The key a property schema holds its pattern under.
const PATTERN: &str = "pattern";
/// The key a `fieldMatches` entry of a contested index holds its pattern under.
const REGEX_PATTERN: &str = "regexPattern";
const REFERENCE: &str = "$ref";
const IDENTIFIER: &str = "$id";
const PATTERN_PROPERTIES: &str = "patternProperties";
const FORMAT: &str = "format";
const REGEX_FORMAT: &str = "regex";

/// What compiles a pattern, each in its own dialect.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Compiler {
    /// The document validator, for a `pattern`.
    DocumentValidator,
    /// The index parser, for a `regexPattern`.
    IndexParser,
}

impl Compiler {
    fn compiles(self, pattern: &str, size_limit: usize) -> bool {
        match self {
            Compiler::DocumentValidator => {
                validator_regex_within(pattern, Some(size_limit)).is_some()
            }
            Compiler::IndexParser => RegexBuilder::new(pattern)
                .size_limit(size_limit)
                .build()
                .is_ok(),
        }
    }

    /// The syntax tree of `pattern` as this compiler reads it before it compiles it, `None`
    /// when it does not parse: then it does not compile either.
    fn syntax(self, pattern: &str) -> Option<Ast> {
        match self {
            Compiler::DocumentValidator => validator_regex_syntax(pattern),
            Compiler::IndexParser => regex_syntax_parser().parse(pattern).ok(),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Compiler::DocumentValidator => PATTERN,
            Compiler::IndexParser => REGEX_PATTERN,
        }
    }
}

/// What compiles the patterns of `$defs`, which is what their count follows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DefinitionsCount {
    /// Once for each document type. The native schema check reads every definition for each
    /// type it checks, and the native document validator, which keeps one compiled pattern
    /// for a type however many of its `$ref`s reach it, compiles no more than those.
    OncePerDocumentType,
    /// Once for each document type, or once for each `$ref` reaching one where that is more:
    /// the JSON Schema validator compiles what a `$ref` names when a document first reaches
    /// that `$ref`, and keeps it there. It resolves a `$ref` through the `$id`s of a schema
    /// before its pointer, so a contract holding a pattern is refused with a `$ref` or an
    /// `$id` this count cannot follow. It also compiles each key of a `patternProperties`,
    /// which the validation schema that replaces it refuses, so that is refused too.
    OncePerReference,
}

impl DefinitionsCount {
    fn counted_for_each(self) -> &'static str {
        match self {
            DefinitionsCount::OncePerDocumentType => "document type",
            DefinitionsCount::OncePerReference => "document type, or for each `$ref` to it",
        }
    }
}

/// What one document schema, or one definition of `$defs`, holds.
#[derive(Default)]
struct Found<'a> {
    patterns: Vec<(Compiler, &'a str)>,
    /// The definition each `$ref` names.
    references: Vec<&'a str>,
    /// Holds a `$ref` that is no plain pointer into `$defs`, or an `$id` that is no plain
    /// name: the JSON Schema validator may resolve a `$ref` elsewhere than its pointer names.
    moves_a_reference: bool,
    holds_pattern_properties: bool,
    declares_regex_format: bool,
}

impl<'a> Found<'a> {
    /// The bytes of the patterns found, as written.
    fn pattern_bytes(&self) -> usize {
        self.patterns
            .iter()
            .map(|(_, pattern)| pattern.len())
            .fold(0, usize::saturating_add)
    }

    /// Every string `root` holds under one of the keys above, at any depth. Not recursive:
    /// nothing has bounded the depth of the value yet.
    fn within(root: &'a Value) -> Self {
        let mut found = Found::default();
        let mut pending = vec![root];
        while let Some(value) = pending.pop() {
            match value {
                Value::Map(entries) => {
                    for (key, inner) in entries {
                        if key.as_text() == Some(PATTERN_PROPERTIES)
                            && matches!(inner, Value::Map(keys) if !keys.is_empty())
                        {
                            found.holds_pattern_properties = true;
                        }
                        if let (Some(key), Value::Text(text)) = (key.as_text(), inner) {
                            match key {
                                PATTERN => found.patterns.push((Compiler::DocumentValidator, text)),
                                REGEX_PATTERN => found.patterns.push((Compiler::IndexParser, text)),
                                REFERENCE => match referenced_definition(text) {
                                    Some(definition) => found.references.push(definition),
                                    None => found.moves_a_reference = true,
                                },
                                IDENTIFIER if !is_plain_name(text) => {
                                    found.moves_a_reference = true
                                }
                                FORMAT if text == REGEX_FORMAT => {
                                    found.declares_regex_format = true
                                }
                                _ => {}
                            }
                        }
                    }
                    pending.extend(entries.iter().rev().map(|(_, inner)| inner));
                }
                Value::Array(items) => pending.extend(items.iter().rev()),
                _ => {}
            }
        }
        found
    }
}

/// The definition a `$ref` names, when it is a plain pointer into `$defs`: one a URL parser
/// and a JSON pointer both read as it is written.
fn referenced_definition(reference: &str) -> Option<&str> {
    let pointer = reference.strip_prefix("#/$defs/")?;
    let name = pointer.split('/').next()?;
    pointer.split('/').all(is_plain_name).then_some(name)
}

/// What an `$id` must be for the JSON Schema validator to resolve no `$ref` through it or
/// against it, as the validation schema of protocol version 15 requires of one.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'$'))
}

/// The definitions of `$defs`.
struct Definitions<'a> {
    definitions: BTreeMap<&'a str, Found<'a>>,
    /// Every pattern the definitions hold.
    patterns: usize,
    /// The bytes of every pattern the definitions hold.
    pattern_bytes: usize,
}

impl<'a> Definitions<'a> {
    fn new(schema_defs: Option<&'a BTreeMap<String, Value>>) -> Self {
        let definitions: BTreeMap<&str, Found> = schema_defs
            .into_iter()
            .flatten()
            .map(|(name, definition)| (name.as_str(), Found::within(definition)))
            .collect();
        let patterns = definitions.values().map(|found| found.patterns.len()).sum();
        let pattern_bytes = definitions
            .values()
            .map(Found::pattern_bytes)
            .fold(0, usize::saturating_add);
        Definitions {
            definitions,
            patterns,
            pattern_bytes,
        }
    }

    /// For each definition, how many patterns compiling what a `$ref` to it names compiles,
    /// when each `$ref` met on the way compiles what it names in turn: its own, and those of
    /// every `$ref` it holds. Counted up to `ceiling` and no further, which a cycle of
    /// definitions reaching a pattern arrives at.
    fn patterns_compiled_through_each(&self, ceiling: usize) -> BTreeMap<&'a str, usize> {
        let mut compiled: BTreeMap<&str, usize> = BTreeMap::new();
        let mut referrers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        // Each count still to add to what the definitions referring to one compile
        let mut pending: Vec<(&str, usize)> = Vec::new();
        for (&name, found) in &self.definitions {
            for &definition in &found.references {
                referrers.entry(definition).or_default().push(name);
            }
            let own = found.patterns.len().min(ceiling);
            compiled.insert(name, own);
            if own > 0 {
                pending.push((name, own));
            }
        }
        // Every step raises a count below the ceiling by at least one, so this ends
        while let Some((name, added)) = pending.pop() {
            for &referrer in referrers.get(name).into_iter().flatten() {
                let Some(count) = compiled.get_mut(referrer) else {
                    continue;
                };
                let raised = count.saturating_add(added).min(ceiling);
                if raised > *count {
                    pending.push((referrer, raised - *count));
                    *count = raised;
                }
            }
        }
        compiled
    }
}

impl DataContract {
    /// Refuses, without compiling any pattern, more than `max_contract_schema_patterns`
    /// patterns, a pattern longer than `max_schema_pattern_bytes`, patterns longer than
    /// `max_contract_schema_pattern_bytes` together (one in `$defs` once for each document
    /// type), and
    /// with `refuse_regex_format` a schema declaring `format: regex`; then, still compiling
    /// none, the first that does not parse or that case-folds more than
    /// `max_schema_pattern_case_folded_code_points` code points; and only then the first that
    /// does not compile within `max_schema_pattern_compiled_size`, each distinct pattern read
    /// and compiled once.
    #[inline(always)]
    pub(super) fn validate_schema_patterns_v0(
        document_schemas: &BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        definitions_count: DefinitionsCount,
        refuse_regex_format: bool,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let max_patterns = usize::from(platform_version.system_limits.max_contract_schema_patterns);
        let max_pattern_bytes =
            usize::from(platform_version.system_limits.max_schema_pattern_bytes);
        let max_contract_pattern_bytes = platform_version
            .system_limits
            .max_contract_schema_pattern_bytes as usize;
        let max_case_folded = u64::from(
            platform_version
                .system_limits
                .max_schema_pattern_case_folded_code_points,
        );
        let size_limit = platform_version
            .system_limits
            .max_schema_pattern_compiled_size as usize;

        let definitions = Definitions::new(schema_defs);
        let document_types: Vec<Found> = document_schemas.values().map(Found::within).collect();

        // One past the limit is as far as a count needs to go to be refused
        let ceiling = max_patterns.saturating_add(1);
        let compiled_through_each = match definitions_count {
            DefinitionsCount::OncePerDocumentType => None,
            DefinitionsCount::OncePerReference => {
                Some(definitions.patterns_compiled_through_each(ceiling))
            }
        };
        // The definitions' patterns count once in a contract with no document type
        let mut count = if document_types.is_empty() {
            definitions.patterns
        } else {
            0
        };
        for document_type in &document_types {
            let of_definitions = match &compiled_through_each {
                None => definitions.patterns,
                Some(compiled_through_each) => document_type
                    .references
                    .iter()
                    .map(|definition| compiled_through_each.get(definition).copied().unwrap_or(0))
                    .fold(0usize, usize::saturating_add)
                    .max(definitions.patterns),
            };
            count = count
                .saturating_add(document_type.patterns.len())
                .saturating_add(of_definitions);
        }
        // The bytes of the patterns, those of the definitions once for each document type, as the
        // schema check reads every definition for each type it checks, and once in a contract
        // with no document type. Not followed through `$ref`s: the count above already holds what
        // they compile to `max_contract_schema_patterns` patterns of at most
        // `max_schema_pattern_bytes` each
        let bytes = if document_types.is_empty() {
            definitions.pattern_bytes
        } else {
            document_types
                .iter()
                .map(|document_type| {
                    document_type
                        .pattern_bytes()
                        .saturating_add(definitions.pattern_bytes)
                })
                .fold(0, usize::saturating_add)
        };
        if count > max_patterns {
            return refusal(format!(
                "the document schemas and `$defs` of a contract may hold at most {max_patterns} \
                 patterns (`{PATTERN}` and `{REGEX_PATTERN}`), one in `$defs` counted for each {}",
                definitions_count.counted_for_each()
            ));
        }

        let schemas = || {
            document_types
                .iter()
                .chain(definitions.definitions.values())
        };
        let patterns = || schemas().flat_map(|found| &found.patterns);
        if let Some((compiler, pattern)) =
            patterns().find(|(_, pattern)| pattern.len() > max_pattern_bytes)
        {
            return refusal(format!(
                "a `{}` may be at most {max_pattern_bytes} bytes long, and one of the contract's \
                 is {} bytes long",
                compiler.key(),
                pattern.len()
            ));
        }
        if bytes > max_contract_pattern_bytes {
            return refusal(format!(
                "the document schemas and `$defs` of a contract may hold patterns of at most \
                 {max_contract_pattern_bytes} bytes together (`{PATTERN}` and `{REGEX_PATTERN}`), \
                 one in `$defs` counted for each document type"
            ));
        }
        if definitions_count == DefinitionsCount::OncePerReference
            && schemas().any(|found| found.holds_pattern_properties)
        {
            return refusal(format!(
                "a schema holding `{PATTERN_PROPERTIES}` is not accepted: the JSON Schema \
                 validator compiles each of its keys as a pattern, which is not counted"
            ));
        }
        if definitions_count == DefinitionsCount::OncePerReference
            && count > 0
            && schemas().any(|found| found.moves_a_reference)
        {
            return refusal(format!(
                "a contract holding a pattern is not accepted with a `{REFERENCE}` that is not \
                 `#/$defs/` and a definition's name, or an `{IDENTIFIER}` that is not a plain \
                 name: what the JSON Schema validator compiles through either is not counted"
            ));
        }
        if refuse_regex_format && schemas().any(|found| found.declares_regex_format) {
            return refusal(format!(
                "a schema declaring `{FORMAT}: {REGEX_FORMAT}` is not accepted: every value of \
                 such a property is compiled as a pattern when a document is checked"
            ));
        }

        // Every pattern is read before any is compiled: what a pattern's syntax shows bounds
        // the time compiling it takes, which its compiled size does not
        let mut read = BTreeSet::new();
        for &(compiler, pattern) in patterns() {
            if !read.insert((compiler, pattern)) {
                continue;
            }
            let Some(syntax) = compiler.syntax(pattern) else {
                return not_compiling(compiler, pattern, size_limit);
            };
            if case_folded_code_points(&syntax) > max_case_folded {
                return refusal(format!(
                    "the `{}` `{pattern}` matches case-insensitively and may fold more than \
                     {max_case_folded} code points: a class naming a Unicode property, and a \
                     negated or Perl class inside a bracketed class, count as every code point, \
                     a range as the code points it spans, once more for each class around it",
                    compiler.key()
                ));
            }
        }

        let mut compiled = BTreeSet::new();
        for &(compiler, pattern) in patterns() {
            if compiled.insert((compiler, pattern)) && !compiler.compiles(pattern, size_limit) {
                return not_compiling(compiler, pattern, size_limit);
            }
        }
        SimpleConsensusValidationResult::new()
    }
}

fn not_compiling(
    compiler: Compiler,
    pattern: &str,
    size_limit: usize,
) -> SimpleConsensusValidationResult {
    refusal(format!(
        "the `{}` `{pattern}` is not a regular expression that compiles within {size_limit} bytes",
        compiler.key()
    ))
}

fn refusal(message: String) -> SimpleConsensusValidationResult {
    SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(
        BasicError::ContractError(DataContractError::InvalidContractStructure(message)),
    ))
}
