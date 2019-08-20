const ConsensusError = require('./ConsensusError');

class DataTriggerExecutionError extends ConsensusError {
  /**
   * @param {DataTrigger} dataTrigger
   * @param {DataTriggerExecutionContext} context
   * @param {Error} error
   */
  constructor(dataTrigger, context, error) {
    super(error.message);

    this.error = error;
    this.dataTrigger = dataTrigger;
    this.context = context;
  }

  /**
   * Return internal error instance
   *
   * @return {Error}
   */
  getError() {
    return this.error;
  }

  /**
   * Get data trigger
   *
   * @return {DataTrigger}
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

module.exports = DataTriggerExecutionError;
