/// Test fixtures
pub mod fixture;
/// Test helpers module
pub mod helpers;

/// The `expect_match` macro is used to match a value against a specified pattern.
/// If the pattern matches, it returns the result specified in the macro.
/// If the pattern does not match, it panics with a message indicating the expected pattern.
///
/// # Arguments
///
/// * `$value:expr` - The value to match against the pattern.
/// * `$pattern:pat` - The pattern to match the value against.
/// * `$result:expr` - The result to return if the pattern matches.
///
/// # Panics
///
/// This macro will panic with a message containing the pattern if the value does not match the pattern.
///
/// # Examples
///
/// ```
/// #[derive(Debug)]
/// struct FeeResult {
///     // fields
/// }
///
/// #[derive(Debug)]
/// enum StateTransitionExecutionResult {
///     SuccessfulExecution(u32, FeeResult),
///     FailedExecution(String),
/// }
///
/// struct ProcessingResult {
///     results: Vec<StateTransitionExecutionResult>,
/// }
///
/// impl ProcessingResult {
///     fn execution_results(&self) -> &[StateTransitionExecutionResult] {
///         &self.results
///     }
/// }
///
/// fn main() {
///     use drive_abci::expect_match;
///     let fee_result = FeeResult {};
///     let mut processing_result = ProcessingResult {
///         results: vec![StateTransitionExecutionResult::SuccessfulExecution(42, fee_result)],
///     };
///
///     let fee_result = expect_match!(
///         &processing_result.execution_results()[0],
///         StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => fee_result
///     );
///
///     // Use fee_result here
///     println!("{:?}", fee_result);
/// }
/// ```
#[macro_export]
macro_rules! expect_match {
    ($value:expr, $pattern:pat => $result:expr) => {
        match $value {
            $pattern => $result,
            e => panic!(
                "expected pattern to match: {:?}, got {:?}",
                stringify!($pattern),
                e
            ),
        }
    };
}
