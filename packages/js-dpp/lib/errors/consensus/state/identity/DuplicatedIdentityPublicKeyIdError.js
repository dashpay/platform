const AbstractStateError = require('../AbstractStateError');

class DuplicatedIdentityPublicKeyIdError extends AbstractStateError {
  /**
   * @param {number[]} duplicatedIds
   */
  constructor(duplicatedIds) {
    super(`Duplicated public key ids ${duplicatedIds.join(', ')} found`);

    this.duplicatedIds = duplicatedIds;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get duplicated public key ids
   *
   * @return {number[]}
   */
  getDuplicatedIds() {
    return this.duplicatedIds;
  }
}

module.exports = DuplicatedIdentityPublicKeyIdError;
