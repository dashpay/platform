class AssetLockOutputNotFoundError extends Error {
  constructor() {
    super();

    this.message = 'Asset Lock transaction output not found';

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

module.exports = AssetLockOutputNotFoundError;
