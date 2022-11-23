/// Query errors
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// Unsupported error
    #[error("unsupported error: {0}")]
    Unsupported(&'static str),
    /// Invalid SQL error
    #[error("invalid sql error: {0}")]
    InvalidSQL(&'static str),

    /// Contract not found error
    #[error("contract not found error: {0}")]
    ContractNotFound(&'static str),
    /// Document type not found error
    #[error("document type not found error: {0}")]
    DocumentTypeNotFound(&'static str),

    /// Duplicate non groupable clause on same field error
    #[error("duplicate non groupable clause on same field error: {0}")]
    DuplicateNonGroupableClauseSameField(&'static str),
    /// Multiple in clauses error
    #[error("multiple in clauses error: {0}")]
    MultipleInClauses(&'static str),
    /// Multiple range clauses error
    #[error("multiple range clauses error: {0}")]
    MultipleRangeClauses(&'static str),
    /// Range clauses not groupable error
    #[error("range clauses not groupable error: {0}")]
    RangeClausesNotGroupable(&'static str),

    /// Invalid between clause error
    #[error("invalid BETWEEN clause error: {0}")]
    InvalidBetweenClause(&'static str),
    /// Invalid in clause error
    #[error("invalid IN clause error: {0}")]
    InvalidInClause(&'static str),
    /// Invalid starts with clause error
    #[error("invalid STARTSWITH clause error: {0}")]
    InvalidStartsWithClause(&'static str),

    /// Invalid where clause order error
    #[error("invalid where clause order error: {0}")]
    InvalidWhereClauseOrder(&'static str),
    /// Invalid where clause components error
    #[error("invalid where clause components error: {0}")]
    InvalidWhereClauseComponents(&'static str),

    /// Invalid orderBy properties error
    #[error("invalid order by properties error: {0}")]
    InvalidOrderByProperties(&'static str),
    /// Invalid orderBy properties order error
    #[error("invalid order by properties order error: {0}")]
    InvalidOrderByPropertiesOrder(&'static str),

    /// Invalid contract id error
    #[error("invalid contract id error: {0}")]
    InvalidContractId(&'static str),
    /// Query invalid limit error
    #[error("query invalid limit error: {0}")]
    InvalidLimit(&'static str),
    /// Query invalid format for where clause error
    #[error("query invalid format for where clause error: {0}")]
    InvalidFormatWhereClause(&'static str),

    /// Conflicting conditions error
    #[error("conflicting conditions error: {0}")]
    ConflictingConditions(&'static str),

    /// Duplicate start conditions error
    #[error("duplicate start conditions error: {0}")]
    DuplicateStartConditions(&'static str),
    /// Start document not found error
    #[error("start document not found error: {0}")]
    StartDocumentNotFound(&'static str),

    /// Invalid document type error
    #[error("invalid document type error: {0}")]
    InvalidDocumentType(&'static str),

    /// Where clause on non indexed property error
    #[error("where clause on non indexed property error: {0}")]
    WhereClauseOnNonIndexedProperty(&'static str),
    /// Query is too far from index error
    #[error("query is too far from index: {0}")]
    QueryTooFarFromIndex(&'static str),
    /// Query on document type with no indexes error
    #[error("query on document type with no indexes: {0}")]
    QueryOnDocumentTypeWithNoIndexes(&'static str),

    /// Missing order by for range error
    #[error("missing order by for range error: {0}")]
    MissingOrderByForRange(&'static str),

    /// Range operator not in final index error
    #[error("range operator not in final index error: {0}")]
    RangeOperatorNotInFinalIndex(&'static str),
    /// In operator not in final indexes error
    #[error("in operator not in final indexes error: {0}")]
    InOperatorNotInFinalIndexesIndex(&'static str),
    /// Range operator does not have order by error
    #[error("range operator does not have order by error: {0}")]
    RangeOperatorDoesNotHaveOrderBy(&'static str),

    /// Validation error
    #[error("validation error: {0}")]
    Validation(&'static str),
    /// Where condition properties number error
    #[error("where condition properties number error: {0}")]
    WhereConditionPropertiesNumber(&'static str),

    /// Starts with illegal string error
    #[error("starts with illegal string error: {0}")]
    StartsWithIllegalString(&'static str),
}
