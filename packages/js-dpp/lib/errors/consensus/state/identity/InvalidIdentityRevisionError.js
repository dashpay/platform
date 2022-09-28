const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class InvalidIdentityRevisionError extends AbstractStateError {
  /**
   * @param {Buffer} identityId
   * @param {number} currentRevision
   */
  constructor(identityId, currentRevision) {
    super(`Identity ${Identifier.from(identityId)} has invalid revision. The current revision is ${currentRevision}`);

    this.identityId = identityId;
    this.currentRevision = currentRevision;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get Identity ID
   *
   * @return {Buffer}
   */
  getIdentityId() {
    return this.identityId;
  }

  /**
   * Get current revision
   *
   * @return {number}
   */
  getCurrentRevision() {
    return this.currentRevision;
  }
}

module.exports = InvalidIdentityRevisionError;
