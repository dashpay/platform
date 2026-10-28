//! Every request the Platform API accepts, against what nodes of an older release understand.
//!
//! A node of an older release drops a request field it does not know and answers a
//! different question, and refuses a method, oneof arm or enum value it does not know with
//! UNIMPLEMENTED or UNKNOWN. Either answer gets an honest node banned, so a request using
//! such a feature must declare the protocol version from which every node serves it
//! (`TransportRequest::needs_protocol_version`).
//!
//! `request_inventory/v4.1.1.txt` lists what 4.1 nodes understand: every method and every
//! field, oneof arm and enum value reachable from a request in v4.1.1's `platform.proto`.
//! `request_inventory/v5.0.txt` lists the same for 5.0 nodes, whose first protocol version
//! is 14. This test fails on anything reachable from a request today that is in neither
//! list nor in [`ADDITIONS`], and on an addition 5.0 nodes do not know that declares 14 or
//! less. It checks each addition's verdict against `needs_protocol_version` on real wire
//! bytes, and that requests made only of what 4.1 nodes know need no protocol version.

use dapi_grpc::platform::v0 as proto;
use dapi_grpc::Message;
use rs_dapi_client::transport::TransportRequest;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// What a node that predates a request feature does with it.
#[derive(Debug, Clone, Copy)]
enum Verdict {
    /// Every node serves requests using the feature from this protocol version.
    Gate(u32),
    /// Nodes of every version serve or refuse it alike, without a ban; the reason says how.
    NoGate(&'static str),
}

/// Request features added since 4.1.1. The verdict of a feature 5.0 added covers everything
/// reachable only through it; anything a later release adds, inside it or not, is declared
/// on its own, with that release's first protocol version, not 14.
const ADDITIONS: &[(&str, Verdict)] = &[
    ("rpc getIdentityKeysRemainingBudgets GetIdentityKeysRemainingBudgetsRequest", Verdict::Gate(14)),
    ("rpc getDataContractsLatestVersions GetDataContractsLatestVersionsRequest", Verdict::Gate(14)),
    ("rpc getDataContractsByRange GetDataContractsByRangeRequest", Verdict::Gate(14)),
    ("rpc getContractGroupInfo GetContractGroupInfoRequest", Verdict::Gate(14)),
    ("rpc getContractGroupMembers GetContractGroupMembersRequest", Verdict::Gate(14)),
    ("rpc getContractGroupsForContract GetContractGroupsForContractRequest", Verdict::Gate(14)),
    ("rpc getContractModerationStatus GetContractModerationStatusRequest", Verdict::Gate(14)),
    ("rpc getContractModerationEntries GetContractModerationEntriesRequest", Verdict::Gate(14)),
    ("rpc getContractDocumentRemovals GetContractDocumentRemovalsRequest", Verdict::Gate(14)),
    ("rpc getContractTeamActions GetContractTeamActionsRequest", Verdict::Gate(14)),
    ("rpc getContractTeamActionSigners GetContractTeamActionSignersRequest", Verdict::Gate(14)),
    ("rpc getContractModerationActionCounts GetContractModerationActionCountsRequest", Verdict::Gate(14)),
    ("rpc getContractFeePots GetContractFeePotsRequest", Verdict::Gate(14)),
    // A 4.1 node drops both and proves the plain page or the inner query instead.
    (
        "field GetDocumentsRequest.GetDocumentsRequestV1.chained = 13 GetDocumentsRequest.GetDocumentsRequestV1.ChainedJoin",
        Verdict::Gate(14),
    ),
    (
        "field GetDocumentsRequest.GetDocumentsRequestV1.sub_queries = 14 repeated GetDocumentsRequest.GetDocumentsRequestV1.SubQuery",
        Verdict::Gate(14),
    ),
    // A 4.1 node drops `token_id` and proves the credit pool instead.
    ("field GetShieldedEncryptedNotesRequest.GetShieldedEncryptedNotesRequestV0.token_id = 4 bytes", Verdict::Gate(14)),
    ("field GetShieldedAnchorsRequest.GetShieldedAnchorsRequestV0.token_id = 2 bytes", Verdict::Gate(14)),
    ("field GetMostRecentShieldedAnchorRequest.GetMostRecentShieldedAnchorRequestV0.token_id = 2 bytes", Verdict::Gate(14)),
    ("field GetShieldedPoolStateRequest.GetShieldedPoolStateRequestV0.token_id = 2 bytes", Verdict::Gate(14)),
    ("field GetShieldedNotesCountRequest.GetShieldedNotesCountRequestV0.token_id = 2 bytes", Verdict::Gate(14)),
    ("field GetShieldedNullifiersRequest.GetShieldedNullifiersRequestV0.token_id = 3 bytes", Verdict::Gate(14)),
    (
        "field GetDocumentsRequest.WhereClause.time_range = 4 GetDocumentsRequest.TimeRangeSelection",
        Verdict::NoGate("sent only with the IN_TIME_RANGE operator, which every node before protocol 14 refuses with INVALID_ARGUMENT"),
    ),
    (
        "field GetDocumentsRequest.WhereClause.integer_range = 5 GetDocumentsRequest.IntegerRangeSelection",
        Verdict::NoGate("sent only with the IN_INTEGER_RANGE operator, which every node before protocol 14 refuses with INVALID_ARGUMENT"),
    ),
    (
        "enum GetDocumentsRequest.WhereOperator.IN_TIME_RANGE = 11",
        Verdict::NoGate("every node before protocol 14 refuses it with INVALID_ARGUMENT"),
    ),
    (
        "enum GetDocumentsRequest.WhereOperator.IN_INTEGER_RANGE = 12",
        Verdict::NoGate("every node before protocol 14 refuses it with INVALID_ARGUMENT"),
    ),
    (
        "enum GetDocumentsRequest.WhereOperator.CONTAINS = 13",
        Verdict::NoGate("every node before protocol 14 refuses it with INVALID_ARGUMENT, and the SDK checks it against its own version"),
    ),
    (
        "enum GetDocumentsRequest.WhereOperator.CONTAINS_ANY = 14",
        Verdict::NoGate("every node before protocol 14 refuses it with INVALID_ARGUMENT, and the SDK checks it against its own version"),
    ),
    (
        "field GetIdentityByNonUniquePublicKeyHashRequest.v1 = 2 GetIdentityByNonUniquePublicKeyHashRequest.GetIdentityByNonUniquePublicKeyHashRequestV1",
        Verdict::NoGate("the SDK selects it only once a verified answer shows protocol 14"),
    ),
    (
        "field WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.request_user_balance = 3 bool",
        Verdict::NoGate("a node that drops it answers without the balance, which the result allows"),
    ),
];

/// The Rust request type of each method, as `TransportRequest::method_name` names it.
macro_rules! requests {
    ($($request:ident),+ $(,)?) => {
        /// `needs_protocol_version` of the request of `method` decoded from `bytes`; `None`
        /// when no request type is listed for the method.
        fn needs_protocol_version(method: &str, bytes: &[u8]) -> Option<Option<u32>> {
            $(
                if proto::$request::default().method_name() == method {
                    let request = proto::$request::decode(bytes)
                        .unwrap_or_else(|error| panic!("{method}: {error}"));
                    return Some(request.needs_protocol_version());
                }
            )+
            None
        }
    };
}

requests!(
    BroadcastStateTransitionRequest,
    GetIdentityRequest,
    GetIdentityKeysRequest,
    GetIdentitiesContractKeysRequest,
    GetIdentityNonceRequest,
    GetIdentityContractNonceRequest,
    GetIdentityKeysRemainingBudgetsRequest,
    GetIdentityBalanceRequest,
    GetIdentitiesBalancesRequest,
    GetIdentityBalanceAndRevisionRequest,
    GetEvonodesProposedEpochBlocksByIdsRequest,
    GetEvonodesProposedEpochBlocksByRangeRequest,
    GetDataContractRequest,
    GetDataContractHistoryRequest,
    GetDataContractsLatestVersionsRequest,
    GetDataContractsRequest,
    GetDataContractsByRangeRequest,
    GetContractGroupInfoRequest,
    GetContractGroupMembersRequest,
    GetContractGroupsForContractRequest,
    GetContractModerationStatusRequest,
    GetContractModerationEntriesRequest,
    GetContractDocumentRemovalsRequest,
    GetContractTeamActionsRequest,
    GetContractTeamActionSignersRequest,
    GetContractModerationActionCountsRequest,
    GetContractFeePotsRequest,
    GetDocumentHistoryRequest,
    GetDocumentsRequest,
    GetIdentityByPublicKeyHashRequest,
    GetIdentityByNonUniquePublicKeyHashRequest,
    WaitForStateTransitionResultRequest,
    GetConsensusParamsRequest,
    GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeVoteStatusRequest,
    GetEpochsInfoRequest,
    GetFinalizedEpochInfosRequest,
    GetContestedResourcesRequest,
    GetContestedResourceVoteStateRequest,
    GetContestedResourceVotersForIdentityRequest,
    GetContestedResourceIdentityVotesRequest,
    GetVotePollsByEndDateRequest,
    GetPrefundedSpecializedBalanceRequest,
    GetTotalCreditsInPlatformRequest,
    GetPathElementsRequest,
    GetStatusRequest,
    GetCurrentQuorumsInfoRequest,
    GetIdentityTokenBalancesRequest,
    GetIdentitiesTokenBalancesRequest,
    GetIdentityTokenInfosRequest,
    GetIdentitiesTokenInfosRequest,
    GetTokenStatusesRequest,
    GetTokenDirectPurchasePricesRequest,
    GetTokenContractInfoRequest,
    GetTokenPreProgrammedDistributionsRequest,
    GetTokenPerpetualDistributionLastClaimRequest,
    GetTokenTotalSupplyRequest,
    GetGroupInfoRequest,
    GetGroupInfosRequest,
    GetGroupActionsRequest,
    GetGroupActionSignersRequest,
    GetAddressInfoRequest,
    GetAddressesInfosRequest,
    GetAddressesTrunkStateRequest,
    GetAddressesBranchStateRequest,
    GetRecentAddressBalanceChangesRequest,
    GetRecentCompactedAddressBalanceChangesRequest,
    GetShieldedEncryptedNotesRequest,
    GetShieldedAnchorsRequest,
    GetMostRecentShieldedAnchorRequest,
    GetShieldedPoolStateRequest,
    GetShieldedNotesCountRequest,
    GetShieldedNullifiersRequest,
);

// ---------------------------------------------------------------------------------------
// A `.proto` reader, enough for `platform.proto`.

const SCALARS: [&str; 15] = [
    "double", "float", "int32", "int64", "uint32", "uint64", "sint32", "sint64", "fixed32",
    "fixed64", "sfixed32", "sfixed64", "bool", "string", "bytes",
];

#[derive(Debug, Clone)]
struct Field {
    name: String,
    number: u32,
    /// The type as written.
    type_name: String,
    repeated: bool,
    /// Key and value types of a map field.
    map: Option<(String, String)>,
    oneof: Option<String>,
}

#[derive(Debug, Default)]
struct Schema {
    messages: BTreeMap<String, Vec<Field>>,
    enums: BTreeMap<String, Vec<(String, i64)>>,
    /// Method name and request type, in declaration order.
    rpcs: Vec<(String, String)>,
}

/// A resolved field or map value type.
#[derive(Debug, Clone, PartialEq)]
enum Type {
    Scalar(String),
    Message(String),
    Enum(String),
    /// A well-known type from an import: written opaquely.
    External(String),
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '"' {
            let start = i;
            i += 1;
            while chars[i] != '"' {
                i += 1;
            }
            i += 1;
            tokens.push(chars[start..i].iter().collect());
        } else if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '.'
                    || chars[i] == '-')
            {
                i += 1;
            }
            tokens.push(chars[start..i].iter().collect());
        } else if c.is_whitespace() {
            i += 1;
        } else {
            tokens.push(c.to_string());
            i += 1;
        }
    }
    tokens
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn next(&mut self) -> String {
        let token = self.tokens[self.at].clone();
        self.at += 1;
        token
    }
    fn peek(&self) -> &str {
        &self.tokens[self.at]
    }
    fn expect(&mut self, token: &str) {
        let next = self.next();
        assert_eq!(next, token, "at token {}", self.at);
    }
    fn skip_statement(&mut self) {
        while self.next() != ";" {}
    }
    /// Skips `[ ... ]` field options, if any.
    fn skip_options(&mut self) {
        if self.peek() == "[" {
            while self.next() != "]" {}
        }
    }
    fn skip_block(&mut self) {
        let mut depth = 0;
        loop {
            match self.next().as_str() {
                "{" => depth += 1,
                "}" => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    fn file(mut self) -> Schema {
        let mut schema = Schema::default();
        while self.at < self.tokens.len() {
            match self.peek() {
                "message" => self.message("", &mut schema),
                "enum" => self.enumeration("", &mut schema),
                "service" => self.service(&mut schema),
                _ => self.skip_statement(),
            }
        }
        schema
    }

    fn message(&mut self, scope: &str, schema: &mut Schema) {
        self.expect("message");
        let name = qualify(scope, &self.next());
        self.expect("{");
        let mut fields = Vec::new();
        self.body(&name, None, &mut fields, schema);
        schema.messages.insert(name, fields);
    }

    fn body(
        &mut self,
        scope: &str,
        oneof: Option<&str>,
        fields: &mut Vec<Field>,
        schema: &mut Schema,
    ) {
        loop {
            match self.peek() {
                "}" => {
                    self.next();
                    return;
                }
                // An empty statement.
                ";" => {
                    self.next();
                }
                "message" => self.message(scope, schema),
                "enum" => self.enumeration(scope, schema),
                "oneof" => {
                    self.next();
                    let name = self.next();
                    self.expect("{");
                    self.body(scope, Some(&name), fields, schema);
                }
                "option" | "reserved" | "extensions" => self.skip_statement(),
                "map" => {
                    self.next();
                    self.expect("<");
                    let key = self.next();
                    self.expect(",");
                    let value = self.next();
                    self.expect(">");
                    let name = self.next();
                    self.expect("=");
                    let number = self.next().parse().unwrap();
                    self.skip_options();
                    self.expect(";");
                    fields.push(Field {
                        name,
                        number,
                        type_name: value.clone(),
                        repeated: true,
                        map: Some((key, value)),
                        oneof: None,
                    });
                }
                _ => {
                    let mut repeated = false;
                    if matches!(self.peek(), "repeated" | "optional") {
                        repeated = self.next() == "repeated";
                    }
                    let type_name = self.next();
                    let name = self.next();
                    self.expect("=");
                    let number = self.next().parse().unwrap();
                    self.skip_options();
                    self.expect(";");
                    fields.push(Field {
                        name,
                        number,
                        type_name,
                        repeated,
                        map: None,
                        oneof: oneof.map(str::to_string),
                    });
                }
            }
        }
    }

    fn enumeration(&mut self, scope: &str, schema: &mut Schema) {
        self.expect("enum");
        let name = qualify(scope, &self.next());
        self.expect("{");
        let mut values = Vec::new();
        loop {
            match self.peek() {
                "}" => {
                    self.next();
                    break;
                }
                "option" | "reserved" => self.skip_statement(),
                _ => {
                    let value = self.next();
                    self.expect("=");
                    let number = self.next().parse().unwrap();
                    self.skip_options();
                    self.expect(";");
                    values.push((value, number));
                }
            }
        }
        schema.enums.insert(name, values);
    }

    fn service(&mut self, schema: &mut Schema) {
        self.expect("service");
        self.next();
        self.expect("{");
        loop {
            match self.peek() {
                "}" => {
                    self.next();
                    return;
                }
                "rpc" => {
                    self.next();
                    let name = self.next();
                    self.expect("(");
                    if self.peek() == "stream" {
                        self.next();
                    }
                    let request = self.next();
                    self.expect(")");
                    while !matches!(self.peek(), ";" | "{") {
                        self.next();
                    }
                    if self.peek() == "{" {
                        self.skip_block();
                    } else {
                        self.next();
                    }
                    schema.rpcs.push((name, request));
                }
                _ => self.skip_statement(),
            }
        }
    }
}

fn qualify(scope: &str, name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else {
        format!("{scope}.{name}")
    }
}

impl Schema {
    fn parse(text: &str) -> Self {
        Parser {
            tokens: tokenize(text),
            at: 0,
        }
        .file()
    }

    /// Resolves `name` as written in message `scope`, innermost scope first.
    fn resolve(&self, scope: &str, name: &str) -> Type {
        if SCALARS.contains(&name) {
            return Type::Scalar(name.to_string());
        }
        if name.starts_with("google.protobuf.") {
            return Type::External(name.to_string());
        }
        let mut scope = scope.to_string();
        loop {
            let candidate = qualify(&scope, name);
            if self.messages.contains_key(&candidate) {
                return Type::Message(candidate);
            }
            if self.enums.contains_key(&candidate) {
                return Type::Enum(candidate);
            }
            assert!(!scope.is_empty(), "unresolved type {name}");
            scope = scope
                .rsplit_once('.')
                .map(|(outer, _)| outer.to_string())
                .unwrap_or_default();
        }
    }

    fn type_text(&self, scope: &str, field: &Field) -> String {
        let resolved = |name: &str| match self.resolve(scope, name) {
            Type::Scalar(name) | Type::Message(name) | Type::Enum(name) | Type::External(name) => {
                name
            }
        };
        match &field.map {
            Some((key, value)) => format!("map<{}, {}>", resolved(key), resolved(value)),
            None if field.repeated => format!("repeated {}", resolved(&field.type_name)),
            None => resolved(&field.type_name),
        }
    }

    /// The inventory line of a field of `message`.
    fn field_item(&self, message: &str, field: &Field) -> String {
        format!(
            "field {message}.{} = {} {}",
            field.name,
            field.number,
            self.type_text(message, field)
        )
    }

    /// Every method and every field, oneof arm and enum value reachable from a request.
    /// The walk does not enter an item `covered` returns true for: its verdict covers what
    /// lies behind it. Returns the items in walk order with the methods they belong to.
    fn reachable(&self, covered: &dyn Fn(&str) -> bool) -> Vec<String> {
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        for (method, request) in &self.rpcs {
            let item = format!("rpc {method} {request}");
            let enter = !covered(&item);
            items.push(item);
            if enter {
                self.walk_message(request, covered, &mut seen, &mut items);
            }
        }
        items
    }

    fn walk_message(
        &self,
        message: &str,
        covered: &dyn Fn(&str) -> bool,
        seen: &mut BTreeSet<String>,
        items: &mut Vec<String>,
    ) {
        if !seen.insert(message.to_string()) {
            return;
        }
        for field in &self.messages[message] {
            let item = self.field_item(message, field);
            let enter = !covered(&item);
            items.push(item);
            if !enter {
                continue;
            }
            let value = field
                .map
                .as_ref()
                .map(|(_, value)| value)
                .unwrap_or(&field.type_name);
            match self.resolve(message, value) {
                Type::Message(name) => self.walk_message(&name, covered, seen, items),
                Type::Enum(name) => {
                    if seen.insert(name.clone()) {
                        for (value, number) in &self.enums[&name] {
                            items.push(format!("enum {name}.{value} = {number}"));
                        }
                    }
                }
                Type::Scalar(_) | Type::External(_) => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Wire bytes of a request, built from the schema.

/// Which items a synthesized request sets.
struct Shape<'a> {
    schema: &'a Schema,
    /// Fields and arms to set; the others stay unset.
    sets: &'a dyn Fn(&str) -> bool,
    /// Which arm of each oneof to take: the `pick`-th settable one, modulo their count.
    pick: usize,
    /// An enum value to use instead of the first settable one.
    enum_value: Option<(&'a str, i64)>,
    /// A field to report writing.
    watched: Option<&'a str>,
    /// Whether the watched field or the overriding enum value was written.
    wrote_watched: std::cell::Cell<bool>,
}

fn varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn key(number: u32, wire_type: u8, out: &mut Vec<u8>) {
    varint(((number as u64) << 3) | wire_type as u64, out);
}

fn length_delimited(number: u32, bytes: &[u8], out: &mut Vec<u8>) {
    key(number, 2, out);
    varint(bytes.len() as u64, out);
    out.extend_from_slice(bytes);
}

impl Shape<'_> {
    fn message(&self, name: &str, stack: &mut Vec<String>) -> Vec<u8> {
        let mut out = Vec::new();
        if stack.iter().any(|entered| entered == name) || stack.len() > 12 {
            return out;
        }
        stack.push(name.to_string());
        let fields = &self.schema.messages[name];
        let mut oneofs: BTreeMap<&str, Vec<&Field>> = BTreeMap::new();
        for field in fields {
            if (self.sets)(&self.schema.field_item(name, field)) {
                match &field.oneof {
                    Some(oneof) => oneofs.entry(oneof).or_default().push(field),
                    None => self.field(name, field, stack, &mut out),
                }
            }
        }
        for arms in oneofs.values() {
            self.field(name, arms[self.pick % arms.len()], stack, &mut out);
        }
        stack.pop();
        out
    }

    fn field(&self, message: &str, field: &Field, stack: &mut Vec<String>, out: &mut Vec<u8>) {
        if self.watched == Some(self.schema.field_item(message, field).as_str()) {
            self.wrote_watched.set(true);
        }
        match &field.map {
            Some((key_type, value_type)) => {
                let mut entry = Vec::new();
                self.value(message, key_type, 1, stack, &mut entry);
                self.value(message, value_type, 2, stack, &mut entry);
                length_delimited(field.number, &entry, out);
            }
            None => self.value(message, &field.type_name, field.number, stack, out),
        }
    }

    fn value(
        &self,
        scope: &str,
        type_name: &str,
        number: u32,
        stack: &mut Vec<String>,
        out: &mut Vec<u8>,
    ) {
        match self.schema.resolve(scope, type_name) {
            Type::Message(name) => {
                let bytes = self.message(&name, stack);
                length_delimited(number, &bytes, out);
            }
            Type::Enum(name) => {
                let value = match self.enum_value {
                    Some((overridden, value)) if overridden == name => {
                        self.wrote_watched.set(true);
                        value
                    }
                    _ => self.schema.enums[&name]
                        .iter()
                        .find(|(value, number)| {
                            *number != 0 && (self.sets)(&format!("enum {name}.{value} = {number}"))
                        })
                        .map(|(_, number)| *number)
                        .unwrap_or(0),
                };
                key(number, 0, out);
                varint(value as u64, out);
            }
            Type::External(_) => length_delimited(number, &[], out),
            Type::Scalar(scalar) => match scalar.as_str() {
                "string" => length_delimited(number, b"a", out),
                "bytes" => length_delimited(number, &[1; 32], out),
                "double" | "fixed64" | "sfixed64" => {
                    key(number, 1, out);
                    out.extend_from_slice(&1u64.to_le_bytes());
                }
                "float" | "fixed32" | "sfixed32" => {
                    key(number, 5, out);
                    out.extend_from_slice(&1u32.to_le_bytes());
                }
                // Zigzag encoding of 1.
                "sint32" | "sint64" => {
                    key(number, 0, out);
                    varint(2, out);
                }
                _ => {
                    key(number, 0, out);
                    varint(1, out);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------------------

fn packages() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn head() -> Schema {
    Schema::parse(
        &std::fs::read_to_string(packages().join("dapi-grpc/protos/platform/v0/platform.proto"))
            .unwrap(),
    )
}

fn baseline() -> BTreeSet<String> {
    inventory("v4.1.1")
}

/// What nodes of `release` understand, as `should_export_an_inventory` wrote it.
fn inventory(release: &str) -> BTreeSet<String> {
    std::fs::read_to_string(packages().join(format!(
        "rs-dapi-client/tests/request_inventory/{release}.txt"
    )))
    .unwrap()
    .lines()
    .filter(|line| !line.is_empty() && !line.starts_with('#'))
    .map(str::to_string)
    .collect()
}

fn additions() -> BTreeMap<&'static str, Verdict> {
    let additions: BTreeMap<_, _> = ADDITIONS.iter().copied().collect();
    assert_eq!(
        additions.len(),
        ADDITIONS.len(),
        "an addition is listed twice"
    );
    additions
}

/// snake_case of a camelCase method name, as `TransportRequest::method_name` spells it.
fn snake(method: &str) -> String {
    let mut out = String::new();
    for c in method.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// What `request` (an inventory `rpc` line's request type) needs, decoded from `bytes`.
fn needs(schema: &Schema, request: &str, bytes: &[u8]) -> Option<u32> {
    let (method, _) = schema
        .rpcs
        .iter()
        .find(|(_, r)| r == request)
        .expect("a method takes the request");
    needs_protocol_version(&snake(method), bytes)
        .unwrap_or_else(|| panic!("no Rust request type listed for {method}"))
}

/// Requests made of anything reachable from them today, of only what 4.1 nodes know, or
/// of that plus one addition, in every choice of oneof arms.
fn shapes<'a>(
    schema: &'a Schema,
    request: &str,
    sets: &'a dyn Fn(&str) -> bool,
    enum_value: Option<(&'a str, i64)>,
    watched: Option<&'a str>,
) -> Vec<(Vec<u8>, bool)> {
    let arms = schema
        .messages
        .values()
        .flat_map(|fields| {
            let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
            for field in fields {
                if let Some(oneof) = &field.oneof {
                    *counts.entry(oneof).or_default() += 1;
                }
            }
            counts.into_values()
        })
        .max()
        .unwrap_or(1);
    (0..arms)
        .map(|pick| {
            let shape = Shape {
                schema,
                sets,
                pick,
                enum_value,
                watched,
                wrote_watched: std::cell::Cell::new(false),
            };
            let bytes = shape.message(request, &mut Vec::new());
            (bytes, shape.wrote_watched.get())
        })
        .collect()
}

/// Every method, field, oneof arm and enum value reachable from a request is either known
/// to 4.1 nodes or declared with a verdict, and every declared addition still exists.
#[test]
fn should_declare_every_request_feature_older_nodes_do_not_know() {
    let schema = head();
    let baseline = baseline();
    let additions = additions();
    let reachable = schema.reachable(&|item| additions.contains_key(item));
    let five = inventory("v5.0");
    // Everything 5.0 nodes do not know, also inside what 5.0 added, is declared on its own.
    let later: Vec<_> = schema
        .reachable(&|item| additions.contains_key(item) && !five.contains(item))
        .into_iter()
        .filter(|item| !five.contains(item) && !additions.contains_key(item.as_str()))
        .collect();
    assert!(
        later.is_empty(),
        "request features 5.0 nodes do not know, without a verdict in ADDITIONS:\n{}",
        later
            .iter()
            .map(|item| format!("  {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    for (item, verdict) in &additions {
        if let (false, Verdict::Gate(version)) = (five.contains(*item), verdict) {
            assert!(
                *version > 14,
                "{item}: 5.0 nodes do not know it, so it needs a version after 14, the first \
                 5.0 nodes sign"
            );
        }
    }
    let undeclared: Vec<_> = reachable
        .iter()
        .filter(|item| !baseline.contains(*item) && !additions.contains_key(item.as_str()))
        .collect();
    assert!(
        undeclared.is_empty(),
        "request features 4.1 nodes do not know, without a verdict in ADDITIONS (declare \
         the protocol version every node serves them from, see \
         TransportRequest::needs_protocol_version):\n{}",
        undeclared
            .iter()
            .map(|item| format!("  {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let stale: Vec<_> = additions
        .keys()
        .filter(|item| !reachable.iter().any(|reached| reached == *item))
        .collect();
    assert!(stale.is_empty(), "additions no request reaches: {stale:?}");
}

/// Each method of the Platform service has a request type that declares what it needs.
#[test]
fn should_know_the_request_type_of_every_method() {
    for (method, request) in &head().rpcs {
        assert!(
            needs_protocol_version(&snake(method), &[]).is_some(),
            "{method}({request}): add its request type to `requests!`"
        );
    }
}

/// A request made only of what 4.1 nodes know is served by every node.
#[test]
fn should_need_no_protocol_version_for_requests_older_nodes_understand() {
    let schema = head();
    let baseline = baseline();
    let sets = |item: &str| baseline.contains(item);
    for (method, request) in &schema.rpcs {
        if !baseline.contains(&format!("rpc {method} {request}")) {
            continue;
        }
        for (pick, (bytes, _)) in shapes(&schema, request, &sets, None, None)
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                needs(&schema, request, &bytes),
                None,
                "{method}, oneof arms #{pick}: {}",
                hex(&bytes)
            );
        }
    }
}

/// Each addition needs exactly what its verdict says, alone on top of what 4.1 nodes know.
#[test]
fn should_need_the_declared_protocol_version_for_each_addition() {
    let schema = head();
    let baseline = baseline();
    for (addition, verdict) in additions() {
        let expected = match verdict {
            Verdict::Gate(version) => Some(version),
            Verdict::NoGate(reason) => {
                assert!(!reason.is_empty(), "{addition}: say why no node is banned");
                None
            }
        };
        let mut checked = 0;
        if let Some(request) = addition
            .strip_prefix("rpc ")
            .and_then(|rest| rest.split(' ').nth(1))
        {
            // Nothing of a new method is known to older nodes: any request needs it.
            let sets = |_: &str| true;
            for (bytes, _) in shapes(&schema, request, &sets, None, None) {
                assert_eq!(needs(&schema, request, &bytes), expected, "{addition}");
                checked += 1;
            }
            assert_eq!(
                needs(&schema, request, &[]),
                expected,
                "{addition}, empty request"
            );
        } else {
            let enum_value = addition.strip_prefix("enum ").map(|rest| {
                let (path, number) = rest.split_once(" = ").unwrap();
                let (name, _) = path.rsplit_once('.').unwrap();
                (name, number.parse::<i64>().unwrap())
            });
            // Requests that carry the addition on top of what 4.1 nodes know.
            let sets = |item: &str| baseline.contains(item) || item == addition;
            for (method, request) in &schema.rpcs {
                if !baseline.contains(&format!("rpc {method} {request}")) {
                    continue;
                }
                let watched = addition.starts_with("field ").then_some(addition);
                for (bytes, wrote) in shapes(&schema, request, &sets, enum_value, watched) {
                    if wrote {
                        assert_eq!(
                            needs(&schema, request, &bytes),
                            expected,
                            "{addition} in {method}: {}",
                            hex(&bytes)
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 0, "no request carries {addition}");
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The shapes a 4.1 node answers differently, spelled out.
#[test]
fn should_need_protocol_14_only_for_composite_chained_and_token_pool_requests() {
    use proto::get_documents_request::get_documents_request_v1::{ChainedJoin, SubQuery};
    use proto::get_documents_request::{GetDocumentsRequestV0, GetDocumentsRequestV1, Version};
    let documents = |v1: GetDocumentsRequestV1| proto::GetDocumentsRequest {
        version: Some(Version::V1(v1)),
    };
    let plain = GetDocumentsRequestV1 {
        document_type: "domain".to_string(),
        limit: Some(10),
        prove: true,
        ..Default::default()
    };
    assert_eq!(documents(plain.clone()).needs_protocol_version(), None);
    assert_eq!(
        proto::GetDocumentsRequest {
            version: Some(Version::V0(GetDocumentsRequestV0::default())),
        }
        .needs_protocol_version(),
        None
    );
    assert_eq!(
        documents(GetDocumentsRequestV1 {
            chained: Some(ChainedJoin::default()),
            ..plain.clone()
        })
        .needs_protocol_version(),
        Some(14)
    );
    assert_eq!(
        documents(GetDocumentsRequestV1 {
            sub_queries: vec![SubQuery::default()],
            ..plain
        })
        .needs_protocol_version(),
        Some(14)
    );

    macro_rules! token_pool {
        ($request:ident, $module:ident, $v0:ident) => {
            for (token_id, expected) in [(None, None), (Some(vec![7; 32]), Some(14))] {
                let request = proto::$request {
                    version: Some(proto::$module::Version::V0(proto::$module::$v0 {
                        token_id,
                        ..Default::default()
                    })),
                };
                assert_eq!(
                    request.needs_protocol_version(),
                    expected,
                    "{}",
                    request.method_name()
                );
            }
        };
    }
    token_pool!(
        GetShieldedEncryptedNotesRequest,
        get_shielded_encrypted_notes_request,
        GetShieldedEncryptedNotesRequestV0
    );
    token_pool!(
        GetShieldedAnchorsRequest,
        get_shielded_anchors_request,
        GetShieldedAnchorsRequestV0
    );
    token_pool!(
        GetMostRecentShieldedAnchorRequest,
        get_most_recent_shielded_anchor_request,
        GetMostRecentShieldedAnchorRequestV0
    );
    token_pool!(
        GetShieldedPoolStateRequest,
        get_shielded_pool_state_request,
        GetShieldedPoolStateRequestV0
    );
    token_pool!(
        GetShieldedNotesCountRequest,
        get_shielded_notes_count_request,
        GetShieldedNotesCountRequestV0
    );
    token_pool!(
        GetShieldedNullifiersRequest,
        get_shielded_nullifiers_request,
        GetShieldedNullifiersRequestV0
    );
}

/// Writes what nodes of a release understand from its `platform.proto`: run with
/// `REQUEST_INVENTORY_PROTO=<path to the proto>` and `REQUEST_INVENTORY_RELEASE=<release>`,
/// e.g. `v4.1.1` with `git show v4.1.1:packages/dapi-grpc/protos/platform/v0/platform.proto`,
/// or `v5.0` with this tree's proto while 5.0 is the release in development.
#[test]
#[ignore = "requires REQUEST_INVENTORY_PROTO and REQUEST_INVENTORY_RELEASE; rewrites a committed inventory"]
fn should_export_an_inventory() {
    let (Ok(path), Ok(release)) = (
        std::env::var("REQUEST_INVENTORY_PROTO"),
        std::env::var("REQUEST_INVENTORY_RELEASE"),
    ) else {
        eprintln!(
            "skipped: set REQUEST_INVENTORY_PROTO and REQUEST_INVENTORY_RELEASE to rewrite an \
             inventory"
        );
        return;
    };
    let schema = Schema::parse(&std::fs::read_to_string(path).unwrap());
    let items: BTreeSet<String> = schema.reachable(&|_| false).into_iter().collect();
    let mut text = format!(
        "# Methods, and fields, oneof arms and enum values reachable from their requests, in\n\
         # {release}'s platform.proto: what {release} nodes understand. Generated by\n\
         # `request_inventory.rs::should_export_an_inventory`; never edit by hand.\n"
    );
    for item in items {
        text.push_str(&item);
        text.push('\n');
    }
    std::fs::write(
        packages().join(format!(
            "rs-dapi-client/tests/request_inventory/{release}.txt"
        )),
        text,
    )
    .unwrap();
}
