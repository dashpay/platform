class WrongStateTransitionTypeError extends Error {
  /**
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super();

    this.name = this.constructor.name;
    this.message = 'Can\'t apply a state transition to a model, wrong state transition type';

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

module.exports = WrongStateTransitionTypeError;
