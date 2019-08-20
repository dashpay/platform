const ConsensusError = require('./ConsensusError');

class DataTriggerInvalidResultError extends ConsensusError {
  /**
   * @param {Dot} document
   * @param {DataTriggerExecutionContext} context
   */
  constructor(dataTrigger, context) {
    super('Data trigger have not returned any result');

    this.dataTrigger = dataTrigger;
    this.context = context;
  }

  /**
   * Get data trigger
   *
   * @returns {DataTrigger}
   */
  getDataTrigger() {
    return this.dataTrigger;
  }

  /**
   * Get data trigger execution context
   *
   * @return {DataTriggerExecutionContext}
   */
  getContext() {
    return this.context;
  }
}

module.exports = DataTriggerInvalidResultError;
