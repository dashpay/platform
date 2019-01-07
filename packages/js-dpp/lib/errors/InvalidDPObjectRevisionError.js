const ConsensusError = require('./ConsensusError');

class InvalidDPObjectRevisionError extends ConsensusError {
  /**
   * @param {DPObject} dpObject
   * @param {DPObject} fetchedDPObject
   */
  constructor(dpObject, fetchedDPObject) {
    super('Invalid DP Object revision');

    this.dpObject = dpObject;
    this.fetchedDPObject = fetchedDPObject;
  }

  /**
   * Get DP Object
   *
   * @return {DPObject}
   */
  getDPObject() {
    return this.dpObject;
  }

  /**
   * Get fetched DP Object
   *
   * @return {DPObject}
   */
  getFetchedDPObject() {
    return this.fetchedDPObject;
  }
}

module.exports = InvalidDPObjectRevisionError;
