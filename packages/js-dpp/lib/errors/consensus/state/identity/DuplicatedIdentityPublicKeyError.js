const AbstractStateError = require('../AbstractStateError');

class DuplicatedIdentityPublicKeyError extends AbstractStateError {
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
