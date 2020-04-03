const AbciError = require('./AbciError');

class ExecutionTimedOutError extends AbciError {
  constructor() {
    super(
      AbciError.CODES.EXECUTION_TIMED_OUT,
      'Execution timed out',
    );
  }
}

module.exports = ExecutionTimedOutError;
