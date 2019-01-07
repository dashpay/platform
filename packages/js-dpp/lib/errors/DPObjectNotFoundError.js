const ConsensusError = require('./ConsensusError');

const DPObject = require('../object/DPObject');

class DPObjectNotFoundError extends ConsensusError {
  /**
   * @param {DPObject} dpObject
   */
  constructor(dpObject) {
    const noun = {
      [DPObject.ACTIONS.UPDATE]: 'Updated',
      [DPObject.ACTIONS.DELETE]: 'Deleted',
    };

    super(`${noun[dpObject.getAction()]} DPObject not found`);

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

module.exports = DPObjectNotFoundError;
