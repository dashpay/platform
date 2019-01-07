const ConsensusError = require('./ConsensusError');

class InvalidDPObjectScopeIdError extends ConsensusError {
  /**
   * @param {Object} rawDPObject
   */
  constructor(rawDPObject) {
    super('Invalid DP Object scopeId');

    this.rawDPObject = rawDPObject;
  }

  /**
   * Get raw DPObject
   *
   * @return {Object}
   */
  getRawDPObject() {
    return this.rawDPObject;
  }
}

module.exports = InvalidDPObjectScopeIdError;
