const AbstractBasicError = require('../AbstractBasicError');

class MissingDocumentTransitionTypeError extends AbstractBasicError {
  constructor() {
    super('$type is not present');
  }
}

module.exports = MissingDocumentTransitionTypeError;
