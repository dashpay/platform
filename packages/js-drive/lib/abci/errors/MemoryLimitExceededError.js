const AbciError = require('./AbciError');

class MemoryLimitExceededError extends AbciError {
  constructor() {
    super(
      AbciError.CODES.MEMORY_LIMIT_EXCEEDED,
      'Memory limit exceeded',
    );
  }
}

module.exports = MemoryLimitExceededError;
