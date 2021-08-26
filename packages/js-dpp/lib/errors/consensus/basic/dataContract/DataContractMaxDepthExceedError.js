const AbstractBasicError = require('../AbstractBasicError');

class DataContractMaxDepthExceedError extends AbstractBasicError {
  constructor() {
    super(`JSON Schema depth is greater than ${DataContractMaxDepthExceedError.MAX_DEPTH}`);
  }
}

DataContractMaxDepthExceedError.MAX_DEPTH = 500;

module.exports = DataContractMaxDepthExceedError;
