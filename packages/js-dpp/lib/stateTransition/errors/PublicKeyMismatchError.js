class PublicKeyMismatchError extends Error {
  /**
   *
   * @param {IdentityPublicKey} publicKey
   */
  constructor(publicKey) {
    super();

    this.name = this.constructor.name;
    this.message = 'Public key mismatched';

    this.publicKey = publicKey;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get mismatched public key
   *
   * @return {IdentityPublicKey}
   */
  getPublicKey() {
    return this.publicKey;
  }
}

module.exports = PublicKeyMismatchError;
