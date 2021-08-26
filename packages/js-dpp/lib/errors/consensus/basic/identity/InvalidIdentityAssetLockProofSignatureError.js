const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityAssetLockProofSignatureError extends AbstractBasicError {
  constructor() {
    super('Invalid Asset lock proof signature');
  }
}

module.exports = InvalidIdentityAssetLockProofSignatureError;
