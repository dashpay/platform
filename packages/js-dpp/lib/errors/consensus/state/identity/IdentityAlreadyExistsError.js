const AbstractStateError = require('../AbstractStateError');

class IdentityAlreadyExistsError extends AbstractStateError {
  /**
   * @param {IdentityCreateTransition} stateTransition
   */
  constructor(stateTransition) {
    super(`Identity with id ${stateTransition.getIdentityId()} already exists`);

    this.stateTransition = stateTransition;
  }

  /**
   * Get state transition
   *
   * @return {IdentityCreateTransition}
   */
  getStateTransition() {
    return this.stateTransition;
  }
}

module.exports = IdentityAlreadyExistsError;
