class DataContractAlreadyExistsError extends Error {
  /**
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super();

    this.name = this.constructor.name;
    this.message = 'Data contract already exists';

    this.stateTransition = stateTransition;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
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
