const AbstractBasicError = require('../AbstractBasicError');

class MissingDataContractIdError extends AbstractBasicError {
  constructor() {
    super('$dataContractId is not present');
  }
}

module.exports = MissingDataContractIdError;
