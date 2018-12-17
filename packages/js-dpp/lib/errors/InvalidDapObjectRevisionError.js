const ConsensusError = require('./ConsensusError');

class InvalidDapObjectRevisionError extends ConsensusError {
  /**
   * @param {DapObject} dapObject
   * @param {DapObject} fetchedDapObject
   */
  constructor(dapObject, fetchedDapObject) {
    super('Invalid Dap Object revision');

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

module.exports = InvalidDapObjectRevisionError;
