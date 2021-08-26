const DPPError = require('../../errors/DPPError');

class WrongStateTransitionTypeError extends DPPError {
  /**
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super('Can\'t apply a state transition to a model, wrong state transition type');

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

module.exports = WrongStateTransitionTypeError;
