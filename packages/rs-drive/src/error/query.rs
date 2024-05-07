use sqlparser::parser::ParserError;

/// Query errors
#[derive(Debug, thiserror::Error)]
pub enum QuerySyntaxError {
    /// Deserialization
    #[error("deserialization error: {0}")]
    DeserializationError(String),
    /// Unsupported error
    #[error("unsupported error: {0}")]
    Unsupported(String),
    /// Invalid SQL error
    #[error("sql parsing error: {0}")]
    SQLParsingError(#[from] ParserError),
    /// Invalid SQL error
    #[error("invalid sql error: {0}")]
    InvalidSQL(String),

    /// We asked for nothing
    #[error("no query items error: {0}")]
    NoQueryItems(String),
    ///DataContract not found error
    #[error("contract not found error: {0}")]
    DataContractNotFound(String),
    /// Document type not found error
    #[error("document type not found error: {0}")]
    DocumentTypeNotFound(String),

    /// Duplicate non groupable clause on same field error
    #[error("duplicate non groupable clause on same field error: {0}")]
    DuplicateNonGroupableClauseSameField(String),
    /// Multiple in clauses error
    #[error("multiple in clauses error: {0}")]
    MultipleInClauses(String),
    /// Multiple range clauses error
    #[error("multiple range clauses error: {0}")]
    MultipleRangeClauses(String),
    /// Range clauses not groupable error
    #[error("range clauses not groupable error: {0}")]
    RangeClausesNotGroupable(String),

    /// Invalid between clause error
    #[error("invalid BETWEEN clause error: {0}")]
    InvalidBetweenClause(String),
    /// Invalid in clause error
    #[error("invalid IN clause error: {0}")]
    InvalidInClause(String),
    /// Invalid starts with clause error
    #[error("invalid STARTSWITH clause error: {0}")]
    InvalidStartsWithClause(String),

    /// Invalid where clause order error
    #[error("invalid where clause order error: {0}")]
    InvalidWhereClauseOrder(String),
    /// Invalid where clause components error
    #[error("invalid where clause components error: {0}")]
    InvalidWhereClauseComponents(String),

    /// Invalid orderBy properties error
    #[error("invalid order by properties error: {0}")]
    InvalidOrderByProperties(String),
    /// Invalid orderBy properties order error
    #[error("invalid order by properties order error: {0}")]
    InvalidOrderByPropertiesOrder(String),

    /// Invalid contract id error
    #[error("invalid contract id error: {0}")]
    InvalidContractId(String),
    /// Query for a key has an invalid parameter
    #[error("query invalid key parameter error: {0}")]
    InvalidKeyParameter(String),
    /// Query invalid limit error
    #[error("query invalid limit error: {0}")]
    InvalidLimit(String),
    /// Query invalid parameter error
    #[error("query invalid parameter error: {0}")]
    InvalidParameter(String),
    /// Query invalid format for where clause error
    #[error("query invalid format for where clause error: {0}")]
    InvalidFormatWhereClause(String),

    /// Conflicting conditions error
    #[error("conflicting conditions error: {0}")]
    ConflictingConditions(String),

    /// Duplicate start conditions error
    #[error("duplicate start conditions error: {0}")]
    DuplicateStartConditions(String),
    /// Start document not found error
    #[error("start document not found error: {0}")]
    StartDocumentNotFound(String),

    /// Invalid document type error
    #[error("invalid document type error: {0}")]
    InvalidDocumentType(String),

    /// Where clause on non indexed property error
    #[error("where clause on non indexed property error: {0}")]
    WhereClauseOnNonIndexedProperty(String),
    /// Query is too far from index error
    #[error("query is too far from index: {0}")]
    QueryTooFarFromIndex(String),
    /// Query on document type with no indexes error
    #[error("query on document type with no indexes: {0}")]
    QueryOnDocumentTypeWithNoIndexes(String),

    /// Missing order by for range error
    #[error("missing order by for range error: {0}")]
    MissingOrderByForRange(String),

    /// Range operator not in final index error
    #[error("range operator not in final index error: {0}")]
    RangeOperatorNotInFinalIndex(String),
    /// In operator not in final indexes error
    #[error("in operator not in final indexes error: {0}")]
    InOperatorNotInFinalIndexesIndex(String),
    /// Range operator does not have order by error
    #[error("range operator does not have order by error: {0}")]
    RangeOperatorDoesNotHaveOrderBy(String),

    /// Validation error
    #[error("validation error: {0}")]
    Validation(String),
    /// Where condition properties number error
    #[error("where condition properties number error: {0}")]
    WhereConditionPropertiesNumber(String),

    /// Starts with illegal string error
    #[error("starts with illegal string error: {0}")]
    StartsWithIllegalString(String),

    /// Invalid identity prove request error
    #[error("invalid identity prove request error: {0}")]
    InvalidIdentityProveRequest(String),
}
