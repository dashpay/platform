const AbstractBasicError = require('../AbstractBasicError');

class MissingDocumentTransitionActionError extends AbstractBasicError {
  constructor() {
    super('$action is not present');
  }
}

module.exports = MissingDocumentTransitionActionError;
