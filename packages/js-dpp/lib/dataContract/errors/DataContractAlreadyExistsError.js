const DPPError = require('../../errors/DPPError');

class DataContractAlreadyExistsError extends DPPError {
  /**
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super('Data contract already exists');

    this.stateTransition = stateTransition;
  }

  /**
   * Get failed state transition
   *
   * @return {AbstractStateTransition}
   */
  getStateTransition() {
    return this.stateTransition;
  }
}

module.exports = DataContractAlreadyExistsError;
