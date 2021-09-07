const AbstractBasicError = require('../AbstractBasicError');

class InvalidInstantAssetLockProofSignatureError extends AbstractBasicError {
  constructor() {
    super('Invalid instant lock proof signature');
  }
}

module.exports = InvalidInstantAssetLockProofSignatureError;
