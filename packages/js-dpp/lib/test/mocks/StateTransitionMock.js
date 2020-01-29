const AbstractStateTransition = require('../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

class StateTransitionMock extends AbstractStateTransition {
  getType() {
    return stateTransitionTypes.DATA_CONTRACT;
  }

  toJSON(options = {}) {
    return super.toJSON(options);
  }
}

module.exports = StateTransitionMock;
