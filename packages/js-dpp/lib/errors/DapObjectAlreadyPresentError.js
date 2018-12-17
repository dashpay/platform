const ConsensusError = require('./ConsensusError');

class DapObjectAlreadyPresentError extends ConsensusError {
  /**
   * @param {DapObject} dapObject
   * @param {DapObject} fetchedDapObject
   */
  constructor(dapObject, fetchedDapObject) {
    super('Dap Object with the same ID is already present');

    this.dapObject = dapObject;
    this.fetchedDapObject = fetchedDapObject;
  }

  /**
   * Get Dap Object
   *
   * @return {DapObject}
   */
  getDapObject() {
    return this.dapObject;
  }

  /**
   * Get fetched DAP Object
   *
   * @return {DapObject}
   */
  getFetchedDapObject() {
    return this.fetchedDapObject;
  }
}

module.exports = DapObjectAlreadyPresentError;
