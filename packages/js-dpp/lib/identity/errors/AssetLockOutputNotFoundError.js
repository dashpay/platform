const DPPError = require('../../errors/DPPError');

class AssetLockOutputNotFoundError extends DPPError {
  constructor() {
    super('Asset Lock transaction output not found');
  }
}

module.exports = AssetLockOutputNotFoundError;
