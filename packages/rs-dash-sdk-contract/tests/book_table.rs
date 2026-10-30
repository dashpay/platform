//! The grammar table in the book chapter is checked against `ATTRIBUTES`:
//! every attribute row names exactly the options the grammar declares, in
//! grammar order, so an edit to either side without the other fails here.
//!
//! The chapter is read from the repository at test time, so this test runs
//! only where the repository layout exists (it is skipped when the book file
//! is absent, for example in a published crate).

#![cfg(feature = "std")]

use dash_sdk_contract::grammar::ATTRIBUTES;

const CHAPTER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../book/src/dashvm/contract-declarations.md"
);

/// Option names as they appear in a table cell: every backtick span at
/// parenthesis depth zero whose first character starts an identifier, keeping
/// the identifier only (`sum = "<p>"` yields `sum`, `fields(<path> = "asc",
/// ...)` yields `fields`). Spans inside parentheses are the values an option
/// admits (`(`any` / `owner`)`), not options, which is the table's convention.
fn options_in_cell(cell: &str) -> Vec<String> {
    let mut options = Vec::new();
    let mut depth = 0usize;
    let mut chars = cell.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '`' => {
                let mut span = String::new();
                for c in chars.by_ref() {
                    if c == '`' {
                        break;
                    }
                    span.push(c);
                }
                let identifier: String = span
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                let starts_identifier = span.chars().next().is_some_and(|c| c.is_ascii_lowercase());
                if depth == 0 && starts_identifier && !options.contains(&identifier) {
                    options.push(identifier);
                }
            }
            _ => {}
        }
    }
    options
}

fn table_rows(chapter: &str) -> Vec<(String, Vec<String>)> {
    chapter
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split(" | ").collect();
            let [name, _target, options] = cells.as_slice() else {
                return None;
            };
            let name = name.trim().trim_matches('`').to_string();
            Some((name, options_in_cell(options)))
        })
        .collect()
}

#[test]
fn should_keep_the_book_grammar_table_equal_to_the_grammar() {
    let Ok(chapter) = std::fs::read_to_string(CHAPTER) else {
        eprintln!("book chapter not found at {CHAPTER}; skipping");
        return;
    };
    let rows = table_rows(&chapter);
    let attribute_rows: Vec<&(String, Vec<String>)> = rows
        .iter()
        .filter(|(name, _)| ATTRIBUTES.iter().any(|spec| spec.name == name))
        .collect();
    assert_eq!(
        attribute_rows.len(),
        ATTRIBUTES.len(),
        "the book table has {} attribute rows, the grammar {}",
        attribute_rows.len(),
        ATTRIBUTES.len()
    );
    for (spec, (name, options)) in ATTRIBUTES.iter().zip(attribute_rows) {
        assert_eq!(
            spec.name, name,
            "attribute row order differs from the grammar"
        );
        let declared: Vec<&str> = spec.keys.iter().map(|key| key.name).collect();
        // `rule` spells its exclusive pair as prose ("exactly one of"), the
        // identifiers still appear in grammar order.
        assert_eq!(
            options, &declared,
            "options of `{name}` in the book differ from the grammar"
        );
    }
}
