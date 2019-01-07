const ConsensusError = require('./ConsensusError');

class DPObjectAlreadyPresentError extends ConsensusError {
  /**
   * @param {DPObject} dpObject
   * @param {DPObject} fetchedDPObject
   */
  constructor(dpObject, fetchedDPObject) {
    super('DP Object with the same ID is already present');

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

module.exports = DPObjectAlreadyPresentError;
