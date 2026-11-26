//! What the node that wrote the log answered to `ProcessProposal`.
//!
//! The log a replay reads its requests from holds the node's answers as well: to each
//! proposal the status, the app hash, and every state transition's result code and `gas_used`
//! (its storage fee plus its processing fee). A replay holds its own answer to the same
//! request against them. That shows the replay answering as that one node did, and no more:
//! how far the node's answers are the chain's rests on the node, which a log does not prove.
//!
//! The answer is logged as text and comes in two forms, by how the logging node was built:
//! as JSON, or as the keys each followed by the `Debug` form of its value in quotes, for
//! example `"status" " "Accepted"", "app_hash" " "26a5…"", "tx_results" " ["…"]",`. Both are
//! then logged through `Debug` once more.

use serde::{Deserialize, Serialize};
use std::str::CharIndices;

/// A state transition's result code and `gas_used`, as `ProcessProposal` answers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TxOutcome(pub(super) u32, pub(super) i64);

/// The part of a logged `ProcessProposal` answer a replay is held against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChainProposalAnswer {
    /// `Accepted`, `Rejected` or `Unknown`
    pub(super) status: String,
    /// In hex
    pub(super) app_hash: String,
    pub(super) tx: Vec<TxOutcome>,
}

/// The logged answer that follows a `ProcessProposal` request in the log.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum LoggedAnswer {
    /// The log has no answer to the request, or the node answered it with an error
    #[default]
    Absent,
    /// The log has an answer that cannot be read, or cannot be told to be this request's, and
    /// why
    Unreadable(String),
    /// The node's answer
    Proposal(ChainProposalAnswer),
}

/// Reads the `response_log` field of a `sending ABCI response` line. `None` when the line
/// answers another kind of request.
pub(super) fn parse_response_log(
    response_log: &str,
) -> Result<Option<ChainProposalAnswer>, String> {
    if !response_log.starts_with('"') {
        return Ok(None);
    }
    let mut scanner = Scanner { rest: response_log };
    let text = scanner.string()?;
    if !scanner.rest.trim().is_empty() {
        return Err("text follows the logged answer".to_owned());
    }

    if text.starts_with('{') {
        parse_json_answer(&text)
    } else if text.starts_with("\"status\"") {
        parse_debug_answer(&text).map(Some)
    } else {
        Ok(None)
    }
}

#[derive(Deserialize)]
struct JsonAnswer {
    status: Option<String>,
    app_hash: Option<String>,
    tx_results: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct JsonTxResult {
    code: u32,
    gas_used: i64,
}

fn parse_json_answer(text: &str) -> Result<Option<ChainProposalAnswer>, String> {
    let answer: JsonAnswer =
        serde_json::from_str(text).map_err(|error| format!("answer is not JSON: {error}"))?;
    // Only the answer to `ProcessProposal` has a status
    let Some(status) = answer.status else {
        return Ok(None);
    };
    let app_hash = answer
        .app_hash
        .ok_or_else(|| "answer has no app_hash".to_owned())?;
    let tx = answer
        .tx_results
        .ok_or_else(|| "answer has no tx_results".to_owned())?
        .iter()
        .map(|result| {
            serde_json::from_str::<JsonTxResult>(result)
                .map(|result| TxOutcome(result.code, result.gas_used))
                .map_err(|error| format!("transaction result is not JSON: {error}"))
        })
        .collect::<Result<_, _>>()?;
    Ok(Some(ChainProposalAnswer {
        status,
        app_hash,
        tx,
    }))
}

fn parse_debug_answer(text: &str) -> Result<ChainProposalAnswer, String> {
    let mut scanner = Scanner { rest: text };
    scanner.key("status")?;
    let status = scanner.string()?;
    scanner.end_of_value()?;
    scanner.key("app_hash")?;
    let app_hash = scanner.string()?;
    scanner.end_of_value()?;
    scanner.key("tx_results")?;
    scanner.expect("[")?;
    let mut tx = Vec::new();
    loop {
        scanner.skip_whitespace();
        if scanner.take("]") {
            break;
        }
        tx.push(parse_debug_tx_result(&scanner.string()?)?);
        scanner.skip_whitespace();
        scanner.take(",");
    }
    scanner.end_of_value()?;
    Ok(ChainProposalAnswer {
        status,
        app_hash,
        tx,
    })
}

fn parse_debug_tx_result(text: &str) -> Result<TxOutcome, String> {
    let mut scanner = Scanner { rest: text };
    scanner.key("code")?;
    let code = scanner.integer()?;
    let code = u32::try_from(code).map_err(|_| format!("result code {code} is out of range"))?;
    scanner.end_of_value()?;
    for name in ["data", "log", "info"] {
        scanner.key(name)?;
        scanner.string()?;
        scanner.end_of_value()?;
    }
    scanner.key("gas_used")?;
    let gas_used = scanner.integer()?;
    scanner.end_of_value()?;
    Ok(TxOutcome(code, gas_used))
}

/// Reads the keys and `Debug` values of a logged answer from the front of a text.
struct Scanner<'a> {
    rest: &'a str,
}

impl Scanner<'_> {
    fn skip_whitespace(&mut self) {
        self.rest = self.rest.trim_start();
    }

    fn take(&mut self, token: &str) -> bool {
        match self.rest.strip_prefix(token) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        if self.take(token) {
            Ok(())
        } else {
            Err(format!("expected `{}` before `{}`", token, self.next_few()))
        }
    }

    fn next_few(&self) -> String {
        self.rest.chars().take(24).collect()
    }

    /// A key in quotes and the quote that opens its value: `"status" " `
    fn key(&mut self, name: &str) -> Result<(), String> {
        self.skip_whitespace();
        self.expect("\"")?;
        self.expect(name)?;
        self.expect("\"")?;
        self.skip_whitespace();
        self.expect("\"")?;
        self.skip_whitespace();
        Ok(())
    }

    /// The quote that closes a value, and the comma after it
    fn end_of_value(&mut self) -> Result<(), String> {
        self.expect("\"")?;
        self.skip_whitespace();
        self.take(",");
        Ok(())
    }

    fn integer(&mut self) -> Result<i64, String> {
        let length = self
            .rest
            .char_indices()
            .find(|(index, character)| {
                !(character.is_ascii_digit() || (*index == 0 && *character == '-'))
            })
            .map(|(index, _)| index)
            .unwrap_or(self.rest.len());
        let (digits, rest) = self.rest.split_at(length);
        let value = digits
            .parse()
            .map_err(|_| format!("expected a number before `{}`", self.next_few()))?;
        self.rest = rest;
        Ok(value)
    }

    /// A string in its `Debug` form, quotes and escapes included
    fn string(&mut self) -> Result<String, String> {
        self.expect("\"")?;
        let mut text = String::new();
        let mut characters = self.rest.char_indices();
        while let Some((index, character)) = characters.next() {
            match character {
                '"' => {
                    self.rest = &self.rest[index + 1..];
                    return Ok(text);
                }
                '\\' => match characters.next() {
                    Some((_, 'n')) => text.push('\n'),
                    Some((_, 'r')) => text.push('\r'),
                    Some((_, 't')) => text.push('\t'),
                    Some((_, '0')) => text.push('\0'),
                    Some((_, escaped @ ('\\' | '"' | '\''))) => text.push(escaped),
                    Some((_, 'u')) => text.push(unicode_escape(&mut characters)?),
                    Some((_, other)) => return Err(format!("unknown escape `\\{other}`")),
                    None => break,
                },
                other => text.push(other),
            }
        }
        Err("a string does not end".to_owned())
    }
}

/// The character of a `\u{…}` escape, read from after its `u`.
fn unicode_escape(characters: &mut CharIndices<'_>) -> Result<char, String> {
    if !matches!(characters.next(), Some((_, '{'))) {
        return Err("a `\\u` escape has no `{`".to_owned());
    }
    let mut digits = String::new();
    for (_, character) in characters.by_ref() {
        if character == '}' {
            return u32::from_str_radix(&digits, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| format!("`\\u{{{digits}}}` is no character"));
        }
        digits.push(character);
    }
    Err("a `\\u` escape does not end".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From a mainnet node's log: one state transition.
    const MAINNET_ONE_TRANSITION: &str = r#""\"status\" \" \"Accepted\"\", \"app_hash\" \" \"26a5ed2dabc8046c800cec5a8f3b411ab7ba6c47775deaf7dd4d73901946bd6a\"\", \"tx_results\" \" [\"\\\"code\\\" \\\" 0\\\", \\\"data\\\" \\\" \\\"\\\"\\\", \\\"log\\\" \\\" \\\"\\\"\\\", \\\"info\\\" \\\" \\\"\\\"\\\", \\\"gas_used\\\"\\n\\\" 117438280\\\", \\\"events\\\" \\\" \\\"[]\\\"\\\", \\\"codespace\\\" \\\" \\\"\\\"\\\",\"]\",\n\"consensus_param_updates\" \" \"None\"\", \"validator_set_update\" \" \"None\"\",""#;

    /// From a mainnet node's log: no state transition.
    const MAINNET_NO_TRANSITION: &str = r#""\"status\" \" \"Accepted\"\", \"app_hash\" \" \"50c88b58a1eb1377cbcc22166c72479c05e73f34aaf942d06736cf4d11b0df0b\"\", \"tx_results\" \" []\",\n\"consensus_param_updates\" \" \"None\"\", \"validator_set_update\" \" \"None\"\",""#;

    /// From a mainnet node's log: a validator set update, cut after its first validator.
    const MAINNET_VALIDATOR_SET_UPDATE: &str = r#""\"status\" \" \"Accepted\"\", \"app_hash\" \" \"dc70395acb1efcf1c4ad9f458989aa9d3d5d656025f9e627780471ca7cc12cbb\"\", \"tx_results\" \" []\",\n\"consensus_param_updates\" \" \"None\"\", \"validator_set_update\" \" \"\\\"validator_updates\\\" \\\" [\\\"\\\\\\\"pub_key\\\\\\\" \\\\\\\" None\\\\\\\", \\\\\\\"power\\\\\\\" \\\\\\\" 100\\\\\\\", \\\\\\\"pro_tx_hash\\\\\\\" \\\\\\\" \\\\\\\"007a7d6e5fc0adca9dab24584a03dabfcd4d907d811e4b69aa3bf2f9cdd80232\\\\\\\"\\\\\\\", \\\\\\\"node_address\\\\\\\"\\\\n\\\\\\\" \\\\\\\"tcp://849d52a31a19dcdfda3a18ffbd70ce341c625e06@149.28.201.164:26656\\\\\\\"\\\\\\\",\\\"]\\\",\"\",""#;

    /// A transaction result as a node built without JSON logging writes it.
    fn debug_tx_result(code: u32, info: &str, gas_used: i64) -> String {
        format!(
            "\"code\" \" {:?}\", \"data\" \" {:?}\", \"log\" \" {:?}\", \"info\" \" {:?}\", \"gas_used\"\n\" {:?}\", \"events\" \" {:?}\", \"codespace\" \" {:?}\",",
            code, "", "", info, gas_used, "[]", ""
        )
    }

    /// The answer as such a node logs it: the keys and values, then `Debug` of the whole.
    fn debug_answer(status: &str, app_hash: &str, tx_results: &[String]) -> String {
        let text = format!(
            "\"status\" \" {:?}\", \"app_hash\" \" {:?}\", \"tx_results\" \" {:?}\",\n\"consensus_param_updates\" \" {:?}\", \"validator_set_update\" \" {:?}\",",
            status, app_hash, tx_results, "None", "None"
        );
        format!("{text:?}")
    }

    fn proposal(response_log: &str) -> ChainProposalAnswer {
        parse_response_log(response_log)
            .expect("the answer is read")
            .expect("it answers a proposal")
    }

    #[test]
    fn should_read_the_answers_of_a_mainnet_node_log() {
        assert_eq!(
            proposal(MAINNET_ONE_TRANSITION),
            ChainProposalAnswer {
                status: "Accepted".to_owned(),
                app_hash: "26a5ed2dabc8046c800cec5a8f3b411ab7ba6c47775deaf7dd4d73901946bd6a"
                    .to_owned(),
                tx: vec![TxOutcome(0, 117438280)],
            }
        );
        assert_eq!(
            proposal(MAINNET_NO_TRANSITION),
            ChainProposalAnswer {
                status: "Accepted".to_owned(),
                app_hash: "50c88b58a1eb1377cbcc22166c72479c05e73f34aaf942d06736cf4d11b0df0b"
                    .to_owned(),
                tx: vec![],
            }
        );
        assert_eq!(
            proposal(MAINNET_VALIDATOR_SET_UPDATE),
            ChainProposalAnswer {
                status: "Accepted".to_owned(),
                app_hash: "dc70395acb1efcf1c4ad9f458989aa9d3d5d656025f9e627780471ca7cc12cbb"
                    .to_owned(),
                tx: vec![],
            }
        );
    }

    #[test]
    fn should_read_what_the_node_writes_for_such_an_answer() {
        assert_eq!(
            debug_answer(
                "Accepted",
                "26a5ed2dabc8046c800cec5a8f3b411ab7ba6c47775deaf7dd4d73901946bd6a",
                &[debug_tx_result(0, "", 117438280)]
            ),
            MAINNET_ONE_TRANSITION
        );
    }

    #[test]
    fn should_read_every_transition_with_its_code_and_fee() {
        let response_log = debug_answer(
            "Rejected",
            "00ff",
            &[
                debug_tx_result(0, "", 117438280),
                // An error's text in `info`, with what a `Debug` string escapes
                debug_tx_result(
                    40212,
                    "a \"quoted\" \\ text,\n\"gas_used\" \" 7\", é \u{7f}",
                    0,
                ),
                debug_tx_result(0, "", 9_223_372_036_854_775_807),
            ],
        );
        assert_eq!(
            proposal(&response_log),
            ChainProposalAnswer {
                status: "Rejected".to_owned(),
                app_hash: "00ff".to_owned(),
                tx: vec![
                    TxOutcome(0, 117438280),
                    TxOutcome(40212, 0),
                    TxOutcome(0, 9_223_372_036_854_775_807),
                ],
            }
        );
    }

    #[test]
    fn should_read_an_answer_logged_as_json() {
        let text = serde_json::json!({
            "status": "Accepted",
            "app_hash": "26a5",
            "tx_results": [
                serde_json::json!({"code": 0, "data": "", "log": "", "info": "", "gas_used": 117438280, "events": "[]", "codespace": ""}).to_string(),
                serde_json::json!({"code": 10212, "data": "", "log": "", "info": "oA==", "gas_used": 0, "events": "[]", "codespace": ""}).to_string(),
            ],
            "consensus_param_updates": "None",
            "validator_set_update": "None",
        })
        .to_string();
        assert_eq!(
            proposal(&format!("{text:?}")),
            ChainProposalAnswer {
                status: "Accepted".to_owned(),
                app_hash: "26a5".to_owned(),
                tx: vec![TxOutcome(0, 117438280), TxOutcome(10212, 0)],
            }
        );
    }

    #[test]
    fn should_pass_over_the_answers_to_other_requests() {
        // `Info`, `InitChain`, `FinalizeBlock` and `Flush` are logged as their `Debug` form
        let info = format!(
            "{:?}",
            "Info(ResponseInfo { data: \"\", version: \"5.0.3\", app_version: 14, last_block_height: 0, last_block_app_hash: [0, 0] })"
        );
        assert_eq!(parse_response_log(&info), Ok(None));
        assert_eq!(parse_response_log("Flush(ResponseFlush)"), Ok(None));

        // `PrepareProposal` is logged as keys and values too, and has no status
        let prepare = format!(
            "{:?}",
            "\"tx_records\" \" []\", \"app_hash\" \" \"26a5\"\", \"tx_results\" \" []\","
        );
        assert_eq!(parse_response_log(&prepare), Ok(None));
        let prepare = format!(
            "{:?}",
            serde_json::json!({"tx_records": [], "app_hash": "26a5", "tx_results": []}).to_string()
        );
        assert_eq!(parse_response_log(&prepare), Ok(None));
    }

    #[test]
    fn should_say_why_an_answer_cannot_be_read() {
        let unreadable = |text: &str| parse_response_log(&format!("{text:?}")).unwrap_err();

        assert_eq!(
            unreadable("\"status\" \" \"Accepted\"\", \"tx_results\" \" []\","),
            "expected `app_hash` before `tx_results\" \" []\",`"
        );
        assert_eq!(
            unreadable("\"status\" \" \"Accepted\"\", \"app_hash\" \" \"26a5\"\", \"tx_results\" \" [\"\\\"code\\\" \\\" x\\\",\"]\","),
            "expected a number before `x\",`"
        );
        assert_eq!(
            unreadable("\"status\" \" \"Accepted\"\", \"app_hash\" \" \"26a5\"\", \"tx_results\" \" [\"\\\"code\\\" \\\" -1\\\",\"]\","),
            "result code -1 is out of range"
        );
        // A log line cut short
        assert_eq!(
            unreadable("\"status\" \" \"Accepted\"\", \"app_hash\" \" \"26a5"),
            "a string does not end"
        );
        assert_eq!(
            unreadable("\"status\" \" \"Accepted\"\", \"app_hash\" \" \"26a5\"\", \"tx_results\" \" [\"\\q\"]\","),
            "unknown escape `\\q`"
        );
        assert_eq!(
            unreadable("{\"status\":\"Accepted\",\"tx_results\":[]}"),
            "answer has no app_hash"
        );
        assert_eq!(
            unreadable("{\"status\":\"Accepted\",\"app_hash\":\"26a5\"}"),
            "answer has no tx_results"
        );
        assert!(unreadable(
            "{\"status\":\"Accepted\",\"app_hash\":\"26a5\",\"tx_results\":[\"{}\"]}"
        )
        .starts_with("transaction result is not JSON: "),);
        assert!(unreadable("{\"status\":").starts_with("answer is not JSON: "));

        assert_eq!(
            parse_response_log("\"Flush\" and more").unwrap_err(),
            "text follows the logged answer"
        );
        assert_eq!(
            parse_response_log("\"a\\").unwrap_err(),
            "a string does not end"
        );
    }

    #[test]
    fn should_read_the_escapes_of_a_debug_string() {
        let read = |text: &str| {
            let mut scanner = Scanner { rest: text };
            scanner.string()
        };
        let original = "tab\t cr\r nl\n nul\0 quote\" apostrophe' backslash\\ del\u{7f} é";
        assert_eq!(read(&format!("{original:?}")).unwrap(), original);
        assert_eq!(read("\"\\'\"").unwrap(), "'");
        assert_eq!(read("\"\\u{e9}\"").unwrap(), "é");
        assert_eq!(
            read("\"\\u0041\"").unwrap_err(),
            "a `\\u` escape has no `{`"
        );
        assert_eq!(
            read("\"\\u{zz}\"").unwrap_err(),
            "`\\u{zz}` is no character"
        );
        assert_eq!(read("\"\\u{41").unwrap_err(), "a `\\u` escape does not end");
        assert_eq!(
            read("\"\\u{d800}\"").unwrap_err(),
            "`\\u{d800}` is no character"
        );
        assert_eq!(
            read("no quote").unwrap_err(),
            "expected `\"` before `no quote`"
        );
    }

    #[test]
    fn should_write_a_transaction_outcome_as_its_code_and_fee() {
        assert_eq!(
            serde_json::to_string(&vec![TxOutcome(0, 117438280), TxOutcome(40212, 0)]).unwrap(),
            "[[0,117438280],[40212,0]]"
        );
        assert_eq!(LoggedAnswer::default(), LoggedAnswer::Absent);
    }
}
