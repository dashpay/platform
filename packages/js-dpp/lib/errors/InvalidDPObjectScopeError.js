const ConsensusError = require('./ConsensusError');

class InvalidDPObjectScopeError extends ConsensusError {
  /**
   * @param {DPObject} dpObject
   */
  constructor(dpObject) {
    super('Invalid DP Object scope');

    this.dpObject = dpObject;
  }

  /**
   * Get DPObject
   *
   * @return {DPObject}
   */
  getDPObject() {
    return this.dpObject;
  }
}

module.exports = InvalidDPObjectScopeError;
