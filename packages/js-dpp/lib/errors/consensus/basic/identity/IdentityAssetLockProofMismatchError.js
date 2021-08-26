const AbstractBasicError = require('../AbstractBasicError');

class IdentityAssetLockProofMismatchError extends AbstractBasicError {
  constructor() {
    super('Asset lock proof mismatch');
  }
}

module.exports = IdentityAssetLockProofMismatchError;
