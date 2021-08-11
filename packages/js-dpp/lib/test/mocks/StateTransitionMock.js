const AbstractStateTransitionIdentitySigned = require('../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

class StateTransitionMock extends AbstractStateTransitionIdentitySigned {
  getType() {
    return stateTransitionTypes.DATA_CONTRACT_CREATE;
  }

  getModifiedDataIds() {
    return [];
  }
}

module.exports = StateTransitionMock;
