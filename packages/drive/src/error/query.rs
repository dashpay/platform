#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("unsupported error: {0}")]
    Unsupported(&'static str),
    #[error("invalid sql error: {0}")]
    InvalidSQL(&'static str),

    #[error("contract not found error: {0}")]
    ContractNotFound(&'static str),
    #[error("document type not found error: {0}")]
    DocumentTypeNotFound(&'static str),

    #[error("duplicate non groupable clause on same field error: {0}")]
    DuplicateNonGroupableClauseSameField(&'static str),
    #[error("multiple in clauses error: {0}")]
    MultipleInClauses(&'static str),
    #[error("multiple range clauses error: {0}")]
    MultipleRangeClauses(&'static str),
    #[error("range clauses not groupable error: {0}")]
    RangeClausesNotGroupable(&'static str),

    #[error("invalid BETWEEN clause error: {0}")]
    InvalidBetweenClause(&'static str),
    #[error("invalid IN clause error: {0}")]
    InvalidInClause(&'static str),
    #[error("invalid STARTSWITH clause error: {0}")]
    InvalidStartsWithClause(&'static str),

    // Where Condition Errors
    #[error("invalid where clause order error: {0}")]
    InvalidWhereClauseOrder(&'static str),
    #[error("invalid where clause components error: {0}")]
    InvalidWhereClauseComponents(&'static str),

    // Order Errors
    #[error("invalid order by properties error: {0}")]
    InvalidOrderByProperties(&'static str),
    #[error("invalid order by properties order error: {0}")]
    InvalidOrderByPropertiesOrder(&'static str),

    // Query Errors
    #[error("invalid contract id error: {0}")]
    InvalidContractId(&'static str),
    #[error("query invalid limit error: {0}")]
    InvalidLimit(&'static str),
    #[error("query invalid format for where clause error: {0}")]
    InvalidFormatWhereClause(&'static str),

    #[error("conflicting conditions error: {0}")]
    ConflictingConditions(&'static str),

    #[error("duplicate start conditions error: {0}")]
    DuplicateStartConditions(&'static str),
    #[error("start document not found error: {0}")]
    StartDocumentNotFound(&'static str),

    #[error("invalid document type error: {0}")]
    InvalidDocumentType(&'static str),

    #[error("where clause on non indexed property error: {0}")]
    WhereClauseOnNonIndexedProperty(&'static str),
    #[error("query is too far from index: {0}")]
    QueryTooFarFromIndex(&'static str),
    #[error("query on document type with no indexes: {0}")]
    QueryOnDocumentTypeWithNoIndexes(&'static str),

    #[error("missing order by for range error: {0}")]
    MissingOrderByForRange(&'static str),

    #[error("range operator not in final index error: {0}")]
    RangeOperatorNotInFinalIndex(&'static str),
    #[error("in operator not in final indexes error: {0}")]
    InOperatorNotInFinalIndexesIndex(&'static str),
    #[error("range operator does not have order by error: {0}")]
    RangeOperatorDoesNotHaveOrderBy(&'static str),

    #[error("validation error: {0}")]
    Validation(&'static str),
    #[error("where condition properties number error: {0}")]
    WhereConditionPropertiesNumber(&'static str),

    #[error("starts with illegal string error: {0}")]
    StartsWithIllegalString(&'static str),
}
