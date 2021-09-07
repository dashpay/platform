const AbstractBasicError = require('../AbstractBasicError');

class IdentityAssetLockProofLockedTransactionMismatchError extends AbstractBasicError {
  /**
   * @param {Buffer} instantLockTransactionId
   * @param {Buffer} assetLockTransactionId
   */
  constructor(instantLockTransactionId, assetLockTransactionId) {
    super(`Instant Lock transaction ${instantLockTransactionId.toString('hex')} and Asset lock transaction ${assetLockTransactionId.toString('hex')} mismatch`);

    this.instantLockTransactionId = instantLockTransactionId;
    this.assetLockTransactionId = assetLockTransactionId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @return {Buffer}
   */
  getInstantLockTransactionId() {
    return this.instantLockTransactionId;
  }

  /**
   * @return {Buffer}
   */
  getAssetLockTransactionId() {
    return this.assetLockTransactionId;
  }
}

module.exports = IdentityAssetLockProofLockedTransactionMismatchError;
