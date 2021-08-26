const DPPError = require('../../errors/DPPError');

class StateTransitionIsNotSignedError extends DPPError {
  /**
   *
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super('State Transition is not signed');

    this.stateTransition = stateTransition;
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
