const AbstractBasicError = require('../AbstractBasicError');

class MissingStateTransitionTypeError extends AbstractBasicError {
  constructor() {
    super('State Transition type is not present');
  }
}

module.exports = MissingStateTransitionTypeError;
