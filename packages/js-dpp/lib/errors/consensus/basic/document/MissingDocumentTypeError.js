const AbstractBasicError = require('../AbstractBasicError');

class MissingDocumentTypeError extends AbstractBasicError {
  constructor() {
    super('$type is not present');
  }
}

module.exports = MissingDocumentTypeError;
