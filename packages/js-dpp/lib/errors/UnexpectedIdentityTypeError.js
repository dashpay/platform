const ConsensusError = require('./ConsensusError');

class UnexpectedIdentityTypeError extends ConsensusError {
  /**
   * @param {RawIdentity|Identity} identity
   * @param {number[]} expectedIdentityTypes
   */
  constructor(identity, expectedIdentityTypes) {
    super('Unexpected identity type');

    this.identity = identity;
    this.expectedIdentityTypes = expectedIdentityTypes;
  }

  /**
   * Get identity
   *
   * @return {RawIdentity|Identity}
   */
  getIdentity() {
    return this.identity;
  }

  /**
   * Get expected identity types
   *
   * @return {number[]}
   */
  getExpectedIdentityTypes() {
    return this.expectedIdentityTypes;
  }
}

module.exports = UnexpectedIdentityTypeError;
