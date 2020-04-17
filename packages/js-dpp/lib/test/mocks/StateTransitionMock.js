const AbstractStateTransitionIdentitySigned = require('../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

class StateTransitionMock extends AbstractStateTransitionIdentitySigned {
  getType() {
    return stateTransitionTypes.DATA_CONTRACT_CREATE;
  }

  toJSON(options = {}) {
    return super.toJSON(options);
  }
}

module.exports = StateTransitionMock;
