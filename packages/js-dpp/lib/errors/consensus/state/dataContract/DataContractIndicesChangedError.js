const AbstractStateError = require('../AbstractStateError');

class DataContractIndicesChangedError extends AbstractStateError {
  constructor() {
    super('Change of indices during Data Contract update is not allowed');

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = DataContractIndicesChangedError;
