const AbstractBasicError = require('../AbstractBasicError');

class DuplicatedIdentityPublicKeyError extends AbstractBasicError {
  /**
   * @param {number[]} duplicatedPublicKeyIds
   */
  constructor(duplicatedPublicKeyIds) {
    super(`Duplicated public keys ${duplicatedPublicKeyIds.join(', ')} found`);

    this.duplicatedPublicKeyIds = duplicatedPublicKeyIds;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get duplicated public key ids
   *
   * @return {number[]}
   */
  getDuplicatedPublicKeysIds() {
    return this.duplicatedPublicKeyIds;
  }
}

module.exports = DuplicatedIdentityPublicKeyError;
