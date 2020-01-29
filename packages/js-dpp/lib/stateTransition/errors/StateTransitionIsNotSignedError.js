class StateTransitionIsNotSignedError extends Error {
  /**
   *
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super();

    this.name = this.constructor.name;
    this.message = 'State Transition is not signed';

    this.stateTransition = stateTransition;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get unsigned state transition
   *
   * @return {AbstractStateTransition}
   */
  getStateTransition() {
    return this.stateTransition;
  }
}

module.exports = StateTransitionIsNotSignedError;
