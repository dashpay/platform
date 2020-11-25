const ConsensusError = require('./ConsensusError');

class IdentityAssetLockProofMismatchError extends ConsensusError {
  constructor() {
    super('Asset lock proof mismatch');
  }
}

module.exports = IdentityAssetLockProofMismatchError;
