const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidStateTransitionSignatureError extends AbstractSignatureError {
  constructor() {
    super('Invalid State Transition signature');
  }
}

module.exports = InvalidStateTransitionSignatureError;
