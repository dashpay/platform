const ConsensusError = require('./ConsensusError');

class InvalidIdentityAssetLockProofSignatureError extends ConsensusError {
  constructor() {
    super('Invalid Asset lock proof signature');
  }
}

module.exports = InvalidIdentityAssetLockProofSignatureError;
